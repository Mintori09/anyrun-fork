# AGENTS.md — Anyrun Repository Guidelines

## Project Overview

Anyrun is a Wayland-native application launcher (KRunner-like) written in Rust. It uses GTK4 + relm4 (Elm-like architecture) + gtk4-layer-shell. The project is a Cargo workspace with a core binary, plugin SDK, proc-macros, helper library, and many plugins compiled as `cdylib` `.so` files.

## Build / Test / Lint Commands

```bash
# Build
just build              # core binaries only (anyrun + anyrun-provider)
just build all          # entire workspace
just build plugins      # all plugins
just build <pkg-name>   # single package, e.g. just build anyrun-calc

# Test
just test               # entire workspace
just test all           # same as above
just test bin           # core binaries only
just test <pkg-name>    # single package

# Run
just run                # cargo run -p anyrun

# Check / Clean
just check              # cargo check --workspace
just clean              # cargo clean

# Install
just install            # copy binaries to /usr/bin
just install-plugin     # copy .so files to ~/.config/anyrun/plugins
```

### Running a single test

```bash
cargo test -p <package-name> <test_name>
# Example:
cargo test -p anyrun-bluetooth test_parse_device_info
```

### CI

- GitHub Actions runs `cargo build` with nightly Rust on push/PR to `master`
- `cargo deny` runs for security auditing (`.github/workflows/cargo-audit.yml`)
- Tests and clippy are NOT run in CI — run them locally before submitting

## Code Style

### Formatting

- No `rustfmt.toml` exists — use default `cargo fmt` settings
- TOML files (Cargo.toml) follow `.taplo.toml`: aligned entries, 100-char column width, reordered keys
- Run `cargo fmt` and `taplo format` before committing

### Imports

- Group imports: std library, external crates, local crates, sibling modules
- Plugins always import `abi_stable::std_types::{ROption, RString, RVec}` and `anyrun_plugin::*`
- Use `serde::Deserialize` for config structs (RON format)

### Naming Conventions

- `snake_case` for functions, variables, modules, file names
- `PascalCase` for structs, enums, traits
- `SCREAMING_SNAKE_CASE` for constants (e.g., `PLUGIN_NAME`, `ICON_BLUETOOTH_ACTIVE`)
- Plugin directories use `kebab-case` (e.g., `anyrun-applications`)
- Plugin entry functions (`init`, `info`, `get_matches`, `handler`) are NOT marked `pub` — macros handle FFI export

### Types & FFI

- Use `RString`, `RVec`, `ROption` from `abi_stable::std_types` at plugin FFI boundaries
- Use standard `String`, `Vec`, `Option` internally; convert at the boundary with `.into()`
- `HandleResult` variants: `Close`, `Copy(Vec<u8>)`, `Continue`
- Config files are RON format, named `<plugin_name>.ron`

### Error Handling

- **No custom error enums** — no `thiserror` or `anyhow` in the workspace
- Config loading: chain `map_err` + `and_then` + `unwrap_or_else(|_| Config::default())`
- Graceful degradation: use `.ok()` + `.unwrap_or_default()` for non-critical operations
- Log errors with `eprintln!("[plugin_name] message: {}", error)`
- Panic on critical failures with `expect("...")` or `unwrap()`
- Propagate `arboard::Error` only for clipboard operations

### Async

- Create Tokio runtime per-plugin with `Runtime::new()` when needed; do NOT share across plugins
- Use `state.runtime.block_on(async { ... })` inside sync plugin functions

### Comments

- Code comments are primarily in English (some Vietnamese comments exist from contributors)
- No doc comments are required unless the function is part of a public API

## Plugin Interface Pattern

Every plugin must define these 4 functions using proc-macro attributes:

```rust
use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;

#[derive(Deserialize)]
struct Config { /* ... */ }
impl Default for Config { /* ... */ }

pub struct State { /* runtime data */ }

#[init]
fn init(config_dir: RString) -> State { /* load config, return state */ }

#[info]
fn info() -> PluginInfo {
    PluginInfo { name: "...".into(), icon: "...".into() }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> { /* ... */ }

#[handler]
fn handler(selection: Match) -> HandleResult { HandleResult::Close }
```

- `#[init]` runs in a separate thread; `State` is stored in a `static RwLock<Option<T>>`
- `#[get_matches]` is wrapped in `catch_unwind` to prevent panics from crashing the host
- Use `plugins/plugin-sample/` as a template for new plugins

## Git Commit Messages

Follow Conventional Commits format:

```
type: description
type(scope): description
```

Types: `feat`, `fix`, `refactor`, `style`, `docs`, `chore`

- Lowercase description after type prefix
- No period at end (inconsistent in history, but prefer without)
- Scope is optional, in parentheses: `feat(bluetooth): display battery percentage`

## Key Crates

| Crate                 | Path                                   | Purpose                                                           |
| --------------------- | -------------------------------------- | ----------------------------------------------------------------- |
| `anyrun`              | `anyrun/`                              | Main binary: GTK4 UI, D-Bus IPC, config, plugin loading           |
| `anyrun-plugin`       | `anyrun-plugin/`                       | Plugin SDK: re-exports types + proc-macro attributes              |
| `anyrun-macros`       | `anyrun-macros/`                       | Proc-macros: `#[init]`, `#[info]`, `#[get_matches]`, `#[handler]` |
| `anyrun-helper`       | `anyrun-helper/`                       | Shared utils: icons, logging, fuzzy matcher, clipboard            |
| `anyrun-provider`     | `anyrun-provider/`                     | Separate binary for search results via IPC                        |
| `anyrun-provider-ipc` | `anyrun-provider/anyrun-provider-ipc/` | Shared IPC types                                                  |
