# Agent installation instructions

Use this file when an AI agent, setup script, or human helper needs to install prodstats with minimal ambiguity.

## One-line install

```bash
curl -fsSL https://raw.githubusercontent.com/rodmarkun/prodstats/main/install.sh | bash
```

## One-line install with APM input access

This may prompt for administrator approval because global input counting on Wayland requires `/dev/input/event*` access.

```bash
curl -fsSL https://raw.githubusercontent.com/rodmarkun/prodstats/main/install.sh | bash -s -- --input-access
```

## One-line install with custom corner

```bash
curl -fsSL https://raw.githubusercontent.com/rodmarkun/prodstats/main/install.sh | bash -s -- --corner bottom-right
```

Valid corners are usually:

- `top-right`
- `top-left`
- `bottom-right`
- `bottom-left`

## What the installer does

1. Checks it is running on Linux.
2. Ensures Git and Cargo/Rust are available.
3. Installs Rust with rustup if Cargo is missing.
4. Clones or updates `https://github.com/rodmarkun/prodstats.git` into `~/.local/src/prodstats`.
5. Runs `cargo install --path ~/.local/src/prodstats`.
6. Runs `prodstats install all --corner <corner>` unless `--no-install-all` is passed.
7. If `--input-access` is passed, runs `prodstats install input-access` and restarts the user service.

## Verification

After installation, run:

```bash
prodstats doctor
prodstats status
prodstats waybar
```

If `prodstats doctor` reports missing input access, run:

```bash
prodstats install input-access
systemctl --user restart prodstats.service
```

After input-access, log out and back in when convenient so input group membership is present in future sessions.
