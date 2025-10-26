use crate::config::{AppearanceConfig, Config};
use crate::layouts::{Key, KeyboardLayout, KeySide};
use crate::layer_manager::LayerManager;
use fontdue::{Font, FontSettings};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchAction {
    KeyPress(u32),      // Keycode
    MoveUp,             // Move keyboard up
    MoveDown,           // Move keyboard down
}

#[derive(Debug, Clone)]
struct KeyAnimation {
    start_time: Instant,
    touch_x: f32, // Touch point within the key for ripple center
    touch_y: f32,
    is_pressed: bool, // Still being held down
    released_at: Option<Instant>, // When the key was released
}

impl KeyAnimation {
    fn new(touch_x: f32, touch_y: f32) -> Self {
        Self {
            start_time: Instant::now(),
            touch_x,
            touch_y,
            is_pressed: true,
            released_at: None,
        }
    }

    fn release(&mut self) {
        self.is_pressed = false;
        self.released_at = Some(Instant::now());
    }

    fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    fn release_elapsed_ms(&self) -> u64 {
        self.released_at
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }

    fn is_complete(&self, animation_duration_ms: u64) -> bool {
        !self.is_pressed && self.release_elapsed_ms() >= animation_duration_ms
    }
}

pub struct KeyboardUI {
    side: KeySide,
    config: Config,
    appearance: AppearanceConfig,
    keyboard_config: crate::config::KeyboardConfig,
    width: u32,
    height: u32,
    pressed_key: Option<usize>, // Flat index of pressed key
    pub layer_manager: LayerManager,
    pub needs_redraw: bool,
    font: Font,
    animations: HashMap<usize, KeyAnimation>,
    last_frame_time: Instant,
    // Arrow controls
    pub show_up_arrow: bool,
    pub show_down_arrow: bool,
    pressed_arrow: Option<TouchAction>, // Track which arrow is pressed
}

#[derive(Debug, Clone)]
pub struct KeyRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub key_index: usize,
    pub row_index: usize,
}

// Easing function: ease-out cubic for smooth deceleration
fn ease_out_cubic(t: f32) -> f32 {
    let t = t - 1.0;
    t * t * t + 1.0
}

// Easing function: ease-in-out for balanced motion
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let t = 2.0 * t - 2.0;
        1.0 + t * t * t / 2.0
    }
}

impl KeyboardUI {
    pub fn new(side: KeySide, config: &Config) -> Self {
        // Load a simple embedded font (DejaVu Sans)
        let font = Font::from_bytes(
            include_bytes!("../fonts/DejaVuSans.ttf") as &[u8],
            FontSettings::default(),
        )
        .unwrap_or_else(|_| {
            panic!("Failed to load font")
        });

        Self {
            side,
            config: config.clone(),
            appearance: config.appearance.clone(),
            keyboard_config: config.keyboard.clone(),
            width: 0,
            height: 0,
            pressed_key: None,
            layer_manager: LayerManager::new(),
            needs_redraw: true,
            font,
            animations: HashMap::new(),
            last_frame_time: Instant::now(),
            show_up_arrow: true,
            show_down_arrow: true,
            pressed_arrow: None,
        }
    }

    fn get_current_layout(&self) -> KeyboardLayout {
        KeyboardLayout::get_layout(self.layer_manager.current_layer(), self.side, &self.config)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.needs_redraw = true;
    }

    pub fn render(&mut self, buffer: &mut [u32]) {
        if self.width == 0 || self.height == 0 {
            return;
        }

        // Clear background
        let bg_color = self.color_to_u32(&self.appearance.bg_color);
        buffer.fill(bg_color);

        // Calculate key positions and render
        let layout = self.get_current_layout();
        let key_rects = self.calculate_key_positions(&layout);

        for (flat_idx, rect) in key_rects.iter().enumerate() {
            let key = &layout.rows[rect.row_index][rect.key_index];
            let is_pressed = self.pressed_key == Some(flat_idx);
            let anim_state = self.get_animation_state(flat_idx);

            self.draw_key(buffer, rect, key, is_pressed, anim_state);
        }

        // Draw layer indicator
        self.draw_layer_indicator(buffer);

        // Draw arrow controls
        if self.show_up_arrow {
            self.draw_arrow(buffer, true);
        }
        if self.show_down_arrow {
            self.draw_arrow(buffer, false);
        }

        self.needs_redraw = false;
    }

