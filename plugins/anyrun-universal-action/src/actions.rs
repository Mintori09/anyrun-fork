use crate::category::InputCategory;
use std::process::Command;

#[derive(Debug, Clone)]
pub enum ActionTarget {
    Shell(String),                     // Ví dụ: "yt-dlp {clip}"
    Internal(fn(&str, InputCategory)), // Ví dụ: "save_to_db"
}

impl ActionTarget {
    pub fn run_action(&self, filetype: InputCategory, clipboard: &str) {
        match self {
            ActionTarget::Shell(cmd_template) => {
                let cmd_script = cmd_template.replace("{clip}", clipboard);
                let _ = Command::new("sh").arg("-c").arg(cmd_script).spawn();
            }
            ActionTarget::Internal(func) => {
                func(clipboard, filetype);
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
        let category_ok = match (&self.category, &detected_cat) {
            (
                InputCategory::Code { lang: cfg_lang, .. },
                InputCategory::Code { lang: det_lang, .. },
            ) => {
                cfg_lang == "any"
                    || cfg_lang.is_empty()
                    || cfg_lang == "all"
                    || cfg_lang == det_lang
            }

            (c1, c2) => c1 == c2,
        };

        if !category_ok {
            return false;
        }

        if let Some(validator_fn) = self.validator {
            return validator_fn(clipboard);
        }

        true
    }
}
