use anyrun_helper::icon::{SystemIcon, home_dir};
use arboard::Clipboard;
use once_cell::sync::Lazy;
use regex::Regex;
use std::env;
use std::path::PathBuf;

use crate::action::model::InputCategory;
use crate::helper::detect_and_save::call_magika;

static COLOR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$").unwrap());
static EMAIL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap());
static IPV4_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\d{1,3}\.){3}\d{1,3}(:\d+)?$").unwrap());
static IPV6_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}$").unwrap());
static URL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(https?|ftp|file)://").unwrap());

impl InputCategory {
    pub fn classify_clipboard() -> (InputCategory, String) {
        let mut ctx = match Clipboard::new() {
            Ok(c) => c,
            Err(_) => return (InputCategory::PlainText, String::new()),
        };

        if ctx.get_image().is_ok() {
            return (InputCategory::Image, String::new());
        }

        let text = match ctx.get_text() {
            Ok(t) => t,
            Err(_) => return (InputCategory::PlainText, String::new()),
        };

        (Self::classify_text(&text), text)
    }

    pub fn classify_text(text: &str) -> InputCategory {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return InputCategory::PlainText;
        }

        // 1. Check Color Hex (#RGB, #RGBA, #RRGGBB, #RRGGBBAA)
        if COLOR_RE.is_match(trimmed) {
            return InputCategory::Design {
                kind: "hex_color".into(),
            };
        }

        // 2. Check Email
        if trimmed.contains('@') && EMAIL_RE.is_match(trimmed) {
            return InputCategory::System {
                kind: "email".into(),
            };
        }

        // 3. Check IP Address (IPv4 / IPv6)
        if IPV4_RE.is_match(trimmed) {
            return InputCategory::System { kind: "ip".into() };
        }
        if IPV6_RE.is_match(trimmed) {
            return InputCategory::System { kind: "ip".into() };
        }

        // 4. Check URL
        if URL_RE.is_match(trimmed) || trimmed.starts_with("www.") {
            return InputCategory::System { kind: "url".into() };
        }

        // 5. Check Path
        let expanded_path = Self::expand_path(trimmed);
        let looks_like_path = trimmed.starts_with('~')
            || trimmed.starts_with('$')
            || trimmed.contains('/')
            || trimmed.contains('\\')
            || (trimmed.len() >= 3
                && trimmed.as_bytes()[1] == b':'
                && (trimmed.as_bytes()[2] == b'\\' || trimmed.as_bytes()[2] == b'/'));
        if looks_like_path && expanded_path.exists() {
            return InputCategory::System {
                kind: "path".into(),
            };
        }

        // 6. Manual Heuristics (fast checks before expensive ones)
        if trimmed.contains('$') && trimmed.contains("foreach") && trimmed.contains(';') {
            return InputCategory::Code { lang: "php".into() };
        }

        // 7. Check via Magika (skip for short text — score never > 0.30 anyway)
        if trimmed.len() >= 20
            && let Some((group, label, score)) = call_magika(trimmed)
            && score > 0.30
        {
            match group.as_str() {
                "code" => return InputCategory::Code { lang: label },
                // Treat structured / markup text as code
                "text"
                    if matches!(
                        label.as_str(),
                        "xml" | "yaml" | "toml" | "markdown" | "latex" | "jsonl"
                    ) =>
                {
                    return InputCategory::Code { lang: label };
                }
                _ => {}
            }
        }

