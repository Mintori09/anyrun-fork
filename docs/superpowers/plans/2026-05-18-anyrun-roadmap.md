# Anyrun Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve Anyrun diagnostics, discovery, empty-state UX, result ranking, plugin health, and local result actions.

**Architecture:** Keep plugin ABI unchanged. Extend internal IPC for recent rows and health updates, store recent state in provider-owned frecency data, and render non-provider UI rows through explicit row source metadata.

**Tech Stack:** Rust, GTK4/relm4, tokio, serde/bincode IPC, cargo fmt/clippy/test.

---

### Task 1: IPC and Config Primitives

**Files:**
- Modify: `anyrun-provider/anyrun-provider-ipc/src/lib.rs`
- Modify: `anyrun/src/config/search.rs`
- Modify: `anyrun/src/config/mod.rs`

- [x] Add `RecentMatch`, `PluginHealthState`, and `PluginHealth`.
- [x] Add `Request::Recent`, `Response::Recent`, and `Response::Health`.
- [x] Add query `timeout_ms` and `slow_ms`.
- [x] Add search UX defaults for timeout, slow threshold, prefix discovery, and empty recent state.
- [x] Verify with targeted cargo tests.

### Task 2: Provider Recent, Ranking, and Health

**Files:**
- Modify: `anyrun-provider/src/main.rs`

- [x] Store recent selections in `frecency.json` with serde defaults for old files.
- [x] Rank matches with order base, frecency, exact boost, and starts-with boost.
- [x] Add per-plugin query timeout and slow/healthy/timed-out health events.
- [x] Return recent matches through `Request::Recent`.
- [x] Verify with provider unit tests.

### Task 3: UX Rows and Actions

**Files:**
- Modify: `anyrun/src/plugin_box/match_row.rs`
- Modify: `anyrun/src/app/*.rs`
- Modify: `anyrun/src/config/keybind.rs`

- [x] Add row source metadata for provider, recent, prefix help, and action rows.
- [x] Show recent rows on empty state when immediate search is disabled.
- [x] Show prefix help rows for `?`.
- [x] Add `OpenActions`, default `Ctrl+Return`, with open/copy local actions.
- [x] Show concise plugin issue count in footer.

### Task 4: Doctor, CI, and Docs

**Files:**
- Create: `anyrun/src/doctor.rs`
- Modify: `anyrun/src/args.rs`
- Modify: `anyrun/src/main.rs`
- Modify: `.github/workflows/cargo-build.yml`
- Modify: `README.md`
- Modify: `examples/config.ron`

- [x] Add `anyrun doctor` for config parse, provider resolution, and plugin loadability.
- [x] Add CI fmt and clippy gates.
- [x] Update docs for `doctor`, `GtkEntry`, `OpenActions`, and new search UX fields.

### Task 5: Final Verification

- [x] Run `cargo fmt --all --check`.
- [x] Run `cargo clippy --workspace -- -D warnings`.
- [x] Run `cargo test --workspace`.
- [x] Run `cargo check --workspace`.
