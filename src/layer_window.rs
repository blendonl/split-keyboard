use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState, pointer::PointerHandler, touch::TouchHandler},
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_buffer, wl_output, wl_seat, wl_shm, wl_shm_pool::WlShmPool, wl_surface, wl_pointer::WlPointer, wl_touch::WlTouch},
    Connection, QueueHandle,
};

use std::os::unix::io::BorrowedFd;
use memmap2::MmapMut;

use crate::config::Config;
use crate::layouts::KeySide;
use crate::virtual_keyboard::VirtualKeyboard;

pub struct Buffer {
    pub wl_buffer: wl_buffer::WlBuffer,
    pub mmap: MmapMut,
    pub width: u32,
    pub height: u32,
    pub released: bool,
}

pub struct LayerWindow {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,

    pub layer_surface: Option<LayerSurface>,
    pub configured: bool,
    pub width: u32,
    pub height: u32,
    pub side: KeySide,
    pub buffers: Vec<Buffer>,
    pub current_buffer: usize,

    // Input handling
    pub pointer: Option<WlPointer>,
    pub touch: Option<WlTouch>,
    pub touch_x: f32,
    pub touch_y: f32,
    pub touch_active: bool,

    // Configuration
    config: Config,

    // Vertical positioning
    pub vertical_offset: i32,
    pub screen_height: u32,

    // Virtual keyboard
    pub virtual_keyboard: Option<VirtualKeyboard>,
}

impl LayerWindow {
    pub fn new(side: KeySide, config: &Config) -> (Self, Connection, wayland_client::EventQueue<Self>) {
        let conn = Connection::connect_to_env().expect("Failed to connect to Wayland");
        let (globals, event_queue) = registry_queue_init(&conn).expect("Failed to initialize registry");
        let qh = event_queue.handle();

        let registry_state = RegistryState::new(&globals);
        let seat_state = SeatState::new(&globals, &qh);
        let output_state = OutputState::new(&globals, &qh);
        let compositor_state = CompositorState::bind(&globals, &qh)
            .expect("wl_compositor not available");
        let layer_shell = LayerShell::bind(&globals, &qh)
            .expect("zwlr_layer_shell_v1 not available");
        let shm = Shm::bind(&globals, &qh)
            .expect("wl_shm not available");

        let mut window = Self {
            registry_state,
            seat_state,
            output_state,
            compositor_state,
            layer_shell,
            shm,
            layer_surface: None,
            configured: false,
            width: 0,
            height: 0,
            side,
            buffers: Vec::new(),
            current_buffer: 0,
            pointer: None,
            touch: None,
            touch_x: 0.0,
            touch_y: 0.0,
            touch_active: false,
            config: config.clone(),
            vertical_offset: 0,
            screen_height: 0,
            virtual_keyboard: None,
        };

        window.create_layer_surface(&qh, config);

        (window, conn, event_queue)
    }

    pub fn init_virtual_keyboard(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Create the virtual keyboard using uinput
        let vk = VirtualKeyboard::new()?;
        self.virtual_keyboard = Some(vk);

        Ok(())
    }

    fn create_layer_surface(&mut self, qh: &QueueHandle<Self>, config: &Config) {
        let surface = self.compositor_state.create_surface(qh);

        let layer = match config.window.layer.as_str() {
            "background" => Layer::Background,
            "bottom" => Layer::Bottom,
            "top" => Layer::Top,
            _ => Layer::Overlay,
        };

        let anchor = match self.side {
            KeySide::Left => Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM,
            KeySide::Right => Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM,
        };

        let layer_surface = self.layer_shell.create_layer_surface(
            qh,
            surface,
            layer,
            Some("split-keyboard"),
            None,
        );

        layer_surface.set_anchor(anchor);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer_surface.set_margin(0, 0, 0, 0);

        // Calculate width based on screen width and configured percentage
        // Get screen width and height from outputs (use first output if available, otherwise default)
        let (screen_width, screen_height) = self.output_state
            .outputs()
            .next()
            .and_then(|output| self.output_state.info(&output))
            .and_then(|info| info.modes.iter().find(|m| m.current).map(|m| (m.dimensions.0 as u32, m.dimensions.1 as u32)))
            .unwrap_or((1920, 1080)); // Default to 1920x1080 if no output info available yet

        self.screen_height = screen_height;

        let desired_width = (screen_width * config.window.width_percent) / 100;
        let width = desired_width.max(100); // Minimum width of 100px

        println!("Screen width: {}, height: {}, width_percent: {}, desired_width: {}, final width: {}",
                 screen_width, screen_height, config.window.width_percent, desired_width, width);

        layer_surface.set_size(width, 0); // Height 0 means use full height between anchors
        layer_surface.set_exclusive_zone(-1); // -1 means don't reserve space
        layer_surface.set_margin(0, 0, 0, 0); // Will be updated by set_vertical_offset

        layer_surface.commit();

        self.layer_surface = Some(layer_surface);
    }

    pub fn surface(&self) -> Option<&wl_surface::WlSurface> {
        self.layer_surface.as_ref().map(|ls| ls.wl_surface())
    }

    pub fn create_buffers(&mut self, qh: &QueueHandle<Self>) -> Result<(), Box<dyn std::error::Error>> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        // Release old buffers
        self.buffers.clear();

        // Create double buffering
        for _ in 0..2 {
            let buffer = self.create_buffer(qh)?;
            self.buffers.push(buffer);
        }

