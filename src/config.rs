use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub window: WindowConfig,
    pub appearance: AppearanceConfig,
    pub keyboard: KeyboardConfig,
    #[serde(skip)]
    pub layers: HashMap<String, LayerConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LayerConfig {
    pub layer: LayerMetadata,
    #[serde(rename = "row")]
    pub rows: Vec<RowConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LayerMetadata {
    pub name: String,
    #[serde(default)]
    pub split_after_row: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RowConfig {
    pub keys: Vec<KeyConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct KeyConfig {
    pub label: String,
    #[serde(default)]
    pub keycode: Option<u32>,
    #[serde(default = "default_key_width")]
    pub width: f32,
    #[serde(default)]
    pub layer_switch: Option<LayerSwitchConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LayerSwitchConfig {
    pub layer: String,
    pub activation: String, // "Toggle" or "Hold"
}

fn default_key_width() -> f32 {
    1.0
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowConfig {
    pub width_percent: u32,
    pub layer: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppearanceConfig {
    pub bg_color: [u8; 4],
    pub key_color: [u8; 4],
    pub key_pressed_color: [u8; 4],
    pub text_color: [u8; 4],
    pub border_color: [u8; 4],
    pub key_padding: u32,
    pub border_width: u32,
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    #[serde(default = "default_vertical_spacing_multiplier")]
    pub vertical_spacing_multiplier: f32,
    #[serde(default = "default_enable_animations")]
    pub enable_animations: bool,
    #[serde(default = "default_animation_duration_ms")]
    pub animation_duration_ms: u32,
    #[serde(default = "default_ripple_color")]
    pub ripple_color: [u8; 4],
    #[serde(default = "default_scale_amount")]
    pub scale_amount: f32,
    #[serde(default = "default_vertical_move_percent")]
    pub vertical_move_percent: u32,
    #[serde(default = "default_arrow_size")]
    pub arrow_size: u32,
    #[serde(default = "default_arrow_color")]
    pub arrow_color: [u8; 4],
    #[serde(default = "default_arrow_pressed_color")]
    pub arrow_pressed_color: [u8; 4],
    #[serde(default = "default_split_spacing_percent")]
    pub split_spacing_percent: u32,
}

fn default_font_size() -> u32 {
    18
}

fn default_vertical_spacing_multiplier() -> f32 {
    1.25
}

fn default_enable_animations() -> bool {
    true
}

fn default_animation_duration_ms() -> u32 {
    300
}

fn default_ripple_color() -> [u8; 4] {
    [255, 255, 255, 100] // White with some transparency
}

fn default_scale_amount() -> f32 {
    0.05 // 5% scale down
}

fn default_vertical_move_percent() -> u32 {
    5
}

fn default_arrow_size() -> u32 {
    45
}

fn default_arrow_color() -> [u8; 4] {
    [100, 100, 100, 255]
}

fn default_arrow_pressed_color() -> [u8; 4] {
    [150, 150, 150, 255]
}

fn default_split_spacing_percent() -> u32 {
    10
}

#[derive(Debug, Deserialize, Clone)]
pub struct KeyboardConfig {
    pub default_layout: String,
    pub rows: u32,
    pub haptic_feedback: bool,
    #[serde(default = "default_layout_style")]
    pub layout_style: String,
    #[serde(default)]
    pub split_after_row: Option<usize>,
}

fn default_layout_style() -> String {
    "standard".to_string()
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::config_path()?;

        let mut config = if !config_path.exists() {
            Self::default()
        } else {
            let contents = fs::read_to_string(config_path)?;
            toml::from_str(&contents)?
        };

        // Load all layer files from the layers directory
        config.layers = Self::load_layers()?;

        Ok(config)
    }

    fn config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut path = dirs::config_dir()
            .or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
            .ok_or("Cannot find config directory")?;

        path.push("split-keyboard");
        fs::create_dir_all(&path)?;
        path.push("config.toml");
        Ok(path)
    }

    fn layers_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut path = dirs::config_dir()
            .or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
            .ok_or("Cannot find config directory")?;

        path.push("split-keyboard");
        path.push("layers");
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    fn load_layers() -> Result<HashMap<String, LayerConfig>, Box<dyn std::error::Error>> {
        let layers_dir = Self::layers_dir()?;
        let mut layers = HashMap::new();

        // Check if layers directory exists and has files
        if !layers_dir.exists() {
            return Ok(layers); // Return empty if no layers directory
        }

        // Read all .toml files from the layers directory
        for entry in fs::read_dir(layers_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Only process .toml files
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let contents = fs::read_to_string(&path)?;
                let layer_config: LayerConfig = toml::from_str(&contents)?;

                // Use filename without extension as the key
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    layers.insert(filename.to_string(), layer_config);
                }
            }
        }

        Ok(layers)
    }

    pub fn get_layer(&self, layer_name: &str) -> Option<&LayerConfig> {
        self.layers.get(layer_name)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window: WindowConfig {
                width_percent: 25,
                layer: "overlay".to_string(),
            },
            appearance: AppearanceConfig {
                bg_color: [30, 30, 30, 230],
                key_color: [60, 60, 60, 255],
                key_pressed_color: [100, 100, 100, 255],
                text_color: [255, 255, 255, 255],
                border_color: [80, 80, 80, 255],
                key_padding: 4,
                border_width: 1,
                font_size: 18,
                vertical_spacing_multiplier: 1.25,
                enable_animations: true,
                animation_duration_ms: 300,
                ripple_color: [255, 255, 255, 100],
                scale_amount: 0.05,
                vertical_move_percent: 5,
                arrow_size: 45,
                arrow_color: [100, 100, 100, 255],
                arrow_pressed_color: [150, 150, 150, 255],
                split_spacing_percent: 10,
            },
            keyboard: KeyboardConfig {
                default_layout: "qwerty".to_string(),
                rows: 4,
                haptic_feedback: false,
                layout_style: "standard".to_string(),
                split_after_row: Some(3),
            },
            layers: HashMap::new(),
        }
    }
}

// Helper module since we don't want to add dirs as dependency
mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".config"))
            })
    }
}
