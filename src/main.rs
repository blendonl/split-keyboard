mod config;
mod input_handler;
mod keyboard_ui;
mod layer_manager;
mod layer_window;
mod layouts;
mod virtual_keyboard;

use config::Config;
use keyboard_ui::{KeyboardUI, TouchAction};
use layer_window::LayerWindow;
use layouts::KeySide;
use virtual_keyboard::KeyState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <left|right>", args[0]);
        eprintln!("\nTo run both sides:");
        eprintln!("  {} left &", args[0]);
        eprintln!("  {} right &", args[0]);
        std::process::exit(1);
    }

    let side = match args[1].as_str() {
        "left" => KeySide::Left,
        "right" => KeySide::Right,
        _ => {
            eprintln!("Invalid side: {}. Use 'left' or 'right'", args[1]);
            std::process::exit(1);
        }
    };

    let config = Config::load()?;

    run_keyboard(side, config)
}

fn run_keyboard(side: KeySide, config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let (mut window, _conn, mut event_queue) = LayerWindow::new(side, &config);
    let qh = event_queue.handle();

    // Initial dispatch to get configuration
    event_queue.blocking_dispatch(&mut window)?;

    // Wait for configuration
    while !window.configured {
        event_queue.blocking_dispatch(&mut window)?;
    }

    println!(
        "{:?} keyboard initialized: {}x{}",
        side, window.width, window.height
    );

    // Initialize virtual keyboard
    match window.init_virtual_keyboard() {
        Ok(_) => println!("{:?} keyboard: Virtual keyboard initialized", side),
        Err(e) => {
            eprintln!("{:?} keyboard: Failed to initialize virtual keyboard: {}", side, e);
            eprintln!("Keys will not be sent to applications.");
            eprintln!("Note: You may need to be in the 'input' group or run with appropriate permissions.");
        }
    }

    // Create buffers for rendering
    window.create_buffers(&qh)?;

    // Create keyboard UI
    let mut keyboard_ui = KeyboardUI::new(side, &config);
    keyboard_ui.resize(window.width, window.height);

    // Track current UI size
    let mut ui_width = window.width;
    let mut ui_height = window.height;

    // Track last touch state
    let mut last_touch_active = false;

    // Initial render
    if let Some(buffer) = window.get_buffer() {
        let pixels = unsafe {
            std::slice::from_raw_parts_mut(
                buffer.mmap.as_mut_ptr() as *mut u32,
                (buffer.width * buffer.height) as usize,
            )
        };
        keyboard_ui.render(pixels);
        window.draw();
    }

    // Main event loop
    loop {
        // Handle Wayland events (non-blocking when possible)
        event_queue.blocking_dispatch(&mut window)?;

        // Handle touch/pointer input
        if let Some((x, y, active)) = window.get_touch_state() {
            // Check for state changes (press or release)
            if active != last_touch_active {
                if let Some(action) = keyboard_ui.handle_touch(x, y, active) {
                    match action {
                        TouchAction::KeyPress(keycode) => {
                            println!("{:?} keyboard: Key {} {}", side, keycode, if active { "pressed" } else { "released" });

                            // Send keycode to virtual keyboard
                            if let Some(ref mut vk) = window.virtual_keyboard {
                                let key_state = if active { KeyState::Pressed } else { KeyState::Released };
                                vk.send_key(keycode, key_state);
                            }
                        }
                        TouchAction::MoveUp => {
                            if active {
                                let move_delta = -((window.screen_height as f32 * config.appearance.vertical_move_percent as f32 / 100.0) as i32);
                                if window.adjust_vertical_offset(move_delta) {
                                    println!("{:?} keyboard: Moved up", side);
                                }
                                // Update arrow visibility
                                keyboard_ui.show_up_arrow = window.can_move_up();
                                keyboard_ui.show_down_arrow = window.can_move_down();
                                keyboard_ui.needs_redraw = true;
                            }
                        }
                        TouchAction::MoveDown => {
                            if active {
                                let move_delta = (window.screen_height as f32 * config.appearance.vertical_move_percent as f32 / 100.0) as i32;
                                if window.adjust_vertical_offset(move_delta) {
                                    println!("{:?} keyboard: Moved down", side);
                                }
                                // Update arrow visibility
                                keyboard_ui.show_up_arrow = window.can_move_up();
                                keyboard_ui.show_down_arrow = window.can_move_down();
                                keyboard_ui.needs_redraw = true;
                            }
                        }
                    }
                }
                last_touch_active = active;

                // Reset touch state after handling release
                if !active {
                    window.reset_touch();
                }
            }
        }

        // Update animations
        keyboard_ui.update_animations();

        // Check if window was resized
        if window.configured {
            let size_changed = window.width != ui_width || window.height != ui_height;

            if size_changed {
                keyboard_ui.resize(window.width, window.height);
                ui_width = window.width;
                ui_height = window.height;

                // Recreate buffers for new size
                window.create_buffers(&qh)?;
            }

            window.configured = false;

            // Render after configuration
            if let Some(buffer) = window.get_buffer() {
                let pixels = unsafe {
                    std::slice::from_raw_parts_mut(
                        buffer.mmap.as_mut_ptr() as *mut u32,
                        (buffer.width * buffer.height) as usize,
                    )
                };
                keyboard_ui.render(pixels);
                window.draw();
            }
        } else if keyboard_ui.needs_redraw() {
            // Render keyboard when needed
            if let Some(buffer) = window.get_buffer() {
                let pixels = unsafe {
                    std::slice::from_raw_parts_mut(
                        buffer.mmap.as_mut_ptr() as *mut u32,
                        (buffer.width * buffer.height) as usize,
                    )
                };
                keyboard_ui.render(pixels);
                window.draw();
            }
        }

        // Sleep briefly to limit frame rate when animating (~60 FPS)
        // Only sleep if we have active animations to keep smooth rendering
        if keyboard_ui.has_active_animations() {
            std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 FPS
        }
    }
}
