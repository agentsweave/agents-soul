#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/run-mcp-stdio-smoke.sh [--bin /path/to/agents-soul]

Runs a real stdio MCP smoke test against the provided agents-soul binary.
If --bin is omitted, the script resolves `agents-soul` from PATH.
EOF
}

BIN="agents-soul"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin)
      [[ $# -ge 2 ]] || {
        echo "--bin requires a value" >&2
        exit 2
      }
      BIN="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
WORKSPACE=$(mktemp -d)
OUTPUT="$WORKSPACE/mcp.out"
cleanup() {
  rm -rf "$WORKSPACE"
}
trap cleanup EXIT

cp "$REPO_ROOT/examples/workspaces/healthy/soul.toml" "$WORKSPACE/soul.toml"
cp "$REPO_ROOT/tests/fixtures/compose_modes/identity_healthy.json" "$WORKSPACE/identity.json"
cp "$REPO_ROOT/tests/fixtures/compose_modes/verification_active.json" "$WORKSPACE/verification.json"

send() {
  local body="$1"
  local len
  len=$(printf '%s' "$body" | wc -c | tr -d ' ')
  printf 'Content-Length: %s\r\n\r\n%s' "$len" "$body"
}

{
  send '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","clientInfo":{"name":"agents-soul-smoke","version":"1.0.0"},"capabilities":{}}}'
  send '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
  send '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
  send "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"compose_context\",\"arguments\":{\"workspace_id\":\"$WORKSPACE\",\"agent_id\":\"agent.alpha\",\"session_id\":\"mcp-stdio-smoke\",\"identity_snapshot_path\":\"$WORKSPACE/identity.json\",\"registry_verification_path\":\"$WORKSPACE/verification.json\"}}}"
} | "$BIN" mcp > "$OUTPUT"

grep -q '"protocolVersion":"2025-03-26"' "$OUTPUT"
grep -q '"name":"compose_context"' "$OUTPUT"
grep -q 'Alpha Builder' "$OUTPUT"

echo "agents-soul MCP stdio smoke passed"