    fn calculate_key_positions(&self, layout: &KeyboardLayout) -> Vec<KeyRect> {
        let mut rects = Vec::new();
        let padding = self.appearance.key_padding as f32;
        let num_rows = layout.rows.len() as f32;

        // First pass: calculate unit_width to determine key dimensions
        let first_row = &layout.rows[0];
        let total_width_units: f32 = first_row.iter().map(|k| k.width).sum();
        let available_width = self.width as f32 - (padding * (first_row.len() + 1) as f32);
        let unit_width = available_width / total_width_units;

        // Make keys square: height equals width (based on a single unit width)
        let key_height = unit_width;

        // Determine if we're using split layout
        let is_split_layout = self.keyboard_config.layout_style == "split";
        let split_gap = if is_split_layout {
            (self.height as f32 * self.appearance.split_spacing_percent as f32) / 100.0
        } else {
            0.0
        };

        // Calculate total height needed for all rows
        let total_rows_height = num_rows * (key_height + padding * 2.0) + split_gap;

        // Center all rows vertically
        let available_height = self.height as f32;
        let vertical_offset = (available_height - total_rows_height) / 2.0;

        for (row_idx, row) in layout.rows.iter().enumerate() {
            // Calculate y position for this row
            let base_y = vertical_offset + row_idx as f32 * (key_height + padding * 2.0);

            // Add split gap if this row comes after the split point
            let y = if is_split_layout && row_idx >= layout.split_after_row {
                base_y + split_gap
            } else {
                base_y
            };

            // Recalculate for this specific row in case rows have different total width units
            let total_width_units: f32 = row.iter().map(|k| k.width).sum();
            let available_width = self.width as f32 - (padding * (row.len() + 1) as f32);
            let unit_width = available_width / total_width_units;

            let mut x = padding;

            for (key_idx, key) in row.iter().enumerate() {
                let key_width = unit_width * key.width;

                rects.push(KeyRect {
                    x,
                    y,
                    width: key_width,
                    height: key_height,
                    key_index: key_idx,
                    row_index: row_idx,
                });

                x += key_width + padding;
            }
        }

        rects
    }

    fn draw_key(&self, buffer: &mut [u32], rect: &KeyRect, key: &Key, is_pressed: bool, anim_state: Option<AnimationState>) {
        // Calculate scaled rectangle if animation is active
        let (scaled_rect, _scale_factor) = if let Some(ref anim) = anim_state {
            if self.appearance.enable_animations {
                let center_x = rect.x + rect.width / 2.0;
                let center_y = rect.y + rect.height / 2.0;
                let new_width = rect.width * anim.scale_factor;
                let new_height = rect.height * anim.scale_factor;

                let scaled = KeyRect {
                    x: center_x - new_width / 2.0,
                    y: center_y - new_height / 2.0,
                    width: new_width,
                    height: new_height,
                    key_index: rect.key_index,
                    row_index: rect.row_index,
                };
                (scaled, anim.scale_factor)
            } else {
                (rect.clone(), 1.0)
            }
        } else {
            (rect.clone(), 1.0)
        };

        // Calculate key color with glow effect
        let base_color = if is_pressed {
            self.appearance.key_pressed_color
        } else {
            self.appearance.key_color
        };

        let key_color = if let Some(ref anim) = anim_state {
            if self.appearance.enable_animations {
                self.apply_glow(base_color, anim.glow_intensity)
            } else {
                self.color_to_u32(&base_color)
            }
        } else {
            self.color_to_u32(&base_color)
        };

        let border_color = self.color_to_u32(&self.appearance.border_color);
        let border_width = self.appearance.border_width;

        // Draw filled rectangle
        self.fill_rect(
            buffer,
            scaled_rect.x as u32,
            scaled_rect.y as u32,
            scaled_rect.width as u32,
            scaled_rect.height as u32,
            key_color,
        );

        // Draw border
        if border_width > 0 {
            self.draw_rect_border(
                buffer,
                scaled_rect.x as u32,
                scaled_rect.y as u32,
                scaled_rect.width as u32,
                scaled_rect.height as u32,
                border_width,
                border_color,
            );
        }

        // Draw ripple effect if animation is active
        if let Some(ref anim) = anim_state {
            if self.appearance.enable_animations && anim.ripple_progress > 0.0 {
                self.draw_ripple(buffer, &scaled_rect, anim);
            }
        }

        // Draw label (centered text)
        let label = &key.label;
        self.draw_text(buffer, label, &scaled_rect);
    }

