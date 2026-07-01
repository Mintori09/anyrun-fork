use crate::{
    action::model::{ActionTarget, InputCategory},
    helper::path::resolve_path,
};

pub mod check_match;
pub mod clipboard;
pub mod model;

impl ActionTarget {
    pub fn run_action(&self, filetype: InputCategory, clipboard: &str) {
        match self {
            ActionTarget::Shell(cmd_template) => {
                let content_type = filetype.clone();
                let clip = if let InputCategory::System { kind } = &content_type {
                    if kind == "path" {
                        resolve_path(clipboard)
                    } else {
                        clipboard.to_string()
                    }
                } else {
                    clipboard.to_string()
                };

                let cmd_script = cmd_template
                    .replace("{clip}", &clip)
                    .replace("{ext}", &filetype.get_extension());
                let _ = anyrun_plugin::spawn_detached(
                    std::process::Command::new("sh").arg("-c").arg(cmd_script),
                );
            }
            ActionTarget::Internal(func) => {
                func(clipboard, filetype);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mocking get_full_path for the sake of the test environment
    // Remove this mock if it's already accessible in your test scope
    #[test]
    fn test_resolve_path() {
        let clipboard = "~/Desktop/neovim.md";
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/mintori".to_string());

        let expected_path = format!("{}{}", home, &clipboard[1..]);

        assert_eq!(resolve_path(clipboard), expected_path);
    }

    #[test]
    fn test_run_action_system_path() {
        // Arrange
        let target = ActionTarget::Shell("echo {clip} has extension {ext}".to_string());
        let filetype = InputCategory::System {
            kind: "path".to_string(),
        };
        let clipboard = "relative/file.txt";

        // Act & Assert
        // Since spawn_detached runs in the background and returns a result we discard,
        // we can verify it compiles and runs without panicking.
        target.run_action(filetype, clipboard);
    }

    #[test]
    fn test_run_action_plain_text() {
        // Arrange
        let target = ActionTarget::Shell("echo {clip}".to_string());
        let filetype = InputCategory::PlainText;
        let clipboard = "just some standard text";

        // Act
        target.run_action(filetype, clipboard);
    }
}
