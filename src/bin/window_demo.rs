
extern crate gondola;

use gondola::{Window, Timer, InputManager, Key};

fn main() {
    let mut timer = Timer::new();
    let mut input = InputManager::new();
    let mut window = gondola::new_window("This is hopefully still a window");

    loop {
        let (time, delta) = timer.tick();

        window.poll_events(&mut input);
        window.swap_buffers();

        if input.key(Key::A).pressed_repeat() {
            println!("Ahh");
        }

        if input.mouse_key(2).pressed() {
            println!("{}", delta.as_secs_float()*1000.0);
        }

        if window.close_requested() {
            return;
        }
    }
}