    fn draw_text(&self, buffer: &mut [u32], text: &str, rect: &KeyRect) {
        let font_size = self.appearance.font_size as f32;
        let text_color = self.color_to_u32(&self.appearance.text_color);

        // Calculate total width of the text
        let mut total_width = 0.0;
        let mut char_metrics = Vec::new();
        for ch in text.chars() {
            let (metrics, _) = self.font.rasterize(ch, font_size);
            total_width += metrics.advance_width;
            char_metrics.push(metrics);
        }

        // Calculate starting position (centered)
        let mut cursor_x = rect.x + (rect.width - total_width) / 2.0;
        let baseline_y = rect.y + rect.height / 2.0;

        // Render each character
        for (ch, metrics) in text.chars().zip(char_metrics.iter()) {
            let (_, bitmap) = self.font.rasterize(ch, font_size);

            // Position for this character
            let char_x = cursor_x + metrics.xmin as f32;
            let char_y = baseline_y - metrics.height as f32 / 2.0;

            // Draw each pixel of the character
            for (i, &alpha) in bitmap.iter().enumerate() {
                if alpha == 0 {
                    continue;
                }

                let px = i % metrics.width;
                let py = i / metrics.width;

                let screen_x = (char_x + px as f32) as u32;
                let screen_y = (char_y + py as f32) as u32;

                if screen_x < self.width && screen_y < self.height {
                    let idx = (screen_y * self.width + screen_x) as usize;
                    if idx < buffer.len() {
                        // Blend the text color with alpha
                        let bg = buffer[idx];
                        buffer[idx] = self.blend_pixel(bg, text_color, alpha);
                    }
                }
            }

            cursor_x += metrics.advance_width;
        }
    }

    fn blend_pixel(&self, bg: u32, fg: u32, alpha: u8) -> u32 {
        if alpha == 255 {
            return fg;
        }

        let alpha_f = alpha as f32 / 255.0;
        let inv_alpha = 1.0 - alpha_f;

        let bg_r = ((bg >> 16) & 0xFF) as f32;
        let bg_g = ((bg >> 8) & 0xFF) as f32;
        let bg_b = (bg & 0xFF) as f32;

        let fg_r = ((fg >> 16) & 0xFF) as f32;
        let fg_g = ((fg >> 8) & 0xFF) as f32;
        let fg_b = (fg & 0xFF) as f32;

        let r = (bg_r * inv_alpha + fg_r * alpha_f) as u32;
        let g = (bg_g * inv_alpha + fg_g * alpha_f) as u32;
        let b = (bg_b * inv_alpha + fg_b * alpha_f) as u32;

        0xFF000000 | (r << 16) | (g << 8) | b
    }

    fn fill_rect(&self, buffer: &mut [u32], x: u32, y: u32, w: u32, h: u32, color: u32) {
        for py in y..y + h {
            if py >= self.height {
                break;
            }
            for px in x..x + w {
                if px >= self.width {
                    break;
                }
                let idx = (py * self.width + px) as usize;
                if idx < buffer.len() {
                    buffer[idx] = color;
                }
            }
        }
    }

    fn draw_rect_border(
        &self,
        buffer: &mut [u32],
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        border_width: u32,
        color: u32,
    ) {
        // Top border
        self.fill_rect(buffer, x, y, w, border_width, color);
        // Bottom border
        self.fill_rect(buffer, x, y + h - border_width, w, border_width, color);
        // Left border
        self.fill_rect(buffer, x, y, border_width, h, color);
        // Right border
        self.fill_rect(buffer, x + w - border_width, y, border_width, h, color);
    }

    fn color_to_u32(&self, rgba: &[u8; 4]) -> u32 {
        // Format: 0xAARRGGBB (or 0xFFRRGGBB for opaque)
        let [r, g, b, a] = *rgba;
        ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }

    fn apply_glow(&self, base_color: [u8; 4], intensity: f32) -> u32 {
        let [r, g, b, a] = base_color;

        // Brighten the color based on glow intensity
        let glow_amount = intensity * 0.4; // 40% max brightness increase
        let r = ((r as f32) * (1.0 + glow_amount)).min(255.0) as u8;
        let g = ((g as f32) * (1.0 + glow_amount)).min(255.0) as u8;
        let b = ((b as f32) * (1.0 + glow_amount)).min(255.0) as u8;

        self.color_to_u32(&[r, g, b, a])
    }

