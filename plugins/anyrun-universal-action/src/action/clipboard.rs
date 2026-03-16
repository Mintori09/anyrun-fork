use anyrun_helper::icon::{SystemIcon, home_dir};
use arboard::Clipboard;
use std::env;
use std::path::PathBuf;

use crate::action::model::InputCategory;
use crate::helper::detect_and_save::call_magika;

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

        // 1. Check Color Hex
        let color_re = regex::Regex::new(r"^#(?:[0-9a-fA-F]{3}){1,2}$").unwrap();
        if color_re.is_match(trimmed) {
            return InputCategory::Design {
                kind: "hex_color".into(),
            };
        }

        // 2. Check URL
        if (trimmed.starts_with("http") && trimmed.contains("://")) || trimmed.starts_with("www.") {
            return InputCategory::System { kind: "url".into() };
        }

        // 3. Check Path
        let expanded_path = Self::expand_path(trimmed);
        if (trimmed.contains('/') || trimmed.contains('\\') || trimmed.starts_with('~'))
            && expanded_path.exists()
        {
            return InputCategory::System {
                kind: "path".into(),
            };
        }

        // 4. Check JSON (Merged into Code)
        if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
            return InputCategory::Code {
                lang: "json".into(),
            };
        }

        // 5. Check via Magika (Merged Data types into Code)
        if let Some((group, label, score)) = call_magika(trimmed)
            && score > 0.45
        {
            match group.as_str() {
                "code" => return InputCategory::Code { lang: label },
                // Treat structured data as code
                "text" if label == "xml" || label == "yaml" || label == "toml" => {
                    return InputCategory::Code { lang: label };
                }
                _ => {}
            }
        }

        // 6. Manual Heuristics
        if trimmed.contains('$') && trimmed.contains("foreach") && trimmed.contains(';') {
            return InputCategory::Code { lang: "php".into() };
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
                "c++" | "cpp" => "cpp".to_string(),
                "json" => "json".into(),
                "xml" => "xml".into(),
                "yaml" => "yaml".into(),
                "ron" => "ron".into(),
                _ => lang.to_string(),
            },
            Self::System { kind } => match kind.as_str() {
                "path" => "path".into(),
                "url" => "url".into(),
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
            Self::Code { lang, .. } => {
                match lang.to_lowercase().as_str() {
                    "rust" | "rs" => SystemIcon::Rust,
                    "javascript" | "js" => SystemIcon::JavaScript,
                    "typescript" | "ts" => SystemIcon::TypeScript,
                    "python" | "py" => SystemIcon::Python,
                    "php" => SystemIcon::PHP,
                    "lua" => SystemIcon::Lua,
                    "sh" | "shell" | "bash" | "zsh" => SystemIcon::Shell,
                    "nix" => SystemIcon::Nix,
                    "json" => SystemIcon::Json,
                    "yaml" | "yml" => SystemIcon::Yaml,
                    "toml" => SystemIcon::Toml,
                    "html" => SystemIcon::Html,
                    "css" => SystemIcon::Css,
                    "ron" | "config" => SystemIcon::Config,
                    _ => SystemIcon::FileCode, // Default for other code types
                }
            }
            Self::System { kind } => match kind.to_lowercase().as_str() {
                "url" => SystemIcon::Url,
                "path" => SystemIcon::Folder,
                "ip" => SystemIcon::NetworkStatus,
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
