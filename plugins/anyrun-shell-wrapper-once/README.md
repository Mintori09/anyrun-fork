# Shell Wrapper Once

A plugin for [Anyrun](/../../) that executes a shell command with user-provided input as an argument, showing the result as a single match.

## Features

- **One-shot Execution**: Type a query and execute a predefined command with that input.
- **Named Scopes**: Define multiple scopes with different prefixes and commands.
- **Shell Escaping**: User input is safely shell-escaped.
- **Error Notifications**: Sends desktop notifications on configuration or execution errors.

## Usage

Define scopes with a prefix and command template. When the prefix is matched, the text after it becomes the command argument.

```ron
Config(
  scopes: [
    Scope(
      prefix: "yt ",
      description: "Search YouTube",
      command: "firefox https://youtube.com/results?search_query={}",
    ),
  ],
)
```

Typing `yt never gonna give you up` would execute:
```
firefox https://youtube.com/results?search_query=never+gonna+give+you+up
```

## Configuration

The configuration is defined in `shell_wrapper_once.ron` in your Anyrun config directory.

```ron
Config(
  scopes: [
    Scope(
      // The prefix to trigger this scope
      prefix: "s ",

      // Description shown in the match
      description: "Search on example",

      // Command template. {} is replaced with the shell-escaped user query.
      command: "xdg-open https://example.com/search?q={}",
    ),
  ],
)
```

### Scope Fields

- `prefix` (string): Trigger prefix (case-insensitive match).
- `description` (string): Displayed in the match description.
- `command` (string): Shell command template. Use `{}` as the placeholder for the user input. If `{}` is absent, the input is appended.
