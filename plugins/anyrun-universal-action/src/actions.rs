use crate::category::InputCategory;
use std::process::Command;

#[derive(Debug, Clone)]
pub enum ActionTarget {
    Shell(String),      // Ví dụ: "yt-dlp {clip}"
    Internal(fn(&str)), // Ví dụ: "save_to_db"
}

impl ActionTarget {
    pub fn run_action(&self, clipboard: &str) {
        match self {
            ActionTarget::Shell(cmd_template) => {
                let cmd_script = cmd_template.replace("{clip}", clipboard);
                let _ = Command::new("sh").arg("-c").arg(cmd_script).spawn();
            }
            ActionTarget::Internal(func) => {
                func(clipboard);
            }
        }
    }
}

pub struct UniversalAction {
    pub name: String,
    pub name_lowercase: String,
    pub category: InputCategory,
    pub target: ActionTarget,
    pub validator: Option<fn(&str) -> bool>,
}

impl UniversalAction {
    pub fn is_match(&self, clipboard: &str, detected_cat: InputCategory) -> bool {
        let category_ok = self.category == InputCategory::All || self.category == detected_cat;

        if !category_ok {
            return false;
        }

        if let Some(validator_fn) = self.validator {
            return validator_fn(clipboard);
        }

        true
    }
}