    fn draw_ripple(&self, buffer: &mut [u32], rect: &KeyRect, anim: &AnimationState) {
        let ripple_color = self.appearance.ripple_color;

        // Calculate ripple center (absolute coordinates)
        let center_x = rect.x + anim.touch_x * rect.width;
        let center_y = rect.y + anim.touch_y * rect.height;

        // Maximum ripple radius (diagonal of the key)
        let max_radius = ((rect.width * rect.width + rect.height * rect.height).sqrt()) / 2.0;
        let current_radius = max_radius * anim.ripple_progress;

        // Alpha decreases as ripple expands
        let alpha = ((1.0 - anim.ripple_progress) * ripple_color[3] as f32) as u8;

        if alpha == 0 {
            return;
        }

        let ripple_color_with_alpha = [ripple_color[0], ripple_color[1], ripple_color[2], alpha];
        let ripple_u32 = self.color_to_u32(&ripple_color_with_alpha);

        // Draw ripple as a filled circle
        let min_x = (center_x - current_radius).max(rect.x) as u32;
        let max_x = (center_x + current_radius).min(rect.x + rect.width) as u32;
        let min_y = (center_y - current_radius).max(rect.y) as u32;
        let max_y = (center_y + current_radius).min(rect.y + rect.height) as u32;

        for py in min_y..max_y {
            if py >= self.height {
                break;
            }
            for px in min_x..max_x {
                if px >= self.width {
                    break;
                }

                let dx = px as f32 - center_x;
                let dy = py as f32 - center_y;
                let dist = (dx * dx + dy * dy).sqrt();

                // Only draw if within ripple radius
                if dist <= current_radius {
                    let idx = (py * self.width + px) as usize;
                    if idx < buffer.len() {
                        // Blend ripple with existing pixel
                        let bg = buffer[idx];
                        buffer[idx] = self.blend_pixel(bg, ripple_u32, alpha);
                    }
                }
            }
        }
    }

    fn draw_layer_indicator(&self, buffer: &mut [u32]) {
        let layer_name = self.layer_manager.layer_name();
        if layer_name == "Base" {
            return; // Don't show indicator for base layer
        }

        // Position at top center
        let text_size = 14.0;
        let padding = 5.0;
        let text_color = self.color_to_u32(&self.appearance.text_color);

        // Calculate text width
        let mut total_width = 0.0;
        for ch in layer_name.chars() {
            let (metrics, _) = self.font.rasterize(ch, text_size);
            total_width += metrics.advance_width;
        }

        let x = (self.width as f32 - total_width) / 2.0;
        let y = padding;

        // Draw each character
        let mut cursor_x = x;
        for ch in layer_name.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, text_size);

            let char_x = cursor_x + metrics.xmin as f32;
            let char_y = y;

            for (i, &alpha) in bitmap.iter().enumerate() {
                if alpha == 0 {
                    continue;
                }

                let px = i % metrics.width;
                let py = i / metrics.width;

                let screen_x = (char_x + px as f32) as u32;
                let screen_y = (char_y + py as f32) as u32;

                if screen_x < self.width && screen_y < self.height {
                    let idx = (screen_y * self.width + screen_x) as usize;
                    if idx < buffer.len() {
                        let bg = buffer[idx];
                        buffer[idx] = self.blend_pixel(bg, text_color, alpha);
                    }
                }
            }

