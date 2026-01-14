use anyrun_helper::icon::SystemIcon;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use url::Url;

static RE_EMAIL: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap());

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Copy)]
pub enum InputCategory {
    Json,
    Code,
    Url,
    IpAddress,
    Email,
    Plaintext,
    All,
    #[serde(other)]
    Unknown,
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

        if RE_EMAIL.is_match(trimmed) {
            return Self::Email;
        }

        if Url::parse(trimmed).is_ok_and(|url| url.scheme() == "http" || url.scheme() == "https") {
            return Self::Url;
        }

        if ((trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']')))
            && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
        {
            return Self::Json;
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
            Self::Url => SystemIcon::Url,
            Self::IpAddress => SystemIcon::NetworkStatus,
            Self::Email => SystemIcon::MailSend,
            Self::Plaintext => SystemIcon::FileText,
            Self::All => SystemIcon::FileText,
            Self::Unknown => SystemIcon::SystemRun,
        };

        icon.as_str().to_string()
    }
}
