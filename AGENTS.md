# AGENTS.md — Anyrun Repository Guidelines

Anyrun is a Wayland-native application launcher (KRunner-like). Rust, GTK4 + relm4 + gtk4-layer-shell. Cargo workspace: core binary, plugin SDK, proc-macros, helper, provider binary, many cdylib plugins.

## Build / Test / Lint

```bash
just build              # entire workspace (--release)
just build bin          # core binaries (anyrun + anyrun-provider)
just build plugins      # each plugin via find
just build <pkg>        # single package
just test               # cargo test --workspace
just test bin           # core only
just test <pkg>         # single package
just run                # rebuild all, then run
just daemon             # rebuild all, then run daemon
just check              # cargo check --workspace (no build)
just clean              # cargo clean
just install            # sudo cp binaries to /usr/bin
just install-plugin     # cp .so files to ~/.config/anyrun/plugins
```

Single test: `cargo test -p <pkg> <test_name>`

Nix: `nix build` in `nix/packages/`; home-manager module at `nix/modules/home-manager.nix`.

CI: nightly Rust, `cargo build` only (no tests/clippy). `cargo deny` weekly.

## Core Architecture

- **anyrun** — relm4 component (`anyrun/src/app/mod.rs`). Messages via `AppMsg` enum, UI via `AppWidgets`. GTK4 layer-shell window.
- **anyrun-provider** — required separate binary (since 25.12.0). Spawns plugin processes, collects results via IPC. Set path in config: `provider: "anyrun-provider"`.
- **IPC** — `anyrun_provider_ipc::Request::Query { text, phase, plugins }`. `QueryPhase::Settling` (debounced final query) then `QueryPhase::Flushing` (batched incremental). Search UX config in `config.ron`: `search_ux` block with `settle_delay_ms`, `flush_delay_ms`, `typing_visual`, `bare_text_fast_lane`, `prefix_routes`.

## Code Conventions

- No `rustfmt.toml`; default `cargo fmt`. TOML: `.taplo.toml` (aligned, 100-char, reordered keys). Run both before commit.
- No `thiserror`, no `anyhow`. Config: `ron::from_str(...).ok().unwrap_or_default()`. Errors: `eprintln!("[name] {}", err)`.
- Plugin FFI boundaries: `abi_stable::std_types::{RString, RVec, ROption}` → convert with `.into()` to `String`/`Vec`/`Option`.
- `HandleResult`: `Close` / `Copy(Vec<u8>)` / `Continue`.
- Plugin config: RON files named `<plugin_name>.ron` in config dir.
- Plugin functions NOT `pub` — proc-macros handle FFI export.
- Vietnamese comments exist in some plugins; no doc comments required.

## Plugin Interface

4 proc-macro functions. `#[get_matches]` and `#[handler]` optionally borrow `<State>` (`&T` or `&mut T`):

```rust
use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;

#[derive(Deserialize)]
struct Config { /* ... */ }
impl Default for Config { /* ... */ }
pub struct State { /* ... */ }

#[init]
fn init(config_dir: RString) -> State { /* loads config, returns State */ }

#[info]
fn info() -> PluginInfo { PluginInfo { name: "...", icon: "..." } }

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> { /* ... */ }

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult { HandleResult::Close }
```

- `#[init]` spawns a thread; State stored in `static RwLock<Option<T>>`.
- `#[get_matches]` wrapped in `catch_unwind`.
- Template: `plugins/plugin-sample/` (commented out in workspace, but valid source).

## Git

Conventional Commits: `type(scope): description`. Types: `feat`, `fix`, `refactor`, `style`, `docs`, `chore`. No period at end. Scope optional.

## Key Crates

| Crate                 | Path                                   | Purpose                                           |
| --------------------- | -------------------------------------- | ------------------------------------------------- |
| `anyrun`              | `anyrun/`                              | GTK4 UI, D-Bus IPC, config, plugin loading        |
| `anyrun-plugin`       | `anyrun-plugin/`                       | Plugin SDK: re-exports + proc-macro attrs         |
| `anyrun-macros`       | `anyrun-macros/`                       | `#[init]`, `#[info]`, `#[get_matches]`, `#[handler]` |
| `anyrun-helper`       | `anyrun-helper/`                       | Icons, logger, fuzzy matcher, clipboard            |
| `anyrun-provider`     | `anyrun-provider/`                     | Search provider binary (IPC)                       |
| `anyrun-provider-ipc` | `anyrun-provider/anyrun-provider-ipc/` | Shared IPC request/response types                  |
