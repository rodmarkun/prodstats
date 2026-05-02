# Omarchy integration

Omarchy uses Hyprland and Waybar, so prodstats targets Waybar first.

## Install

```bash
cd ~/Projects/prodstats
cargo install --path .
prodstats install all --corner top-right
```

This does four things:

1. Writes `~/.config/prodstats/config.toml` if missing.
2. Installs and starts `~/.config/systemd/user/prodstats.service`.
3. Adds shell git-push tracking to `.zshrc` or `.bashrc`.
4. Patches `~/.config/waybar/config.jsonc` and `~/.config/waybar/style.css`.

The Waybar config is backed up before patching:

```text
~/.config/waybar/config.jsonc.prodstats.bak
```

## Corners

Waybar is a bar, not a free floating surface. These corners map to Waybar module groups:

- `top-left` -> `modules-left`
- `top-center` -> `modules-center`
- `top-right` -> `modules-right`

For a true screen-corner overlay, run:

```bash
prodstats overlay --corner bottom-right
~/.local/share/prodstats/overlay-bottom-right.py
```

That experimental overlay requires GTK4 layer shell packages. On Arch-like systems the needed packages are typically:

```bash
sudo pacman -S gtk4 python-gobject gtk4-layer-shell
```

Package names can vary.

## Input permissions

Wayland blocks global keystroke access for normal clients. Prodstats uses evdev and only stores counts.

On Omarchy/Arch, the practical setup is:

```bash
sudo usermod -aG input "$USER"
```

Then log out and back in.

Check with:

```bash
prodstats doctor
```

## Waybar manual snippet

If the automatic patch fails, add this module to your Waybar config:

```jsonc
"custom/prodstats": {
  "exec": "prodstats waybar",
  "return-type": "json",
  "interval": 2,
  "tooltip": true,
  "on-click": "xdg-terminal-exec prodstats status --watch",
  "on-click-right": "xdg-terminal-exec prodstats doctor"
}
```

Then add `"custom/prodstats"` to one of `modules-left`, `modules-center`, or `modules-right`.
