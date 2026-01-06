#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

main() {
    cargo build --release
    ./target/release/anyrun
}

main "$@"
