use smithay_client_toolkit::{
    delegate_pointer, delegate_touch,
    seat::pointer::{PointerEvent, PointerEventKind, PointerHandler},
    seat::touch::TouchHandler,
};
use wayland_client::{
    protocol::{wl_pointer::WlPointer, wl_touch::WlTouch},
    Connection, QueueHandle,
};

pub struct InputHandler {
    pub touch_x: f32,
    pub touch_y: f32,
    pub touch_active: bool,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            touch_x: 0.0,
            touch_y: 0.0,
            touch_active: false,
        }
    }

    pub fn get_touch_state(&self) -> Option<(f32, f32, bool)> {
        if self.touch_active || self.touch_x != 0.0 || self.touch_y != 0.0 {
            Some((self.touch_x, self.touch_y, self.touch_active))
        } else {
            None
        }
    }

    pub fn reset_touch(&mut self) {
        self.touch_active = false;
    }
}

impl PointerHandler for InputHandler {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            match event.kind {
                PointerEventKind::Enter { .. } => {}
                PointerEventKind::Leave { .. } => {
                    self.touch_active = false;
                }
                PointerEventKind::Motion { .. } => {
                    self.touch_x = event.position.0 as f32;
                    self.touch_y = event.position.1 as f32;
                }
                PointerEventKind::Press { button, .. } => {
                    if button == 0x110 {
                        // BTN_LEFT
                        self.touch_active = true;
                        self.touch_x = event.position.0 as f32;
                        self.touch_y = event.position.1 as f32;
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if button == 0x110 {
                        self.touch_active = false;
                    }
                }
                PointerEventKind::Axis { .. } => {}
            }
        }
    }
}

impl TouchHandler for InputHandler {
    fn down(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _serial: u32,
        _time: u32,
        _surface: wayland_client::protocol::wl_surface::WlSurface,
        _id: i32,
        position: (f64, f64),
    ) {
        self.touch_x = position.0 as f32;
        self.touch_y = position.1 as f32;
        self.touch_active = true;
    }

    fn up(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _serial: u32,
        _time: u32,
        _id: i32,
    ) {
        self.touch_active = false;
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _time: u32,
        _id: i32,
        position: (f64, f64),
    ) {
        self.touch_x = position.0 as f32;
        self.touch_y = position.1 as f32;
    }

    fn shape(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _id: i32,
        _major: f64,
        _minor: f64,
    ) {
    }

    fn orientation(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _id: i32,
        _orientation: f64,
    ) {
    }

    fn cancel(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _touch: &WlTouch) {
        self.touch_active = false;
    }
}

delegate_pointer!(InputHandler);
delegate_touch!(InputHandler);
