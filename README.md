# Split Keyboard for Linux Tablet Mode

A split on-screen keyboard designed for Hyprland (and other wlroots-based Wayland compositors) that provides two keyboard panels positioned on the left and right edges of the screen, perfect for tablet mode.

## Features

- **Split Design**: Two separate keyboards on left and right screen edges
- **Full Height Coverage**: Each keyboard takes configurable width and 100% height
- **Wayland Native**: Uses wlr-layer-shell protocol for proper overlay positioning
- **Virtual Keyboard Support**: Uses Linux uinput to send actual keystrokes to applications
- **Customizable**: Configure colors, sizes, and layouts via TOML config file
- **Multiple Layers**: Support for Base (QWERTY), Symbols, Navigation, and Numbers layers
- **Flexible Layer Switching**: Both toggle and hold-to-activate layer switching modes
- **Touch Animations**: Visual feedback with ripple effects, scaling, and glow animations

## Requirements

- Linux with Wayland compositor supporting wlr-layer-shell (Hyprland, Sway, etc.)
- `libwayland-client` at runtime (the `wayland` package on Arch)
- Read and write access to `/dev/uinput`, see [Setting up uinput permissions](#setting-up-uinput-permissions)
- A current stable Rust toolchain to build. `Cargo.lock` is not committed, so dependencies resolve to their latest releases, which currently need Rust 1.87 or newer.

## Install

### From source

```bash
git clone https://github.com/blendonl/split-keyboard.git
cd split-keyboard
cargo build --release
sudo install -Dm755 target/release/split-keyboard /usr/local/bin/split-keyboard

# Or install into ~/.cargo/bin
cargo install --path .
```

### Arch Linux (AUR)

`packaging/aur/PKGBUILD` builds the `split-keyboard-git` package from the latest commit on `main`. Once it is published on the AUR, install it with an AUR helper:

```bash
yay -S split-keyboard-git
```

Until then, build it from a clone:

```bash
cd packaging/aur
makepkg -si
```

The package installs the binary to `/usr/bin/split-keyboard`, the example `config.toml` and layer files to `/usr/share/split-keyboard/`, and the docs to `/usr/share/doc/split-keyboard/`.

### Layer files

The keyboard reads its layouts from `~/.config/split-keyboard/layers/` (`$XDG_CONFIG_HOME/split-keyboard/layers/` if set). Without layer files both halves start with an empty layout, so copy the examples first:

```bash
mkdir -p ~/.config/split-keyboard/layers

# From a clone
cp layers/*.toml ~/.config/split-keyboard/layers/

# From the AUR package
cp /usr/share/split-keyboard/layers/*.toml ~/.config/split-keyboard/layers/
```

`config.toml` is optional; built-in defaults are used when it is missing. See [Configuration](#configuration).

### Setting up uinput permissions

The virtual keyboard uses Linux uinput to send keystrokes, so your user needs read and write access to `/dev/uinput`. Without it the keyboard still appears, but prints `Failed to initialize virtual keyboard` and keys are not sent. You need to configure permissions:

**Option 1: Temporary fix (until reboot)**
```bash
sudo chmod 660 /dev/uinput
sudo chown root:input /dev/uinput
```

**Option 2: Permanent fix (recommended)**
```bash
# Create udev rule
echo 'KERNEL=="uinput", MODE="0660", GROUP="input", OPTIONS+="static_node=uinput"' | sudo tee /etc/udev/rules.d/99-uinput.rules

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

**Add yourself to the input group** (required for both options):
```bash
sudo usermod -a -G input $USER
```

**Note:** After adding yourself to the input group, you must log out and log back in for the change to take effect.

**Verify permissions:**
```bash
# Check if you're in the input group
groups

# Check uinput device permissions
ls -l /dev/uinput
```

The device should show: `crw-rw---- 1 root input ...`

## Usage

The keyboard runs as two separate processes - one for the left side and one for the right side.

### Basic Usage

```bash
# Start left keyboard
split-keyboard left &

# Start right keyboard
split-keyboard right &
```

### Hyprland Integration

Add to your `~/.config/hypr/hyprland.conf`:

```conf
# Keybind to toggle keyboards
bind = SUPER, K, exec, pkill split-keyboard || (split-keyboard left & split-keyboard right &)

# Window rules (optional, if not using layer-shell)
windowrule = float, ^(split-keyboard)$
windowrule = pin, ^(split-keyboard)$
windowrule = nofocus, ^(split-keyboard)$
```

### Systemd Service (Optional)

Create `~/.config/systemd/user/split-keyboard-left.service`:

```ini
[Unit]
Description=Split Keyboard - Left Panel
After=graphical-session.target

[Service]
ExecStart=/usr/local/bin/split-keyboard left
Restart=on-failure

[Install]
WantedBy=graphical-session.target
```

Create `~/.config/systemd/user/split-keyboard-right.service`:

```ini
[Unit]
Description=Split Keyboard - Right Panel
After=graphical-session.target

[Service]
ExecStart=/usr/local/bin/split-keyboard right
Restart=on-failure

[Install]
WantedBy=graphical-session.target
```

Enable and start:

```bash
systemctl --user enable split-keyboard-left.service split-keyboard-right.service
systemctl --user start split-keyboard-left.service split-keyboard-right.service
```

## Configuration

The keyboard is fully configurable through TOML files. The main configuration file defines appearance and behavior, while layer files define the keyboard layouts.

### Main Configuration

Create a config file at `~/.config/split-keyboard/config.toml`:

```toml
[window]
# Percentage of screen width for each keyboard half
width_percent = 25
# Keyboard layer (overlay, top, bottom, background)
layer = "overlay"

[appearance]
# Background color (RGBA)
bg_color = [30, 30, 30, 230]
# Key background color
key_color = [60, 60, 60, 255]
# Key pressed color
key_pressed_color = [100, 100, 100, 255]
# Text color
text_color = [255, 255, 255, 255]
# Key border color
border_color = [80, 80, 80, 255]
# Key padding in pixels
key_padding = 4
# Key border width
border_width = 1
# Font size for key labels
font_size = 18
# Vertical spacing multiplier (values > 1.0 add extra space between rows)
vertical_spacing_multiplier = 1.25
# Percentage of keyboard height to use as spacing when using split layout style
split_spacing_percent = 10
# Touch animation settings
enable_animations = true
animation_duration_ms = 300
ripple_color = [255, 255, 255, 100]
scale_amount = 0.05

[keyboard]
# Default layout: qwerty
default_layout = "qwerty"
# Number of rows (not enforced, each layer can have its own row count)
rows = 4
# Enable haptic feedback (requires additional setup)
haptic_feedback = false
# Layout style: standard (no gaps) or split (horizontal gap between rows)
layout_style = "split"
# Global split point: split the keyboard after this row index (0-based)
# Can be overridden per-layer in the layer config files
split_after_row = 3
```

### Layer Configuration

Layers are defined in separate TOML files in `~/.config/split-keyboard/layers/`. Each layer file defines:
- The number of rows (implicitly by the number of row sections)
- The keys in each row
- An optional per-layer split point (overrides global setting)

**Layer file naming convention**: `{layer_name}_{side}.toml`
- Examples: `qwerty_left.toml`, `symbols_right.toml`, `navigation_left.toml`

**Example layer file** (`~/.config/split-keyboard/layers/qwerty_left.toml`):

```toml
[layer]
name = "QWERTY Left"
# Optional: override global split_after_row for this layer
split_after_row = 3

# Row 1: Numbers 1-3
[[row]]
keys = [
  { label = "1", keycode = 2 },
  { label = "2", keycode = 3 },
  { label = "3", keycode = 4 }
]

# Row 2: QWE
[[row]]
keys = [
  { label = "q", keycode = 16 },
  { label = "w", keycode = 17 },
  { label = "e", keycode = 18 }
]

# Row 3: Layer switch example
[[row]]
keys = [
  { label = "v", keycode = 47 },
  { label = "Sym", layer_switch = { layer = "Symbols", activation = "Toggle" } },
  { label = "Nav", layer_switch = { layer = "Navigation", activation = "Hold" } }
]
```

**Key properties:**
- `label`: Text displayed on the key
- `keycode`: Linux input event keycode (optional, omit for layer switch keys)
- `width`: Relative width of the key (default: 1.0, can use 2.0 for wider keys)
- `layer_switch`: Define layer switching behavior
  - `layer`: Target layer name ("Base", "Symbols", "Navigation", "Numbers")
  - `activation`: "Toggle" (press to activate/deactivate) or "Hold" (active while held)

The project includes example layer files in the `layers/` directory that you can copy to `~/.config/split-keyboard/layers/` and customize.

## Architecture

### Components

- **layer_window.rs**: Manages wlr-layer-shell windows for overlay positioning
- **keyboard_ui.rs**: Handles keyboard rendering and visual feedback
- **layouts/**: Keyboard layout definitions (QWERTY, symbols, etc.)
- **virtual_keyboard.rs**: Linux uinput virtual keyboard implementation
- **input_handler.rs**: Touch and pointer input handling
- **config.rs**: Configuration file parsing

### Keyboard Layouts

#### Base Layer (QWERTY)

**Left Side**:
- Numbers 1-5
- Letters: q-w-e-r-t, a-s-d-f-g, z-x-c-v
- Modifiers: Ctrl, Alt, Shift
- Layer switches: "Sym" (toggle), "Nav" (hold)

**Right Side**:
- Numbers 6-0
- Letters: y-u-i-o-p, h-j-k-l, b-n-m
- Punctuation: comma, period
- Special keys: Tab, Backspace, Enter
- Layer switch: "Num" (toggle)

#### Symbols Layer

Programming symbols, brackets, operators, and special characters

**Left Side**: !, @, #, $, %, &, [, {, (, <, ;, `, ~, \, |, -, _, ', ", ,
**Right Side**: *, ^, &, (, ), ], }, >, :, /, =, +, ?, -, .

