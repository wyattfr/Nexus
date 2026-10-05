#!/usr/bin/env sh
set -eu

echo "Installing NEXUS CLI..."

if ! command -v cargo >/dev/null 2>&1; then
  echo "Cargo was not found. Install Rust from https://rustup.rs/ and rerun this script." >&2
  exit 1
fi

cargo install --path crates/nexus-cli --force

CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
NEXUS_BIN="$CARGO_BIN/nexus"
if [ ! -x "$NEXUS_BIN" ]; then
  echo "Installation completed but nexus was not found at $NEXUS_BIN" >&2
  exit 1
fi

case ":${PATH}:" in
  *":${CARGO_BIN}:"*) ;;
  *)
    PROFILE="$HOME/.profile"
    if [ -n "${ZSH_VERSION:-}" ]; then
      PROFILE="$HOME/.zprofile"
    fi
    LINE="export PATH=\"\$PATH:$CARGO_BIN\""
    if [ ! -f "$PROFILE" ] || ! grep -Fqx "$LINE" "$PROFILE"; then
      printf '\n%s\n' "$LINE" >> "$PROFILE"
    fi
    export PATH="$CARGO_BIN:$PATH"
    ;;
esac

echo "Installed: $NEXUS_BIN"
"$NEXUS_BIN" --version
"$NEXUS_BIN" --help
echo "If another terminal does not find nexus, close and reopen it or source your shell profile."
