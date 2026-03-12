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
        echo "Building plugin: $$(dirname $$toml)"; \
        cargo build --release --manifest-path "$$toml"; \
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

test-plugins:
    @find plugins -maxdepth 2 -name "Cargo.toml" | while read -r toml; do \
        echo "Testing plugin: $$(dirname $$toml)"; \
        cargo test --manifest-path "$$toml"; \
    done

clean:
    cargo clean

check:
    cargo check --workspace

default:
    just build bin
