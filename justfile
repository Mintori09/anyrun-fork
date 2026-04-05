set shell := ["bash", "-c"]

core_pkgs := "-p anyrun -p anyrun-provider"

build target="all":
    @{{ if target == "all" {
        "cargo build --release --workspace"
    } else if target == "bin" {
        "cargo build --release " + core_pkgs
    } else if target == "plugins" {
        "just build-plugins"
    } else {
        "cargo build --release -p " + target
    } }}

build-plugins:
    @find plugins -maxdepth 2 -name "Cargo.toml" | while read -r toml; do \
        dir=$(dirname "$toml"); \
        echo "Building plugin in directory: $dir"; \
        cargo build --release --manifest-path "$toml"; \
    done

test target="all":
    @{{ if target == "all" {
        "cargo test --workspace"
    } else if target == "bin" {
        "cargo test " + core_pkgs
    } else if target == "plugins" {
        "just test-plugins"
    } else {
        "cargo test -p " + target
    } }}

# test-plugins:
#     @find plugins -maxdepth 2 -name "Cargo.toml" | while read -r toml; do \
#         echo "Testing plugin: $$(dirname $$toml)"; \
#         cargo test --manifest-path "$$toml"; \
#     done

run:
    just build all
    ./target/release/anyrun

daemon:
    just build all
    ./target/release/anyrun daemon

install:
    sudo cp ./target/release/anyrun /usr/bin
    sudo cp ./target/release/anyrun-provider /usr/bin

install-plugin:
    cp ./target/release/*.so ~/.config/anyrun/plugins

clean:
    cargo clean

check:
    cargo check --workspace

default:
    just build bin