        // 8. Check JSON (expensive, so done last before fallback)
        if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
            return InputCategory::Code {
                lang: "json".into(),
            };
        }

        InputCategory::PlainText
    }

    pub fn get_extension(&self) -> String {
        match self {
            Self::Code { lang, .. } => match lang.to_lowercase().as_str() {
                "rust" => "rs".to_string(),
                "python" => "py".to_string(),
                "javascript" => "js".to_string(),
                "typescript" => "ts".to_string(),
                "java" => "java".into(),
                "c" => "c".into(),
                "c++" | "cpp" => "cpp".to_string(),
                "cs" | "csharp" => "cs".into(),
                "go" => "go".into(),
                "ruby" => "rb".into(),
                "kotlin" | "kt" => "kt".into(),
                "swift" => "swift".into(),
                "dart" => "dart".into(),
                "scala" => "scala".into(),
                "shell" | "sh" => "sh".into(),
                "json" => "json".into(),
                "xml" => "xml".into(),
                "yaml" => "yaml".into(),
                "toml" => "toml".into(),
                "ron" => "ron".into(),
                "html" | "htm" => "html".into(),
                "css" => "css".into(),
                "sql" => "sql".into(),
                "markdown" => "md".into(),
                "latex" => "tex".into(),
                "csv" => "csv".into(),
                "jsonl" => "jsonl".into(),
                _ => lang.to_string(),
            },
            Self::System { kind } => match kind.as_str() {
                "path" => "path".into(),
                "url" => "url".into(),
                "email" => "eml".into(),
                "ip" => "ip".into(),
                _ => "md".into(),
            },
            Self::Design { kind } if kind == "hex_color" => "color".into(),
            Self::Image => "png".into(),
            Self::PlainText => "md".into(),
            _ => "".into(),
        }
    }

    pub fn get_icon(&self) -> SystemIcon {
        match self {
            Self::Code { lang, .. } => match lang.to_lowercase().as_str() {
                "rust" | "rs" => SystemIcon::Rust,
                "javascript" | "js" => SystemIcon::JavaScript,
                "typescript" | "ts" => SystemIcon::TypeScript,
                "python" | "py" => SystemIcon::Python,
                "java" => SystemIcon::FileCode,
                "c" => SystemIcon::C,
                "c++" | "cpp" => SystemIcon::Cpp,
                "cs" | "csharp" => SystemIcon::FileCode,
                "go" => SystemIcon::Go,
                "ruby" | "rb" => SystemIcon::FileCode,
                "kotlin" | "kt" => SystemIcon::FileCode,
                "swift" => SystemIcon::FileCode,
                "dart" => SystemIcon::FileCode,
                "scala" => SystemIcon::FileCode,
                "php" => SystemIcon::PHP,
                "lua" => SystemIcon::Lua,
                "sh" | "shell" | "bash" | "zsh" => SystemIcon::Shell,
                "nix" => SystemIcon::Nix,
                "json" | "jsonl" => SystemIcon::Json,
                "yaml" | "yml" => SystemIcon::Yaml,
                "toml" => SystemIcon::Toml,
                "html" | "htm" => SystemIcon::Html,
                "css" => SystemIcon::Css,
                "ron" | "config" => SystemIcon::Config,
                "sql" => SystemIcon::FileCode,
                "markdown" => SystemIcon::FileText,
                "latex" => SystemIcon::FileText,
                "csv" => SystemIcon::FileText,
                _ => SystemIcon::FileCode,
            },
            Self::System { kind } => match kind.to_lowercase().as_str() {
                "url" => SystemIcon::Url,
                "path" => SystemIcon::Folder,
                "ip" => SystemIcon::NetworkStatus,
                "email" => SystemIcon::MailSend,
                _ => SystemIcon::SystemRun,
            },
            Self::Design { .. } => SystemIcon::Display,
            Self::Image => SystemIcon::FileImage,
            _ => SystemIcon::FileText,
        }
    }
    pub fn expand_path(input: &str) -> PathBuf {
        if input.starts_with('~')
            && let Some(home_dir) = home_dir()
        {
            return home_dir.join(input.trim_start_matches("~/"));
        }

        if input.starts_with("$HOME")
            && let Ok(home_env) = env::var("HOME")
        {
            let rest = input.trim_start_matches("$HOME").trim_start_matches('/');
            return PathBuf::from(home_env).join(rest);
        }

        PathBuf::from(input)
    }
}
