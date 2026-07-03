# Sub-command Routing for `anyrun-git-projects`

## Summary

Add configurable sub-command routing to `anyrun-git-projects` so users can type
`git/nvim myrepo` to open a repo in Neovim, `git/op myrepo` for OpenCode, or
`git/ myrepo` (no sub-command) for the default terminal behavior.

## Config

Replace the `shell` field with a `default_command` template and a `commands`
map:

```ron
Config(
    prefix: "git/",
    max_entries: 10,
    show_results_immediately: true,
    cache_ttl_hours: 0,
    default_command: "kitty --directory {path}",
    commands: {
        "nvim": (
            command: "nvim {path}",
            icon: "nvim",
        ),
        "op": (
            command: "opencode {path}",
            icon: "opencode",
        ),
        "code": (
            command: "code {path}",
            icon: "visual-studio-code",
        ),
    },
)
```

**Migration:** If old config has `shell` but no `default_command`,
auto-construct `default_command: "cd {path} && exec <shell>"` to avoid breaking
existing configs.

**New structs:**

```rust
#[derive(Deserialize)]
struct SubCommand {
    command: String,
    icon: String,
}
```

Config struct:
- Remove `shell`
- Add `default_command: String` (default: `"kitty --directory {path}"`)
- Add `commands: HashMap<String, SubCommand>` (default: empty)

## Matching Logic (`get_matches`)

```
Input:           "git/nvim my proj"
Strip prefix:    "nvim my proj"
First word match?  "nvim" ∈ commands → sub_cmd = "nvim", query = "my proj"
                   Not found        → sub_cmd = None, query = whole input

Fuzzy match repos against query.
```

Match struct:
- `icon` → Sub-command's `icon` if routed, else `SystemIcon::Folder`
- `id`  → Serialized command template + path, or separated by delimiter
- `description` → Full repo path (unchanged)

## Handler

1. Extract command template from `selection.id`
2. Extract path from `selection.description`
3. Execute `sh -c "{command}"` after replacing `{path}` with escaped path
4. HandleResult::Close

## Backward Compatibility

- Old `shell` field → auto-generate `default_command` if missing
- Old prefix `git/` still works
- Old results with `ROption::RNone` id → use `default_command`

## Files Changed

| File | Change |
|------|--------|
| `plugins/anyrun-git-projects/src/lib.rs` | Add SubCommand struct, update Config, update matching, update handler |
| `plugins/anyrun-git-projects/git-projects.ron` | New config format |
| (cache format unchanged) | |
