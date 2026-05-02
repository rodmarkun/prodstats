#!/usr/bin/env bash
set -euo pipefail

REPO_URL="${PRODSTATS_REPO_URL:-https://github.com/rodmarkun/prodstats.git}"
BRANCH="${PRODSTATS_BRANCH:-main}"
INSTALL_DIR="${PRODSTATS_INSTALL_DIR:-$HOME/.local/src/prodstats}"
CORNER="${PRODSTATS_CORNER:-top-right}"
INPUT_ACCESS=0
INSTALL_ALL=1

usage() {
  cat <<'USAGE'
Install prodstats from source.

Usage:
  install.sh [--corner top-right|top-left|bottom-right|bottom-left] [--input-access] [--no-install-all]

Environment overrides:
  PRODSTATS_REPO_URL       Git URL to clone (default: https://github.com/rodmarkun/prodstats.git)
  PRODSTATS_BRANCH         Branch/tag to install (default: main)
  PRODSTATS_INSTALL_DIR    Clone/update directory (default: ~/.local/src/prodstats)
  PRODSTATS_CORNER         Display corner (default: top-right)

Examples:
  curl -fsSL https://raw.githubusercontent.com/rodmarkun/prodstats/main/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/rodmarkun/prodstats/main/install.sh | bash -s -- --corner bottom-right --input-access
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --corner)
      CORNER="${2:?--corner requires a value}"
      shift 2
      ;;
    --input-access)
      INPUT_ACCESS=1
      shift
      ;;
    --no-install-all)
      INSTALL_ALL=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "prodstats currently supports Linux only." >&2
  exit 1
fi

need_cmd() {
  command -v "$1" >/dev/null 2>&1
}

if ! need_cmd git; then
  echo "Missing dependency: git" >&2
  echo "Install git with your package manager, then re-run this installer." >&2
  exit 1
fi

if ! need_cmd cargo; then
  if need_cmd rustup; then
    rustup default stable
  else
    echo "Rust/Cargo not found; installing rustup with the official non-interactive installer."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
  fi
fi

mkdir -p "$(dirname "$INSTALL_DIR")"
if [[ -d "$INSTALL_DIR/.git" ]]; then
  git -C "$INSTALL_DIR" fetch --depth 1 origin "$BRANCH"
  git -C "$INSTALL_DIR" checkout -B "$BRANCH" "origin/$BRANCH"
else
  rm -rf "$INSTALL_DIR"
  git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$INSTALL_DIR"
fi

cargo install --path "$INSTALL_DIR"

if [[ "$INSTALL_ALL" -eq 1 ]]; then
  "$HOME/.cargo/bin/prodstats" install all --corner "$CORNER"
fi

if [[ "$INPUT_ACCESS" -eq 1 ]]; then
  "$HOME/.cargo/bin/prodstats" install input-access
  systemctl --user restart prodstats.service 2>/dev/null || true
fi

cat <<EOF

prodstats installed.

Next checks:
  prodstats doctor
  prodstats status

If APM is 0 because input access is missing, run:
  prodstats install input-access
  systemctl --user restart prodstats.service

For permanent input permissions, log out and back in after input-access adds you to the input group.
EOF
