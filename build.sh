#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

PLUGIN_DIR="$HOME/.config/anyrun/plugins"
BINARY_DEST="/usr/bin/anyrun"
PROVIDER_DEST="/usr/bin/anyrun-provider"

main() {
    echo "--- Building in release mode ---"
    cargo build --release

    echo "--- Preparing plugin directory ---"
    mkdir -p "$PLUGIN_DIR"

    echo "--- Installing binaries ---"
    cp ./target/release/*.so "$PLUGIN_DIR"
    sudo install -Dm755 ./target/release/anyrun "$BINARY_DEST"
    sudo install -Dm755 ./target/release/anyrun-provider "$PROVIDER_DEST"

    echo "Done! anyrun and anyrun-provider have been updated."
}

main "$@"
