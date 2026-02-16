use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum ActionTarget {
    Shell(String),                     // Eg: "yt-dlp {clip}"
    Internal(fn(&str, InputCategory)), // Eg: "save_to_db"
}

pub struct UniversalAction {
    pub name: String,
    pub name_lowercase: String,
    pub category: InputCategory,
    pub target: ActionTarget,
    pub validator: Option<fn(&str) -> bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "lowercase")]
pub enum InputCategory {
    Code {
        lang: String,
    },
    System {
        kind: String,
    },
    Design {
        kind: String,
    },
    #[serde(alias = "PlainText", alias = "Plaintext")]
    PlainText,
    Image,
}
