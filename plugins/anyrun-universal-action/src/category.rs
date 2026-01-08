use anyrun_helper::icon::SystemIcon;
use chrono::Utc;
use chrono_english::{Dialect, parse_date_string};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use url::Url;

static RE_COLOR_HEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})$").unwrap());
static RE_GIT_HASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[0-9a-f]{40}$").unwrap());
static RE_EMAIL: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap());

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Copy)]
pub enum InputCategory {
    Json,
    Code,
    ShellCommand,
    GitHash,
    MathExpression,
    Url,
    FilePath,
    IpAddress,
    Color,
    DateTime,
    Email,
    Plaintext,
    All,
}

impl InputCategory {
    pub fn detect(input: &str) -> Self {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Self::Plaintext;
        }

        if trimmed.parse::<IpAddr>().is_ok() {
            return Self::IpAddress;
        }

        if RE_COLOR_HEX.is_match(trimmed) {
            return Self::Color;
        }

        if RE_EMAIL.is_match(trimmed) {
            return Self::Email;
        }

        if RE_GIT_HASH.is_match(trimmed) {
            return Self::GitHash;
        }

        if Url::parse(trimmed).is_ok_and(|url| url.scheme() == "http" || url.scheme() == "https") {
            return Self::Url;
        }

        if trimmed.starts_with('/') || trimmed.starts_with("./") || trimmed.starts_with("~/") {
            return Self::FilePath;
        }

        if ((trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']')))
            && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
        {
            return Self::Json;
        }

        if trimmed.chars().any(|c| "+-*/^%".contains(c))
            && trimmed.chars().any(|c| c.is_numeric())
            && evalexpr::build_operator_tree::<evalexpr::DefaultNumericTypes>(trimmed).is_ok()
        {
            return Self::MathExpression;
        }

        if parse_date_string(trimmed, Utc::now(), Dialect::Uk).is_ok()
            && !trimmed.chars().all(|c| c.is_numeric())
        {
            return Self::DateTime;
        }

        if trimmed.starts_with('$') || trimmed.starts_with("sudo ") {
            return Self::ShellCommand;
        }

        let code_keywords = [
            "fn ", "public ", "import ", "let ", "const ", "def ", "class ", "function", "func",
        ];
        if code_keywords.iter().any(|&k| trimmed.contains(k)) {
            return Self::Code;
        }

        Self::Plaintext
    }

    pub fn get_icon(&self) -> String {
        let icon = match self {
            Self::Json => SystemIcon::Json,
            Self::Code => SystemIcon::FileCode,
            Self::ShellCommand => SystemIcon::Terminal,
            Self::GitHash => SystemIcon::Symbol,
            Self::MathExpression => SystemIcon::Calculator,
            Self::Url => SystemIcon::Url,
            Self::FilePath => SystemIcon::Folder,
            Self::IpAddress => SystemIcon::NetworkStatus,
            Self::Color => SystemIcon::FileImage,
            Self::DateTime => SystemIcon::Language,
            Self::Email => SystemIcon::MailSend,
            Self::Plaintext => SystemIcon::FileText,
            Self::All => SystemIcon::FileText,
        };

        icon.as_str().to_string()
    }
}
