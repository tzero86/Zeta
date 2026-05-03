#!/usr/bin/env bash
# Wrapper for tui-test: each invocation gets a fresh, unique config path.
# This prevents test state (e.g. preview_panel_open) from leaking between tests.
# A minimal valid config is written so Zeta skips the first-run wizard.
ZETA_CONFIG="$(mktemp /tmp/zeta-test-XXXXXX.toml)"
export ZETA_CONFIG
# Clean up the temp config if exec fails (exec success replaces this shell, trap won't run then)
trap 'rm -f "$ZETA_CONFIG"' EXIT

cat > "$ZETA_CONFIG" <<'EOF'
[theme]
preset = "zeta"
status_bar_label = "Zeta"

[keymap]
quit = "q"
switch_pane = "tab"
refresh = "r"
EOF

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ZETA_BINARY="${ZETA_BIN:-${SCRIPT_DIR}/../target/debug/zeta}"
exec "$ZETA_BINARY" "$@"