        self.current_buffer = 0;
        Ok(())
    }

    fn create_buffer(&mut self, qh: &QueueHandle<Self>) -> Result<Buffer, Box<dyn std::error::Error>> {
        let width = self.width;
        let height = self.height;
        let stride = width * 4; // 4 bytes per pixel (ARGB8888)
        let size = stride * height;

        // Create shared memory file
        let fd = Self::create_shm_fd(size as usize)?;

        // Create memory map
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        let mmap = unsafe { MmapMut::map_mut(&file)? };

        // Create wl_shm_pool using BorrowedFd
        let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };
        let pool = self.shm.wl_shm().create_pool(
            borrowed_fd,
            size as i32,
            qh,
            (),
        );

        // Create wl_buffer
        let buffer = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );

        pool.destroy();

        Ok(Buffer {
            wl_buffer: buffer,
            mmap,
            width,
            height,
            released: true,
        })
    }

    fn create_shm_fd(size: usize) -> Result<i32, Box<dyn std::error::Error>> {
        use std::ffi::CString;
        use std::os::unix::io::RawFd;

        let name = CString::new("split-keyboard-shm")?;

        // Try memfd_create first (Linux 3.17+)
        let fd: RawFd = unsafe {
            libc::syscall(
                libc::SYS_memfd_create,
                name.as_ptr(),
                libc::MFD_CLOEXEC,
            ) as RawFd
        };

        if fd >= 0 {
            unsafe { libc::ftruncate(fd, size as i64) };
            return Ok(fd);
        }

        // Fallback to shm_open
        let fd = unsafe {
            libc::shm_open(
                name.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
                0o600,
            )
        };

        if fd < 0 {
            return Err("Failed to create shared memory".into());
        }

        unsafe {
            libc::shm_unlink(name.as_ptr());
            libc::ftruncate(fd, size as i64);
        }

        Ok(fd)
    }

    pub fn get_buffer(&mut self) -> Option<&mut Buffer> {
        // Find a released buffer
        for (i, buf) in self.buffers.iter_mut().enumerate() {
            if buf.released {
                self.current_buffer = i;
                buf.released = false;
                return Some(buf);
            }
        }
        None
    }

    pub fn draw(&mut self) {
        if let Some(surface) = self.surface() {
            if let Some(buffer) = self.buffers.get(self.current_buffer) {
                surface.attach(Some(&buffer.wl_buffer), 0, 0);
                surface.damage_buffer(0, 0, buffer.width as i32, buffer.height as i32);
                surface.commit();
            }
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

    pub fn set_vertical_offset(&mut self, offset: i32) {
        self.vertical_offset = offset;

        if let Some(layer_surface) = &self.layer_surface {
            // Set top margin to position the keyboard vertically
            // We're only anchored to top (and left/right), so we can move by adjusting top margin
            let top_margin = offset.max(0) as i32;

            layer_surface.set_margin(top_margin, 0, 0, 0);
            layer_surface.commit();
        }
    }

    pub fn adjust_vertical_offset(&mut self, delta: i32) -> bool {
        let new_offset = self.vertical_offset + delta;
        let max_offset = (self.screen_height as i32 - self.height as i32).max(0);

        // Clamp offset to valid range
        let clamped_offset = new_offset.max(0).min(max_offset);

        if clamped_offset != self.vertical_offset {
            self.set_vertical_offset(clamped_offset);
            true
        } else {
            false
        }
    }

    pub fn can_move_up(&self) -> bool {
        self.vertical_offset > 0
    }

    pub fn can_move_down(&self) -> bool {
        let max_offset = (self.screen_height as i32 - self.height as i32).max(0);
        self.vertical_offset < max_offset
    }
}

use std::os::unix::io::FromRawFd;

impl CompositorHandler for LayerWindow {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for LayerWindow {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for LayerWindow {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        // Layer surface was closed
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        println!("Configure event: compositor says width={}, height={}",
                 configure.new_size.0, configure.new_size.1);
        self.width = configure.new_size.0;
        self.height = configure.new_size.1;
        self.configured = true;
    }
}

impl SeatHandler for LayerWindow {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Pointer capability available, registering pointer");
            self.pointer = self.seat_state.get_pointer(qh, &seat).ok();
        }
        if capability == Capability::Touch && self.touch.is_none() {
            println!("Touch capability available, registering touch");
            self.touch = self.seat_state.get_touch(qh, &seat).ok();
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
        }
        if capability == Capability::Touch {
            if let Some(touch) = self.touch.take() {
                touch.release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl ShmHandler for LayerWindow {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for LayerWindow {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState, SeatState];
}

// Buffer release event handler
impl wayland_client::Dispatch<wl_buffer::WlBuffer, ()> for LayerWindow {
    fn event(
        state: &mut Self,
        buffer: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            // Mark buffer as released
            for buf in &mut state.buffers {
                if &buf.wl_buffer == buffer {
                    buf.released = true;
                    break;
                }
            }
        }
    }
}

// Shm pool dispatch (no-op)
impl wayland_client::Dispatch<WlShmPool, ()> for LayerWindow {
    fn event(
        _: &mut Self,
        _: &WlShmPool,
        _: wayland_client::protocol::wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// Pointer and Touch event handlers
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind};

impl PointerHandler for LayerWindow {
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

impl TouchHandler for LayerWindow {
    fn down(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _serial: u32,
        _time: u32,
        _surface: wl_surface::WlSurface,
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

delegate_compositor!(LayerWindow);
delegate_output!(LayerWindow);
delegate_seat!(LayerWindow);
delegate_layer!(LayerWindow);
delegate_registry!(LayerWindow);
delegate_shm!(LayerWindow);

use smithay_client_toolkit::delegate_pointer;
use smithay_client_toolkit::delegate_touch;

delegate_pointer!(LayerWindow);
delegate_touch!(LayerWindow);
