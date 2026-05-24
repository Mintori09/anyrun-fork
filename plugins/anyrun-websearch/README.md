# Web Search

A simple but powerful web search plugin for [Anyrun](https://github.com/Kirottu/anyrun) with custom engine support.

## Usage

Use a configured engine prefix followed by your search query.

Default examples:
- `gg rust relm4` -> Google
- `gh anyrun` -> GitHub

Selecting a result opens your default browser. If configured, the plugin also focuses that browser window.

## Configuration

The configuration is done in `websearchs.ron` located in your Anyrun config directory.

```ron
Config(
  // Optional: focus this browser window class after opening URL.
  // Common classes: "firefox", "zen", "google-chrome", "chromium", "brave-browser"
  // Leave as None to disable forced focus.
  focus_class: Some("firefox"),

  // Delay before focus attempt (milliseconds).
  focus_delay_ms: 120,

  engines: [
    SearchEngine(
      name: "Google",
      prefix: "gg ",
      url: "https://www.google.com/search?q={}",
    ),
    SearchEngine(
      name: "GitHub",
      prefix: "gh ",
      url: "https://github.com/search?q={}",
    ),
    SearchEngine(
      name: "DuckDuckGo",
      prefix: "d ",
      url: "https://duckduckgo.com/?q={}",
    ),
    SearchEngine(
      name: "YouTube",
      prefix: "yt ",
      url: "https://www.youtube.com/results?search_query={}",
    ),
    SearchEngine(
      name: "Twitter/X",
      prefix: "tw ",
      url: "https://x.com/search?q={}",
    ),
  ],
)
```

If your browser opens but does not gain focus, set `focus_class` to your browser class and adjust `focus_delay_ms` (e.g. 120-300).
