use evdev::{uinput::VirtualDeviceBuilder, AttributeSet, EventType, InputEvent, Key};
use std::io;

pub struct VirtualKeyboard {
    device: evdev::uinput::VirtualDevice,
}

impl VirtualKeyboard {
    pub fn new() -> Result<Self, io::Error> {
        // Create a set of supported keys
        let mut keys = AttributeSet::<Key>::new();

        // Add all common keys (most common Linux keycodes)
        for i in 0..256u16 {
            keys.insert(Key::new(i));
        }

        // Create the virtual keyboard device
        let device = VirtualDeviceBuilder::new()?
            .name("Split Virtual Keyboard")
            .with_keys(&keys)?
            .build()?;

        Ok(Self { device })
    }

    pub fn send_key(&mut self, keycode: u32, state: KeyState) {
        // Convert keycode to evdev Key
        let key = Key::new(keycode as u16);
        let value = match state {
            KeyState::Pressed => 1,
            KeyState::Released => 0,
        };

        // Create and emit the key event
        let events = [
            InputEvent::new(EventType::KEY, key.code(), value),
            InputEvent::new(EventType::SYNCHRONIZATION, 0, 0),
        ];

        if let Err(e) = self.device.emit(&events) {
            eprintln!("Failed to send key event: {}", e);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum KeyState {
    Pressed,
    Released,
}
