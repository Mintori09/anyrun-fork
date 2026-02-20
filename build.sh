#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

PLUGIN_DIR="$HOME/.config/anyrun/plugins"
BINARY_DEST="/usr/bin/anyrun"

main() {
    echo "--- Building in release mode ---"
    cargo build --release

    echo "--- Preparing plugin directory ---"
    mkdir -p "$PLUGIN_DIR"

    echo "--- Installing plugins and binary ---"
    # cp target/release/*.so "$PLUGIN_DIR"

    sudo install -Dm755 ./target/release/anyrun "$BINARY_DEST"

    echo "Done! anyrun has been updated."
}

main "$@"
