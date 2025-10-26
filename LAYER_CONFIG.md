# Layer Configuration Guide

This document explains how to configure keyboard layers in the split keyboard.

## Overview

Layers are completely configurable via TOML files. Each layer is defined in a separate file in the `~/.config/split-keyboard/layers/` directory.

## Configuration Structure

### Global Settings (config.toml)

The main configuration file (`~/.config/split-keyboard/config.toml`) contains a global `split_after_row` setting:

```toml
[keyboard]
split_after_row = 3  # Split keyboard after row 3 (0-indexed)
layout_style = "split"  # Use "standard" for no gap
```

This setting determines where the visual gap appears between rows. It can be overridden per-layer.

### Layer Files

Layer files are named following this pattern: `{layer_name}_{side}.toml`

Required files for a complete keyboard:
- `qwerty_left.toml` and `qwerty_right.toml` (Base layer)
- `symbols_left.toml` and `symbols_right.toml`
- `navigation_left.toml` and `navigation_right.toml`
- `numbers_left.toml` and `numbers_right.toml`

## Layer File Structure

```toml
[layer]
name = "Layer Display Name"
split_after_row = 3  # Optional: override global setting

# Define each row with [[row]]
[[row]]
keys = [
  { label = "Key", keycode = 42 },
  { label = "Wide", keycode = 43, width = 2.0 },
  { label = "Switch", layer_switch = { layer = "Symbols", activation = "Toggle" } }
]

[[row]]
keys = [
  # More keys...
]
```

## Key Properties

Each key in the `keys` array can have the following properties:

### Required Properties

- **`label`** (string): Text displayed on the key
  - Example: `"a"`, `"Enter"`, `"Shift"`

### Optional Properties

- **`keycode`** (integer): Linux input event keycode
  - Omit for layer switch keys
  - See [Linux keycodes reference](https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h)
  - Common examples:
    - Letters: a=30, b=48, c=46, etc.
    - Numbers: 1=2, 2=3, 3=4, etc.
    - Special: Enter=28, Backspace=14, Space=57, Tab=15

- **`width`** (float): Relative width of the key
  - Default: `1.0` (standard width)
  - Use `2.0` for double-width keys (e.g., Space, Enter)
  - Example: `{ label = "Space", keycode = 57, width = 2.0 }`

- **`layer_switch`** (object): Define layer switching behavior
  - **`layer`** (string): Target layer name
    - Options: `"Base"`, `"Symbols"`, `"Navigation"`, `"Numbers"`
  - **`activation`** (string): Activation mode
    - `"Toggle"`: Press once to activate, press again to deactivate
    - `"Hold"`: Layer active only while key is held down
  - Example: `{ label = "Sym", layer_switch = { layer = "Symbols", activation = "Toggle" } }`

## Examples

### Simple Key
```toml
{ label = "a", keycode = 30 }
```

### Wide Key
```toml
{ label = "Enter", keycode = 28, width = 2.0 }
```

### Layer Switch (Toggle)
```toml
{ label = "Sym", layer_switch = { layer = "Symbols", activation = "Toggle" } }
```

### Layer Switch (Hold)
```toml
{ label = "Nav", layer_switch = { layer = "Navigation", activation = "Hold" } }
```

### Return to Base Layer
```toml
{ label = "Base", layer_switch = { layer = "Base", activation = "Toggle" } }
```

## Per-Layer Row Count

Each layer can have a different number of rows. Simply define as many `[[row]]` sections as you need:

```toml
[layer]
name = "Custom Layer"

[[row]]
keys = [{ label = "1", keycode = 2 }]

[[row]]
keys = [{ label = "2", keycode = 3 }]

[[row]]
keys = [{ label = "3", keycode = 4 }]
# Add more rows as needed
```

## Per-Layer Split Configuration

Each layer can override the global `split_after_row` setting:

```toml
[layer]
name = "Navigation Left"
split_after_row = 2  # Split after row 2 for this layer (overrides global)
```

If not specified, the layer uses the global `split_after_row` from `config.toml`.

## Installation

1. Copy the example layer files to your config directory:
   ```bash
   mkdir -p ~/.config/split-keyboard/layers
   cp layers/*.toml ~/.config/split-keyboard/layers/
   ```

2. Edit the layer files to customize your keyboard layout

3. Restart the split keyboard application

## Common Linux Keycodes

Here are some commonly used keycodes:

| Key | Keycode | Key | Keycode | Key | Keycode |
|-----|---------|-----|---------|-----|---------|
| 1 | 2 | q | 16 | a | 30 |
| 2 | 3 | w | 17 | s | 31 |
| 3 | 4 | e | 18 | d | 32 |
| 4 | 5 | r | 19 | f | 33 |
| 5 | 6 | t | 20 | g | 34 |
| 6 | 7 | y | 21 | h | 35 |
| 7 | 8 | u | 22 | j | 36 |
| 8 | 9 | i | 23 | k | 37 |
| 9 | 10 | o | 24 | l | 38 |
| 0 | 11 | p | 25 | z | 44 |
| - | 12 | [ | 26 | x | 45 |
| = | 13 | ] | 27 | c | 46 |
| Backspace | 14 | Enter | 28 | v | 47 |
| Tab | 15 | Ctrl | 29 | b | 48 |
| ; | 39 | Shift | 42 | n | 49 |
| ' | 40 | \\ | 43 | m | 50 |
| \` | 41 | , | 51 | Space | 57 |
| / | 53 | . | 52 | Alt | 56 |

### Special Keys

| Key | Keycode | Key | Keycode |
|-----|---------|-----|---------|
| Esc | 1 | Left Arrow | 105 |
| F1 | 59 | Right Arrow | 106 |
| F2 | 60 | Up Arrow | 103 |
| F3 | 61 | Down Arrow | 108 |
| F4 | 62 | Home | 102 |
| F5 | 63 | End | 107 |
| F6 | 64 | Page Up | 104 |
| F7 | 65 | Page Down | 109 |
| F8 | 66 | Insert | 110 |
| F9 | 67 | Delete | 111 |
| F10 | 68 | | |
| F11 | 87 | | |
| F12 | 88 | | |

For a complete list, see the [Linux input event codes](https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h).

## Troubleshooting

### Layer not appearing

1. Check that the layer file exists in `~/.config/split-keyboard/layers/`
2. Verify the filename follows the `{layer_name}_{side}.toml` pattern
3. Check for TOML syntax errors in the file
4. Look for error messages in the terminal when starting the keyboard

### Keys not working

1. Verify the `keycode` values are correct Linux input event codes
2. Check that you have permission to access `/dev/uinput`
3. Ensure the key doesn't have both `keycode` and `layer_switch` defined

### Layout looks wrong

1. Check the `split_after_row` value (should be 0-indexed)
2. Verify all rows have the same total width (sum of key widths)
3. Adjust `key_padding` and `width_percent` in the main config
