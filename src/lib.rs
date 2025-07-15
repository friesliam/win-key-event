/*

References:

https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/Input/KeyboardAndMouse/fn.GetKeyState
https://learn.microsoft.com/en-us/windows/win32/api/_inputdev/
https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes

*/


use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

use tokio::time::{sleep, Duration};
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};



const KEY_DOWN_MASK: i16 = -32768; // using 0x8000 gives an overflow error, so directly state the negative int

enum KeyEvent {
    Press(i32),
    Release(i32)
}

enum KeyState {
    KeyPress,
    KeyRelease,
    StaticDown,
    StaticUp
}



pub struct KeyListener {
    vk_codes: Vec<i32>,
    unbounded_sender: UnboundedSender<KeyEvent>,
    previous_key_states: Vec<bool>,
    polling_wait: u64,
    is_watching: Arc<AtomicBool>,
}

impl KeyListener {
    fn new_default(unbounded_sender: UnboundedSender<KeyEvent>) -> Self {
        KeyListener {
            vk_codes: vec![
                // 0 - 9
                0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39,

                // 0 - 9 (numpad)
                0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,

                // a - z
                0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F,
                0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A,

                // punctuation, symbols (not shift + num symbols)
                // (shift normal)
                0xBA, // : ;
                0xBB, // + =
                0xBC, // < ,
                0xBD, // _ -
                0xBE, // > .
                0xBF, // ? /
                0xC0, // ~ `
                0xDB, // { [
                0xDC, // | \
                0xDD, // } ]
                0xDE, // " '

                0x20, // space
                0x0D, // enter
                0x08, // backspace
                0x09, // tab

                // numpad operators
                0x6A, // *
                0x6B, // +
                0x6D, // -
                0x6E, // .
                0x6F, // /

                // shift
                0xA0, // left
                0xA1, // right
                0x10  // generic
            ],
            unbounded_sender,
            previous_key_states: vec![false; 69],
            polling_wait: 10,
            is_watching: Arc::new(AtomicBool::new(false))
        }
    }

    fn new_custom(unbounded_sender: UnboundedSender<KeyEvent>, vk_codes: Vec<i32>, polling_wait: u64) -> Self {
        let key_num = &vk_codes.len();
        KeyListener {
            vk_codes,
            unbounded_sender,
            previous_key_states: vec![false; *key_num],
            polling_wait,
            is_watching: Arc::new(AtomicBool::new(false))
        }
    }

    pub fn quit(&mut self) {
        self.is_watching.store(false, Ordering::Relaxed);
    }
}



async fn listen(
    sleep_time: Duration,
    vk_codes: Vec<i32>,
    mut previous_key_states: Vec<bool>,
    sender: UnboundedSender<KeyEvent>,
    is_watching: Arc<AtomicBool>
) {
    is_watching.store(true, Ordering::Relaxed);
    while is_watching.load(Ordering::Relaxed) {
        for i in 0..vk_codes.len() {
            let vk_code = vk_codes[i];
            let key_state = get_key_state(&vk_code, i, &mut previous_key_states);
            match key_state {
                KeyState::StaticUp => {}
                KeyState::StaticDown => {}
                KeyState::KeyRelease => {
                    let _ = sender.send(KeyEvent::Release(vk_code));
                }
                KeyState::KeyPress => {
                    let _ = sender.send(KeyEvent::Press(vk_code));
                }

            }
        }
        sleep(sleep_time).await;
    }
}



fn get_key_state(vk_code: &i32, i: usize, previous_key_states: &mut Vec<bool>) -> KeyState {
    let state = unsafe {
        GetAsyncKeyState(*vk_code)
    };
    let is_down = (state & KEY_DOWN_MASK) != 0;
    let was_down = previous_key_states[i];

    previous_key_states[i] = is_down;

    if is_down {
        if was_down {
            return KeyState::StaticDown;
        } else {
            return KeyState::KeyPress;
        }
    } else {
        if was_down {
            return KeyState::KeyRelease;
        } else {
            return KeyState::StaticUp;
        }
    }
}






fn spawn_receiver(
    mut receiver: UnboundedReceiver<KeyEvent>,
    key_down_callback: Box<dyn Fn(i32) + Send + Sync + 'static>, key_up_callback: Box<dyn Fn(i32) + Send + Sync + 'static>
) {
    tokio::spawn(async move {
        while let Some(key_event) = receiver.recv().await {
            match key_event {
                KeyEvent::Press(vk) => key_down_callback(vk),
                KeyEvent::Release(vk) => key_up_callback(vk)
            }
        }
    });
}

fn spawn_listener(listener: Arc<Mutex<KeyListener>>) {
    tokio::spawn(async move {

        let locked = listener.lock().await;

        let sleep_time =  Duration::from_millis(locked.polling_wait);
        let vk_codes = locked.vk_codes.clone();
        let previous_key_states = locked.previous_key_states.clone();
        let sender = locked.unbounded_sender.clone();
        let is_watching = locked.is_watching.clone();

        drop(locked); // drops locked so that the user instance of the listener can be locked and 'quit' can be called

        listen(sleep_time, vk_codes, previous_key_states, sender, is_watching).await;
    });
}




pub fn init_default_key_listener(
    key_down_callback: Box<dyn Fn(i32) + Send + Sync + 'static>, key_up_callback: Box<dyn Fn(i32) + Send + Sync + 'static>
) -> Arc<tokio::sync::Mutex<KeyListener>> {
    let (sender, receiver) = unbounded_channel();
    let key_listener = Arc::new(Mutex::new(KeyListener::new_default(sender)));

    let listener = Arc::clone(&key_listener);
    spawn_listener(listener);

    spawn_receiver(receiver, key_down_callback, key_up_callback);

    key_listener
}

pub fn init_custom_key_listener(
    key_down_callback: Box<dyn Fn(i32) + Send + Sync + 'static>, key_up_callback: Box<dyn Fn(i32) + Send + Sync + 'static>,
    vk_codes: Vec<i32>,
    polling_wait: u64
) -> Arc<tokio::sync::Mutex<KeyListener>> {
    let (sender, receiver) = unbounded_channel();
    let key_listener = Arc::new(Mutex::new(KeyListener::new_custom(sender, vk_codes, polling_wait)));

    let listener = Arc::clone(&key_listener);
    spawn_listener(listener);

    spawn_receiver(receiver, key_down_callback, key_up_callback);

    key_listener
}