            cursor_x += metrics.advance_width;
        }
    }

    fn get_arrow_rect(&self, is_up: bool) -> (f32, f32, f32, f32) {
        let arrow_size = self.appearance.arrow_size as f32;
        let center_x = self.width as f32 / 2.0;
        let x = center_x - arrow_size / 2.0;
        let margin = 10.0;

        // Calculate keyboard bounds
        let layout = self.get_current_layout();
        let key_rects = self.calculate_key_positions(&layout);
        let keyboard_top = key_rects.iter().map(|r| r.y).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);
        let keyboard_bottom = key_rects.iter().map(|r| r.y + r.height).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(self.height as f32);

        let y = if is_up {
            keyboard_top - arrow_size - margin
        } else {
            keyboard_bottom + margin
        };

        (x, y, arrow_size, arrow_size)
    }

    fn draw_arrow(&self, buffer: &mut [u32], is_up: bool) {
        let (x, y, size, _) = self.get_arrow_rect(is_up);

        // Determine if this arrow is pressed
        let action = if is_up { TouchAction::MoveUp } else { TouchAction::MoveDown };
        let is_pressed = self.pressed_arrow == Some(action);

        let arrow_color = if is_pressed {
            self.appearance.arrow_pressed_color
        } else {
            self.appearance.arrow_color
        };

        let color = self.color_to_u32(&arrow_color);
        let border_color = self.color_to_u32(&self.appearance.border_color);

        // Draw background circle
        let center_x = x + size / 2.0;
        let center_y = y + size / 2.0;
        let radius = size / 2.0;

        for py in (y as u32)..(y + size) as u32 {
            if py >= self.height {
                break;
            }
            for px in (x as u32)..(x + size) as u32 {
                if px >= self.width {
                    break;
                }

                let dx = px as f32 - center_x;
                let dy = py as f32 - center_y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= radius {
                    let idx = (py * self.width + px) as usize;
                    if idx < buffer.len() {
                        buffer[idx] = color;
                    }
                }
            }
        }

        // Draw arrow triangle
        let triangle_height = size * 0.3;
        let triangle_width = size * 0.4;

        if is_up {
            // Upward pointing triangle
            let tip_y = center_y - triangle_height / 2.0;
            let _base_y = center_y + triangle_height / 2.0;

            for py_i in 0..triangle_height as u32 {
                let py = (tip_y + py_i as f32) as u32;
                if py >= self.height {
                    continue;
                }

                let progress = py_i as f32 / triangle_height;
                let half_width = triangle_width * progress / 2.0;

                let start_x = (center_x - half_width) as u32;
                let end_x = (center_x + half_width) as u32;

                for px in start_x..=end_x {
                    if px < self.width {
                        let idx = (py * self.width + px) as usize;
                        if idx < buffer.len() {
                            buffer[idx] = border_color;
                        }
                    }
                }
            }
        } else {
            // Downward pointing triangle
            let _tip_y = center_y + triangle_height / 2.0;
            let base_y = center_y - triangle_height / 2.0;

            for py_i in 0..triangle_height as u32 {
                let py = (base_y + py_i as f32) as u32;
                if py >= self.height {
                    continue;
                }

                let progress = 1.0 - (py_i as f32 / triangle_height);
                let half_width = triangle_width * progress / 2.0;

                let start_x = (center_x - half_width) as u32;
                let end_x = (center_x + half_width) as u32;

                for px in start_x..=end_x {
                    if px < self.width {
                        let idx = (py * self.width + px) as usize;
                        if idx < buffer.len() {
                            buffer[idx] = border_color;
                        }
                    }
                }
            }
        }
    }

    pub fn handle_touch(&mut self, x: f32, y: f32, pressed: bool) -> Option<TouchAction> {
        // Check arrow touches first (they have priority)
        if self.show_up_arrow {
            let (ax, ay, aw, ah) = self.get_arrow_rect(true);
            if x >= ax && x <= ax + aw && y >= ay && y <= ay + ah {
                if pressed {
                    self.pressed_arrow = Some(TouchAction::MoveUp);
                    self.needs_redraw = true;
                    return Some(TouchAction::MoveUp);
                } else {
                    self.pressed_arrow = None;
                    self.needs_redraw = true;
                    return None;
                }
            }
        }

        if self.show_down_arrow {
            let (ax, ay, aw, ah) = self.get_arrow_rect(false);
            if x >= ax && x <= ax + aw && y >= ay && y <= ay + ah {
                if pressed {
                    self.pressed_arrow = Some(TouchAction::MoveDown);
                    self.needs_redraw = true;
                    return Some(TouchAction::MoveDown);
                } else {
                    self.pressed_arrow = None;
                    self.needs_redraw = true;
                    return None;
                }
            }
        }

        // Clear arrow press if touching elsewhere
        if self.pressed_arrow.is_some() {
            self.pressed_arrow = None;
            self.needs_redraw = true;
        }

        // Check key touches
        let layout = self.get_current_layout();
        let key_rects = self.calculate_key_positions(&layout);

        for (flat_idx, rect) in key_rects.iter().enumerate() {
            if x >= rect.x
                && x <= rect.x + rect.width
                && y >= rect.y
                && y <= rect.y + rect.height
            {
                let key = &layout.rows[rect.row_index][rect.key_index];

                if pressed {
                    self.pressed_key = Some(flat_idx);
                    self.needs_redraw = true;

                    // Start animation with normalized touch coordinates within the key
                    if self.appearance.enable_animations {
                        let touch_x_norm = (x - rect.x) / rect.width;
                        let touch_y_norm = (y - rect.y) / rect.height;
                        self.animations.insert(flat_idx, KeyAnimation::new(touch_x_norm, touch_y_norm));
                    }

                    // Handle layer switching
                    if let Some((layer, activation)) = key.layer_switch {
                        // Use a dummy keycode for layer switches (they don't send actual keys)
                        let dummy_keycode = 1000 + flat_idx as u32;
                        if self.layer_manager.activate_layer(layer, activation, dummy_keycode) {
                            self.needs_redraw = true;
                        }
                        return None; // Layer switches don't send keycodes
                    }

                    // Return keycode to send (if this is a regular key)
                    return key.keycode.map(TouchAction::KeyPress);
                } else {
                    // Key released
                    self.pressed_key = None;
                    self.needs_redraw = true;

                    // Mark animation as released (if exists)
                    if let Some(anim) = self.animations.get_mut(&flat_idx) {
                        anim.release();
                    }

                    // Handle layer release for Hold layers
                    if key.layer_switch.is_some() {
                        let dummy_keycode = 1000 + flat_idx as u32;
                        if self.layer_manager.release_layer(dummy_keycode) {
                            self.needs_redraw = true;
                        }
                        return None;
                    }

                    // Return keycode for release event
                    return key.keycode.map(TouchAction::KeyPress);
                }
            }
        }

        if self.pressed_key.is_some() {
            self.pressed_key = None;
            self.needs_redraw = true;

            // Release any active animation
            if let Some(idx) = self.pressed_key {
                if let Some(anim) = self.animations.get_mut(&idx) {
                    anim.release();
                }
            }
        }
        None
    }

    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    pub fn has_active_animations(&self) -> bool {
        !self.animations.is_empty()
    }

    pub fn update_animations(&mut self) {
        let animation_duration_ms = self.appearance.animation_duration_ms as u64;

        // Remove completed animations
        self.animations.retain(|_, anim| !anim.is_complete(animation_duration_ms));

        // Mark for redraw if we have active animations
        if !self.animations.is_empty() {
            self.needs_redraw = true;
        }

        self.last_frame_time = Instant::now();
    }

    fn get_animation_state(&self, key_index: usize) -> Option<AnimationState> {
        self.animations.get(&key_index).map(|anim| {
            let animation_duration_ms = self.appearance.animation_duration_ms as u64;
            let press_duration = 150; // First 150ms is press phase

            let (progress, is_releasing) = if anim.is_pressed {
                // Still pressed - animate press phase
                let t = (anim.elapsed_ms().min(press_duration) as f32) / press_duration as f32;
                (t, false)
            } else {
                // Released - animate release phase
                let release_ms = anim.release_elapsed_ms();
                let release_duration = animation_duration_ms.saturating_sub(press_duration);
                let t = (release_ms.min(release_duration) as f32) / release_duration as f32;
                (t, true)
            };

            AnimationState {
                progress,
                is_releasing,
                ripple_progress: if is_releasing {
                    ease_out_cubic(progress)
                } else {
                    progress * 0.3 // Ripple starts slow during press
                },
                scale_factor: if is_releasing {
                    // Scale back to 1.0
                    let scale_min = 1.0 - self.appearance.scale_amount;
                    scale_min + (1.0 - scale_min) * ease_out_cubic(progress)
                } else {
                    // Scale down
                    1.0 - self.appearance.scale_amount * ease_in_out_cubic(progress)
                },
                glow_intensity: if is_releasing {
                    1.0 - ease_out_cubic(progress)
                } else {
                    ease_in_out_cubic(progress)
                },
                touch_x: anim.touch_x,
                touch_y: anim.touch_y,
            }
        })
    }
}

#[derive(Debug, Clone)]
struct AnimationState {
    progress: f32,
    is_releasing: bool,
    ripple_progress: f32,
    scale_factor: f32,
    glow_intensity: f32,
    touch_x: f32,
    touch_y: f32,
}
