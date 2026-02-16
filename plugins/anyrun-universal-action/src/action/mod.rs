use crate::action::model::{ActionTarget, InputCategory};

pub mod check_match;
pub mod clipboard;
pub mod model;

impl ActionTarget {
    pub fn run_action(&self, filetype: InputCategory, clipboard: &str) {
        match self {
            ActionTarget::Shell(cmd_template) => {
                let cmd_script = cmd_template.replace("{clip}", clipboard);
                let _ = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd_script)
                    .spawn();
            }
            ActionTarget::Internal(func) => {
                func(clipboard, filetype);
            }
        }
    }
}
