# win-key-event

Win-key-event is a Rust library for monitoring **global** key events on Windows systems.
This crate works through spawning the key listening task on one thread, and calling user defined key event callbacks (safely) on the thread in which the key listener was instantiated. This means that this crate works in a way that is **non-blocking** to other code.

 > [!WARNING]
 > This library polls key events using GetAsyncKeyState from the [Windows](https://docs.rs/crate/windows/latest) crate for Rust.
 > It does not set a windows hook.

 > [!NOTE]
 > I have not yet put this program on crates.io, I may if I extend it and add more features. For now however, clone the repository for use.

## Usage
Refer to this list of [virtual key codes](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes) for usage.

examples/example.rs provides a usage example.
Run it from the root directory with `cargo run --example example`.

### How to create a key listener
 > Immediately after creation, the key listener will automatically start listening.

Create a default listener with `init_default_key_listener`.
```Rust
let key_listener = init_default_key_listener(
    Box::new(key_down_callback), // callback for key presses
    Box::new(key_up_callback) // callback for key releases
);
```
Or create a custom listener with `init_custom_key_listener`.
```Rust
let key_listener = init_custom_key_listener(
    Box::new(key_down_callback), // callback for key presses
    Box::new(key_up_callback) // callback for key releases
    vec![
        0x30, 0x31, 0x32, 0x33, 0x34, // key codes for 0 - 9 (not numpad)
        0x35, 0x36, 0x37, 0x38, 0x39
    ],
    12 // time in milliseconds between each round of key polling (default: 10ms)
);
```

### How to delete a key listener
To then delete or stop the key listener: (asynchronous code)
```Rust
let clone = key_listener.clone();
clone.lock().await.quit();
```

### Key event callbacks
As shown earlier, the key event callbacks are passed into the either `init_default_key_listener` or `init_custom_key_listener` after being enclosed with Box::new().
The callback methods can be of the following form:
```Rust
fn key_down_callback(virtual_key_code: i32) {
    println!("Press: {}", virtual_key_code);
}

fn key_up_callback(virtual_key_code: i32) {
    println!("Release: {}", virtual_key_code);
}
```
