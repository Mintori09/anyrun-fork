#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

main() {
    cargo build --release
    cp target/release/*.so ~/.config/anyrun/plugins
}

main "$@"
