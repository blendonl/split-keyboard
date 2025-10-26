use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerType {
    Base,
    Symbols,
    Navigation,
    Numbers,
}

impl LayerType {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "base" => LayerType::Base,
            "symbols" => LayerType::Symbols,
            "navigation" => LayerType::Navigation,
            "numbers" => LayerType::Numbers,
            _ => {
                eprintln!("Warning: Unknown layer type '{}', defaulting to Base", s);
                LayerType::Base
            }
        }
    }

    pub fn to_layer_name(&self) -> &'static str {
        match self {
            LayerType::Base => "qwerty",
            LayerType::Symbols => "symbols",
            LayerType::Navigation => "navigation",
            LayerType::Numbers => "numbers",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerActivation {
    Hold,      // Layer active only while key is held
    Toggle,    // Layer toggles on/off with each press
}

pub struct LayerManager {
    /// Stack of active layers (bottom = base layer, top = most recent active layer)
    layer_stack: Vec<LayerType>,
    /// Track which keys are currently held for momentary (Hold) layers
    held_keys: HashMap<u32, LayerType>, // keycode -> layer
    /// Track toggle state for Toggle layers
    toggled_layers: HashSet<LayerType>,
}

impl LayerManager {
    pub fn new() -> Self {
        Self {
            layer_stack: vec![LayerType::Base], // Always start with base layer
            held_keys: HashMap::new(),
            toggled_layers: HashSet::new(),
        }
    }

    /// Get the current active layer (top of stack)
    pub fn current_layer(&self) -> LayerType {
        *self.layer_stack.last().unwrap_or(&LayerType::Base)
    }

    /// Get all active layers (for display purposes)
    pub fn active_layers(&self) -> &[LayerType] {
        &self.layer_stack
    }

    /// Activate a layer with specified activation mode
    /// Returns true if layer state changed
    pub fn activate_layer(&mut self, layer: LayerType, activation: LayerActivation, keycode: u32) -> bool {
        // Special case: "Return to Base" means clear all other layers
        if layer == LayerType::Base {
            // Only clear if we're not already on base layer
            if self.layer_stack.len() > 1 {
                self.layer_stack = vec![LayerType::Base];
                self.held_keys.clear();
                self.toggled_layers.clear();
                return true;
            }
            return false;
        }

        match activation {
            LayerActivation::Hold => {
                // Track the held key
                self.held_keys.insert(keycode, layer);

                // Add layer to stack if not already there
                if !self.layer_stack.contains(&layer) {
                    self.layer_stack.push(layer);
                    true
                } else {
                    false
                }
            }
            LayerActivation::Toggle => {
                if self.toggled_layers.contains(&layer) {
                    // Layer is already toggled on, turn it off
                    self.toggled_layers.remove(&layer);
                    self.layer_stack.retain(|&l| l != layer);
                    true
                } else {
                    // Toggle layer on
                    self.toggled_layers.insert(layer);
                    if !self.layer_stack.contains(&layer) {
                        self.layer_stack.push(layer);
                    }
                    true
                }
            }
        }
    }

    /// Deactivate a layer when key is released (for Hold layers)
    /// Returns true if layer state changed
    pub fn release_layer(&mut self, keycode: u32) -> bool {
        if let Some(layer) = self.held_keys.remove(&keycode) {
            // Only remove if it's not also toggled on
            if !self.toggled_layers.contains(&layer) {
                self.layer_stack.retain(|&l| l != layer);
                return true;
            }
        }
        false
    }

    /// Check if a specific layer is currently active
    pub fn is_layer_active(&self, layer: LayerType) -> bool {
        self.layer_stack.contains(&layer)
    }

    /// Get layer name for display
    pub fn layer_name(&self) -> &'static str {
        match self.current_layer() {
            LayerType::Base => "Base",
            LayerType::Symbols => "Symbols",
            LayerType::Navigation => "Nav",
            LayerType::Numbers => "Numbers",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let manager = LayerManager::new();
        assert_eq!(manager.current_layer(), LayerType::Base);
        assert_eq!(manager.active_layers().len(), 1);
    }

    #[test]
    fn test_hold_activation() {
        let mut manager = LayerManager::new();
        let changed = manager.activate_layer(LayerType::Symbols, LayerActivation::Hold, 100);
        assert!(changed);
        assert_eq!(manager.current_layer(), LayerType::Symbols);

        let changed = manager.release_layer(100);
        assert!(changed);
        assert_eq!(manager.current_layer(), LayerType::Base);
    }

    #[test]
    fn test_toggle_activation() {
        let mut manager = LayerManager::new();

        // Toggle on
        let changed = manager.activate_layer(LayerType::Symbols, LayerActivation::Toggle, 100);
        assert!(changed);
        assert_eq!(manager.current_layer(), LayerType::Symbols);

        // Toggle off
        let changed = manager.activate_layer(LayerType::Symbols, LayerActivation::Toggle, 100);
        assert!(changed);
        assert_eq!(manager.current_layer(), LayerType::Base);
    }

    #[test]
    fn test_layer_stacking() {
        let mut manager = LayerManager::new();

        manager.activate_layer(LayerType::Symbols, LayerActivation::Hold, 100);
        manager.activate_layer(LayerType::Navigation, LayerActivation::Hold, 101);

        // Navigation should be on top
        assert_eq!(manager.current_layer(), LayerType::Navigation);

        // Release navigation
        manager.release_layer(101);
        assert_eq!(manager.current_layer(), LayerType::Symbols);

        // Release symbols
        manager.release_layer(100);
        assert_eq!(manager.current_layer(), LayerType::Base);
    }
}
