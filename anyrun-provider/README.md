# anyrun-provider

The search backend for [Anyrun](https://github.com/anyrun-org/anyrun). Loads plugin libraries and acts as a middleman between launcher frontends and plugins.

## Overview

Since Anyrun 25.12.0, the search functionality has been extracted into this separate binary. It communicates with the Anyrun UI process via Unix domain sockets using the IPC types in `anyrun-provider-ipc`.

## Usage

The provider is spawned automatically by Anyrun. Set its path in `config.ron` if it's not in `$PATH`:

```ron
Config(
  provider: "/usr/bin/anyrun-provider",
  // ...
)
```

### Standalone

You can also use the provider independently for integrating Anyrun plugins into other applications:

```bash
anyrun-provider --plugins-dir ~/.config/anyrun/plugins --socket /tmp/anyrun-provider.sock
```

## IPC Protocol

Messages use types from the `anyrun-provider-ipc` crate:

### Request

```rust
enum Request {
    Query {
        text: String,           // Search text
        phase: QueryPhase,      // Settling | Flushing
        plugins: Vec<String>,   // Plugin names to query
    },
}
```

### Response

```rust
struct Response {
    plugin: String,
    matches: Vec<Match>,
    phase: QueryPhase,
}
```

### Query Phases

- **Settling**: The final debounced query after the user stops typing (configured via `search_ux.settle_delay_ms`).
- **Flushing**: Intermediate results sent while the user is still typing (configured via `search_ux.flush_delay_ms`).

## CLI Arguments

| Argument | Description |
|----------|-------------|
| `--plugins-dir` | Directory containing plugin `.so` files |
| `--socket` | Path to the Unix domain socket |
| `--listen` | Let the provider create and listen on the socket (vs. using an existing one) |

## Crate

The `anyrun-provider-ipc` crate (in `anyrun-provider-ipc/`) provides the shared request/response types and socket helper utilities for integrating with the provider from other Rust programs.
