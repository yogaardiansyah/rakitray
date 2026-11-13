#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RAKIT_BIN="$REPO_ROOT/target/debug/rakit"

if [ ! -x "$RAKIT_BIN" ]; then
  cargo build --manifest-path "$REPO_ROOT/Cargo.toml" -p rakit-cli
fi

SYS_PHP_VER="$(php -v | head -n1)"
echo "[e2e] System PHP before: $SYS_PHP_VER"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

cat > "$TMP_DIR/.rakitray.toml" <<'EOF'
schema_version = 1
name = "e2e-sample-app"

[services.php]
version = "8.3.9"
port = 9000

[services.php.config]
memory_limit = "512M"

[services.node]
version = "22.11.0"
EOF

echo "[e2e] Testing isolated subshell with eval \"\$(rakit env)\"..."
(
  cd "$TMP_DIR"
  export RAKITRAY_RUNTIME_DIR="$REPO_ROOT/runtime"
  eval "$("$RAKIT_BIN" env)"

  ACTIVE_PHP="$(php -v | head -n1)"
  ACTIVE_NODE="$(node -v)"
  echo "[e2e] Active PHP inside Rakitray env:  $ACTIVE_PHP"
  echo "[e2e] Active Node inside Rakitray env: $ACTIVE_NODE"

  echo "$ACTIVE_PHP" | grep -q "8.3.9"
  echo "$ACTIVE_NODE" | grep -q "v22.11.0"

  if command -v composer >/dev/null 2>&1; then
    COMPOSER_VER="$(composer --version | head -n1)"
    echo "[e2e] Composer inside Rakitray env:    $COMPOSER_VER"
  fi
)

AFTER_PHP_VER="$(php -v | head -n1)"
echo "[e2e] System PHP after exit: $AFTER_PHP_VER"
[ "$SYS_PHP_VER" = "$AFTER_PHP_VER" ]

echo "[e2e] PASS: Rakitray env switches php/node cleanly and restores system default on exit."

