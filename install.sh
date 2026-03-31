#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="agents-soul"
DEST="${DEST:-$HOME/.local/bin}"
VERIFY=0

log() {
  echo "[$BINARY_NAME] $*" >&2
}

usage() {
  cat <<'EOF'
Install agents-soul from the current source checkout and auto-register its MCP server.

Usage:
  ./install.sh [options]

Options:
  --from-source       Accepted for compatibility; this installer always builds from source
  --dest PATH         Install bin directory (default: ~/.local/bin)
  --dest=PATH         Install bin directory
  --system            Install into /usr/local/bin
  --verify            Run a post-install MCP smoke check against the installed binary
  -h, --help          Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --from-source)
      shift
      ;;
    --dest)
      DEST="$2"
      shift 2
      ;;
    --dest=*)
      DEST="${1#*=}"
      shift
      ;;
    --system)
      DEST="/usr/local/bin"
      shift
      ;;
    --verify)
      VERIFY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 64
      ;;
  esac
done

if [[ "$DEST" == */bin ]]; then
  INSTALL_ROOT="${DEST%/bin}"
else
  INSTALL_ROOT="$DEST"
  DEST="$DEST/bin"
fi

mkdir -p "$DEST"

log "Building and installing from source into $DEST"
cargo install --path . --locked --force --root "$INSTALL_ROOT"

BINARY_PATH="$DEST/$BINARY_NAME"
if [[ ! -x "$BINARY_PATH" ]]; then
  echo "ERROR: installed binary missing at $BINARY_PATH" >&2
  exit 1
fi

write_json_config() {
  local path="$1"
  local servers_key="$2"
  local command_path="$3"

  mkdir -p "$(dirname "$path")"
  python3 - "$path" "$servers_key" "$command_path" <<'PY'
import json
import os
import sys

path, servers_key, command_path = sys.argv[1:4]
entry = {"command": command_path, "args": ["mcp"]}
if os.path.exists(path):
    with open(path, "r", encoding="utf-8") as fh:
        data = json.load(fh)
else:
    data = {}

if not isinstance(data, dict):
    raise SystemExit(f"{path} root is not a JSON object")

cursor = data
parts = servers_key.split(".")
for part in parts[:-1]:
    child = cursor.get(part)
    if not isinstance(child, dict):
        child = {}
        cursor[part] = child
    cursor = child

leaf = parts[-1]
servers = cursor.get(leaf)
if not isinstance(servers, dict):
    servers = {}

existing = servers.get("agents-soul")
if existing is None:
    status = "installed"
elif existing == entry:
    status = "unchanged"
else:
    status = "updated"

servers["agents-soul"] = entry
cursor[leaf] = servers

with open(path, "w", encoding="utf-8") as fh:
    json.dump(data, fh, indent=2)
    fh.write("\n")

print(status)
PY
}

write_toml_config() {
  local path="$1"
  local command_path="$2"

  mkdir -p "$(dirname "$path")"
  python3 - "$path" "$command_path" <<'PY'
import os
import re
import sys

path, command_path = sys.argv[1:3]
section = '[mcp_servers.agents-soul]\ncommand = "{0}"\nargs = ["mcp"]\n'.format(
    command_path.replace("\\", "\\\\").replace('"', '\\"')
)

if os.path.exists(path):
    with open(path, "r", encoding="utf-8") as fh:
        text = fh.read()
else:
    text = ""

pattern = re.compile(r'(?ms)^\[mcp_servers\.agents-soul\]\n.*?(?=^\[|\Z)')
match = pattern.search(text)
if match is None:
    status = "installed"
    if text and not text.endswith("\n"):
        text += "\n"
    if text:
        text += "\n"
    text += section
else:
    current = match.group(0).strip()
    replacement = section.rstrip()
    status = "unchanged" if current == replacement else "updated"
    text = text[:match.start()] + section + text[match.end():]

with open(path, "w", encoding="utf-8") as fh:
    fh.write(text)

print(status)
PY
}

install_host() {
  local host="$1"
  local path
  local status
  local note

  case "$host" in
    claude-code)
      path="$HOME/.claude.json"
      status="$(write_json_config "$path" "mcpServers" "$BINARY_PATH")"
      note="User scope."
      ;;
    codex)
      path="$HOME/.codex/config.toml"
      status="$(write_toml_config "$path" "$BINARY_PATH")"
      note="User scope."
      ;;
    cursor)
      path="$HOME/.cursor/mcp.json"
      status="$(write_json_config "$path" "mcpServers" "$BINARY_PATH")"
      note="Global scope."
      ;;
    windsurf)
      path="$HOME/.codeium/windsurf/mcp_config.json"
      status="$(write_json_config "$path" "mcpServers" "$BINARY_PATH")"
      note="Global scope."
      ;;
    vscode)
      path="$(pwd)/.vscode/mcp.json"
      status="$(write_json_config "$path" "servers" "$BINARY_PATH")"
      note="Project scope."
      ;;
    gemini)
      path="$HOME/.gemini/settings.json"
      status="$(write_json_config "$path" "mcpServers" "$BINARY_PATH")"
      note="User scope."
      ;;
    opencode)
      path="$HOME/.opencode.json"
      status="$(write_json_config "$path" "mcpServers" "$BINARY_PATH")"
      note="User scope."
      ;;
    amp)
      path="$HOME/.config/amp/settings.json"
      status="$(write_json_config "$path" "amp.mcpServers" "$BINARY_PATH")"
      note="User scope."
      ;;
    droid)
      path="$HOME/.factory/mcp.json"
      status="$(write_json_config "$path" "mcpServers" "$BINARY_PATH")"
      note="User scope."
      ;;
    *)
      return 1
      ;;
  esac

  echo "- $host: $status ($path)"
  echo "  $note"
}

detect_hosts() {
  local hosts=()
  [[ -d "$HOME/.codex" ]] && hosts+=("codex")
  [[ -e "$HOME/.claude.json" ]] && hosts+=("claude-code")
  [[ -d "$HOME/.cursor" ]] && hosts+=("cursor")
  [[ -d "$HOME/.codeium/windsurf" ]] && hosts+=("windsurf")
  [[ -d "$(pwd)/.vscode" ]] && hosts+=("vscode")
  [[ -d "$HOME/.gemini" ]] && hosts+=("gemini")
  [[ -e "$HOME/.opencode.json" ]] && hosts+=("opencode")
  [[ -d "$HOME/.config/amp" ]] && hosts+=("amp")
  [[ -d "$HOME/.factory" ]] && hosts+=("droid")
  if [[ "${#hosts[@]}" -gt 0 ]]; then
    printf '%s\n' "${hosts[@]}"
  fi
}

mapfile -t HOSTS < <(detect_hosts)
if [[ "${#HOSTS[@]}" -gt 0 ]]; then
  log "Auto-installing MCP provider configs..."
  echo "$BINARY_NAME MCP auto-install results:"
  for host in "${HOSTS[@]}"; do
    install_host "$host"
  done
else
  log "No supported MCP providers detected. Skipping MCP config install."
fi

if [[ "$VERIFY" -eq 1 ]]; then
  "$PWD/scripts/run-mcp-stdio-smoke.sh" --bin "$BINARY_PATH"
fi

echo "✓ $BINARY_NAME installed → $BINARY_PATH"
