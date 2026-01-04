# Web Search

A simple but powerful web search plugin for [Anyrun](https://github.com/Kirottu/anyrun) with custom engine support.

## Usage

Use a configured engine prefix followed by your search query. For example, `g my query` to search Google. Selecting the result will open your default browser.

## Configuration

The configuration is done in `websearchs.ron` located in your Anyrun config directory.

```ron
Config(
  engines: [
    SearchEngine(
      name: "Google",
      prefix: "g ",
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
  ],
)
```