#### Navigation Layer

Function keys, arrow keys, and navigation controls

**Left Side**: F1-F6, Home, End, PgUp, PgDn, Arrow keys, Insert
**Right Side**: F7-F12, Arrow keys, Home, End, PgUp, PgDn, Insert, Delete, Esc

#### Numbers Layer

Number pad layout with operators

**Both Sides**:
- Number pad layout (0-9)
- Math operators: +, -, *, /, =
- Decimal point

### Layer Switching

- **Toggle Mode**: Press layer key once to activate, press again to deactivate
- **Hold Mode**: Layer active only while holding the layer key
- **Visual Indicator**: Current layer name displayed at the top of the keyboard
- **Layer Keys**:
  - Left side: "Sym" (toggle), "Nav" (hold)
  - Right side: "Num" (toggle)
  - All layers: "Base" button to return to base layer

## Current Limitations

1. **Input method integration**: Currently sends raw key events via uinput. Full IME support for complex text input (e.g., CJK languages) is not yet implemented.

2. **Haptic feedback**: Visual feedback is implemented with animations. Haptic feedback would require additional system integration.

3. **Shift/Caps Lock**: Currently shift and modifiers work as regular keys. May need dedicated shift layer or better modifier handling in the future.

## Roadmap

- [x] Add text rendering with a font library
- [x] Add symbol/number layer switching
- [x] Touch animations and visual feedback
- [x] Configurable custom layers via TOML files
- [ ] Implement IME protocol for better text input
- [ ] Support for more keyboard layouts (Dvorak, Colemak, international)
- [ ] Word prediction and autocorrect
- [ ] Swipe typing gestures
- [ ] Theming support
- [ ] Per-application keyboard profiles
- [ ] Better shift/modifier handling

