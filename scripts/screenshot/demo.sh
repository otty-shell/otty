#!/usr/bin/env bash
# Start OTTY in a clean demo home for the README screenshot.
#
# The demo home lives in its own folder, so your real settings, launches,
# shell history and processes never get into the picture. Helix and btop
# use themes built from the 16 terminal colors, so they follow the OTTY
# palette instead of bringing their own.
#
# Usage:
#   scripts/screenshot/demo.sh               build the scene and start OTTY
#   scripts/screenshot/demo.sh --containers  also start three demo containers
#   scripts/screenshot/demo.sh --clean       remove the demo folder and containers
#
# DEMO_DIR (default: ~/.cache/otty-demo) is where the scene is built. It is
# kept out of /tmp, which is often held in memory.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEMO_DIR="${DEMO_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/otty-demo}"
DEMO_HOME="$DEMO_DIR/home"
DEMO_PROJECT="$DEMO_HOME/otty"
MARKER="$DEMO_DIR/.otty-demo"
CONTAINERS=(demo-web demo-cache demo-db)

REAL_CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
REAL_RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"

main() {
  case "${1:-}" in
    --clean)
      clean
      ;;
    --containers)
      build_scene
      start_containers
      start_otty
      ;;
    "")
      build_scene
      start_otty
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 2
      ;;
  esac
}

clean() {
  stop_containers

  if [[ -d "$DEMO_DIR" && ! -f "$MARKER" ]]; then
    echo "$DEMO_DIR was not made by this script, leaving it alone." >&2
    exit 1
  fi

  rm -rf "$DEMO_DIR"
  echo "Removed $DEMO_DIR"
}

build_scene() {
  if [[ -d "$DEMO_DIR" && ! -f "$MARKER" ]]; then
    echo "$DEMO_DIR exists and was not made by this script." >&2
    echo "Set DEMO_DIR to another folder." >&2
    exit 1
  fi

  rm -rf "$DEMO_DIR"
  mkdir -p "$DEMO_HOME"
  touch "$MARKER"

  clone_project
  write_otty_config
  write_tool_config
  write_shell_config
  prebuild_tests
}

# A clean clone of the committed tree: no target/, no local tool folders,
# so the Explorer shows only the project itself.
clone_project() {
  git clone --quiet --no-hardlinks "$REPO" "$DEMO_PROJECT"
  git -C "$DEMO_PROJECT" remote remove origin
}

write_otty_config() {
  local dir="$DEMO_HOME/.config/otty"
  mkdir -p "$dir"

  cat >"$dir/settings.json" <<'JSON'
{
  "general": { "language": "en" },
  "terminal": { "shell": "/usr/bin/zsh", "editor": "hx", "equalize_panes": true },
  "theme": { "name": "otty-dark" }
}
JSON

  cat >"$dir/modes.json" <<'JSON'
{ "pinned": ["docker"] }
JSON

  cat >"$dir/quick_launches.json" <<JSON
{
  "version": 1,
  "root": {
    "title": "Quick Commands",
    "expanded": true,
    "children": [
      {
        "kind": "Folder",
        "title": "Servers",
        "expanded": true,
        "children": [
          $(ssh_launch "api-prod" "api-prod.example.com" "deploy"),
          $(ssh_launch "db-replica" "db-replica.example.com" "postgres"),
          $(ssh_launch "staging" "staging.example.com" "deploy")
        ]
      },
      {
        "kind": "Folder",
        "title": "Project",
        "expanded": true,
        "children": [
          $(local_launch "Run tests" "cargo" '"test", "--workspace"'),
          $(local_launch "Watch logs" "tail" '"-f", "/var/log/messages"'),
          $(local_launch "Open editor" "hx" '"."')
        ]
      },
      {
        "kind": "Folder",
        "title": "Tools",
        "expanded": false,
        "children": [
          $(local_launch "btop" "btop" '')
        ]
      }
    ]
  }
}
JSON
}

