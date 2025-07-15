use win_key_event::*;

/*
This example creates a custom key listener that listens for key events on the spacebar or escape key
The program then sleeps for 4 seconds while simultaneously taking key input
For each key press or release, it fires the user provided callback and provides the corresponding key
After the 4 seconds the key listener quits and no longer takes input
*/

#[tokio::main]
async fn main() {
    // create a key listener and begin listening (this is non-blocking)
    let key_listener = init_custom_key_listener(
        Box::new(on_key_down), // on key press callback
        Box::new(on_key_up), // on key release callback
        vec![0x20, 0x1B], // keys to watch, (spacebar, esc)
        12 // time between each key state poll
    );

    println!("Listening for 'spacebar' or 'esc' key events...");

    // sleep for some time (4s)
    tokio::time::sleep(tokio::time::Duration::from_millis(4000)).await;

    // quit the key listener
    let clone = key_listener.clone();
    clone.lock().await.quit();
    println!("Key events are no longer fired after quitting the listener, test it!");
    println!("Press ctrl+c to exit");

    tokio::signal::ctrl_c().await.unwrap();
}

// on key down callback fn
fn on_key_down(vk: i32) {
    println!("Key Pressed: {}", vk);
}

// on key up callback fn
fn on_key_up(vk: i32) {
    println!("Key Released: {}", vk);
}