## Troubleshooting

### Keyboard doesn't appear

1. Check if wlr-layer-shell is supported:
   ```bash
   wayland-info | grep layer_shell
   ```

2. Check compositor logs for errors

3. Try changing the `layer` setting in config.toml from "overlay" to "top"

### Keys don't send input

If you see "Failed to initialize virtual keyboard: Permission denied (os error 13)":

1. **Check uinput permissions:**
   ```bash
   ls -l /dev/uinput
   ```
   Should show: `crw-rw---- 1 root input ...`

2. **Fix permissions if needed:**
   ```bash
   sudo chmod 660 /dev/uinput
   sudo chown root:input /dev/uinput
   ```

3. **Verify you're in the input group:**
   ```bash
   groups
   ```
   If `input` is not listed, run:
   ```bash
   sudo usermod -a -G input $USER
   ```
   Then log out and log back in.

4. **For a permanent fix, see the "Setting up uinput permissions" section above.**

### Wrong positioning

1. Adjust `width_percent` in config.toml
2. Check if Hyprland window rules are conflicting
3. Verify layer-shell anchoring is working correctly

## Contributing

Contributions are welcome! Areas that need work:

- Text rendering implementation
- Additional keyboard layouts
- Better touch gesture support
- Accessibility features
- Testing on different compositors

## License

MIT License - see LICENSE file for details

## Credits

Built with:
- [smithay-client-toolkit](https://github.com/Smithay/client-toolkit) - Wayland client library
- [softbuffer](https://github.com/rust-windowing/softbuffer) - Software rendering
- [wayland-rs](https://github.com/Smithay/wayland-rs) - Wayland protocol bindings
- [evdev](https://github.com/emberian/evdev) - Linux input device interface