ssh_launch() {
  local title=$1 host=$2 user=$3
  cat <<JSON
{
            "kind": "Command",
            "title": "$title",
            "spec": { "type": "Ssh", "ssh": {
              "host": "$host", "port": 22, "user": "$user",
              "identity_file": null, "extra_args": []
            } },
            "exit_behavior": "close"
          }
JSON
}

local_launch() {
  local title=$1 program=$2 args=$3
  cat <<JSON
{
            "kind": "Command",
            "title": "$title",
            "spec": { "type": "Custom", "custom": {
              "program": "$program", "args": [$args], "env": [],
              "working_directory": "$DEMO_PROJECT"
            } },
            "exit_behavior": "keep_open"
          }
JSON
}

# Helix and btop themes that draw with the terminal colors only, so they
# match whatever OTTY theme is active.
write_tool_config() {
  mkdir -p "$DEMO_HOME/.config/helix" "$DEMO_HOME/.config/btop"

  cat >"$DEMO_HOME/.config/helix/config.toml" <<'TOML'
theme = "base16_transparent"

[editor]
line-number = "absolute"
cursorline = true
bufferline = "never"

[editor.statusline]
left = ["mode", "file-name"]
right = ["position"]
TOML

  cat >"$DEMO_HOME/.config/btop/btop.conf" <<'CONF'
color_theme = "TTY"
theme_background = false
truecolor = true
rounded_corners = true
shown_boxes = "cpu mem proc"
proc_sorting = "cpu lazy"
update_ms = 2000
CONF
}

write_shell_config() {
  cat >"$DEMO_HOME/.zshrc" <<ZSH
export CARGO_HOME="$REAL_CARGO_HOME"
export RUSTUP_HOME="$REAL_RUSTUP_HOME"
export CARGO_TARGET_DIR="$DEMO_DIR/target"
export PATH="\$CARGO_HOME/bin:\$PATH"
export EDITOR=hx

HISTFILE="\$HOME/.zsh_history"
HISTSIZE=100
SAVEHIST=100
setopt INC_APPEND_HISTORY

autoload -Uz colors && colors
PROMPT='%F{cyan}%1~%f %F{magenta}❯%f '

alias ls='ls --color=auto'
cd "$DEMO_PROJECT"
ZSH

  # Arrow up gives the commands of the shot list in order.
  cat >"$DEMO_HOME/.zsh_history" <<'HIST'
git push
docker ps
cargo test -p otty-escape --quiet
git log --oneline --graph --decorate -8
HIST
}

# Build the test binary now, so the test block on camera finishes at once.
prebuild_tests() {
  echo "Building the demo tests once..."
  CARGO_TARGET_DIR="$DEMO_DIR/target" nice -n 10 \
    cargo test --quiet --manifest-path "$DEMO_PROJECT/Cargo.toml" \
    -p otty-escape --no-run
}

start_containers() {
  docker run -d --name demo-web -p 8080:80 nginx:alpine >/dev/null
  docker run -d --name demo-cache redis:alpine >/dev/null
  docker run -d --name demo-db -e POSTGRES_PASSWORD=demo postgres:16-alpine >/dev/null
  echo "Started ${CONTAINERS[*]}"
}

stop_containers() {
  if ! command -v docker >/dev/null 2>&1; then
    return
  fi

  docker rm -f "${CONTAINERS[@]}" >/dev/null 2>&1 || true
}

start_otty() {
  local otty="$REPO/target/release/otty"

  if [[ ! -x "$otty" ]]; then
    echo "Building OTTY..."
    nice -n 10 cargo build --release --manifest-path "$REPO/Cargo.toml" -p otty
  fi

  echo "Starting OTTY in $DEMO_HOME"
  echo "When done: scripts/screenshot/demo.sh --clean"

  env -u ZDOTDIR -u XDG_CACHE_HOME -u XDG_DATA_HOME -u XDG_STATE_HOME \
    HOME="$DEMO_HOME" \
    XDG_CONFIG_HOME="$DEMO_HOME/.config" \
    SHELL=/usr/bin/zsh \
    "$otty"
}

main "$@"
