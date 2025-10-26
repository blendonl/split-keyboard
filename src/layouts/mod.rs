use crate::layer_manager::{LayerType, LayerActivation};
use crate::config::{Config, LayerConfig, KeyConfig};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeySide {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct Key {
    pub label: String,
    pub keycode: Option<u32>, // None for layer switch keys
    pub width: f32, // Relative width (1.0 = standard key)
    pub layer_switch: Option<(LayerType, LayerActivation)>, // Layer switching info
}

#[derive(Debug, Clone)]
pub struct KeyboardLayout {
    pub name: String,
    pub rows: Vec<Vec<Key>>,
    pub split_after_row: usize, // Split layout after this row index (0-based)
}

impl KeyboardLayout {
    pub fn get_layout(layer: LayerType, side: KeySide, config: &Config) -> Self {
        // Determine the layer config key based on layer type and side
        let layer_key = format!("{}_{}",
            layer.to_layer_name().to_lowercase(),
            match side {
                KeySide::Left => "left",
                KeySide::Right => "right",
            }
        );

        // Try to load from config
        if let Some(layer_config) = config.get_layer(&layer_key) {
            Self::from_config(layer_config, config)
        } else {
            // Fallback to empty layout if config not found
            eprintln!("Warning: Layer config '{}' not found, using empty layout", layer_key);
            Self {
                name: format!("{:?} {:?}", layer, side),
                rows: vec![],
                split_after_row: config.keyboard.split_after_row.unwrap_or(3),
            }
        }
    }

    fn from_config(layer_config: &LayerConfig, global_config: &Config) -> Self {
        let mut rows = Vec::new();

        for row_config in &layer_config.rows {
            let mut row = Vec::new();
            for key_config in &row_config.keys {
                row.push(Key::from_config(key_config));
            }
            rows.push(row);
        }

        Self {
            name: layer_config.layer.name.clone(),
            rows,
            split_after_row: layer_config.layer.split_after_row
                .or(global_config.keyboard.split_after_row)
                .unwrap_or(3),
        }
    }
}

impl Key {
    pub fn new(label: impl Into<String>, keycode: u32) -> Self {
        Self {
            label: label.into(),
            keycode: Some(keycode),
            width: 1.0,
            layer_switch: None,
        }
    }

    pub fn layer_switch(label: impl Into<String>, layer: LayerType, activation: LayerActivation) -> Self {
        Self {
            label: label.into(),
            keycode: None,
            width: 1.0,
            layer_switch: Some((layer, activation)),
        }
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn from_config(config: &KeyConfig) -> Self {
        let layer_switch = if let Some(ref ls_config) = config.layer_switch {
            let layer = LayerType::from_string(&ls_config.layer);
            let activation = match ls_config.activation.to_lowercase().as_str() {
                "toggle" => LayerActivation::Toggle,
                "hold" => LayerActivation::Hold,
                _ => {
                    eprintln!("Warning: Unknown activation type '{}', defaulting to Toggle", ls_config.activation);
                    LayerActivation::Toggle
                }
            };
            Some((layer, activation))
        } else {
            None
        };

        Self {
            label: config.label.clone(),
            keycode: config.keycode,
            width: config.width,
            layer_switch,
        }
    }
}
