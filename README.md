# prodstats

A tiny, configurable Linux productivity stats daemon and corner display.

Built first for Omarchy/Hyprland/Waybar, but the collector is compositor-neutral and stores local CSV history.

## Stats

- APM: StarCraft-style average actions per active-input minute. Default actions are keyboard presses, mouse button presses, and wheel events. Held-key repeats and mouse movement are disabled by default.
- Active agent harnesses: Hermes, Claude Code, Codex, OpenCode, Aider, Goose, Gemini, Amp by default. Hermes and Claude are the primary tuned targets.
- Git pushes today: successful shell `git push` commands logged through shell integration.
- Total actions today.
- Agent active time today: summed across concurrent active harnesses.

Example compact display:

```text
󰘳 42  󰚩 2  󰊢 3  󰐌 1.8k  󱎫 1h 11m
```

## Quick install

Copy this one line on Linux/Omarchy:

```bash
curl -fsSL https://raw.githubusercontent.com/rodmarkun/prodstats/main/install.sh | bash
```

With immediate input-device access for APM/action counting:

```bash
curl -fsSL https://raw.githubusercontent.com/rodmarkun/prodstats/main/install.sh | bash -s -- --input-access
```

Choose another display corner:

```bash
curl -fsSL https://raw.githubusercontent.com/rodmarkun/prodstats/main/install.sh | bash -s -- --corner bottom-right
```

The installer clones/updates the repo under `~/.local/src/prodstats`, installs the Rust binary with Cargo, and runs `prodstats install all --corner <corner>`.

## Agent-friendly install prompt

If you are asking an AI coding/desktop agent to install prodstats for you, paste this:

```text
Install prodstats on this Linux machine by running:
curl -fsSL https://raw.githubusercontent.com/rodmarkun/prodstats/main/install.sh | bash -s -- --input-access
Then run `prodstats doctor` and fix any reported issue.
```

## Install from a local checkout

```bash
cd ~/Projects/prodstats
cargo install --path .
prodstats install all --corner top-right
prodstats doctor
```

For input counting on Wayland/Omarchy you normally need access to `/dev/input/event*`:

```bash
prodstats install input-access
systemctl --user restart prodstats.service
# log out and back in later so input group membership is present in new sessions
```

Equivalent manual commands:

```bash
sudo usermod -aG input "$USER"
sudo setfacl -m u:$USER:rw /dev/input/event*
```

Then check:

```bash
systemctl --user status prodstats.service
prodstats status
prodstats waybar
```

## Commands

```bash
prodstats daemon                         # run foreground daemon
prodstats status                         # compact current status
prodstats status --format json           # current state JSON
prodstats status --watch                 # update in terminal
prodstats waybar                         # one-line Waybar JSON
prodstats doctor                         # diagnostics
prodstats install service                # install user systemd service
prodstats install input-access           # grant evdev input access for APM/actions
prodstats install shell                  # install git push tracking function
prodstats install omarchy --corner top-right
prodstats install all --corner top-right
prodstats overlay --corner bottom-right  # writes experimental layer-shell overlay script
```

## Files

```text
~/.config/prodstats/config.toml
~/.local/share/prodstats/state.json
~/.local/share/prodstats/history/YYYY-MM.csv
~/.local/share/prodstats/git-pushes.csv
~/.local/share/prodstats/prodstatsd.lock
```

## Privacy

Prodstats does not store keys, characters, commands, window titles, or process prompts. It stores counts and agent process names. See `docs/PRIVACY.md`.

## Display frontends

Stable v1:

- Waybar custom module, including Omarchy installer.

Experimental v1:

- `prodstats overlay` writes a GTK4 layer-shell Python frontend. It requires `python-gobject`, `gtk4`, and `gtk4-layer-shell`.

Future:

- native Rust layer-shell frontend
- eww/ags snippets
- X11/polybar/conky support
