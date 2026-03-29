use anyrun_helper::set_clipboard;
use std::{collections::HashMap, fs, process::Command};

use crate::action::model::InputCategory;

pub fn format_content(content: &str, filetype: InputCategory) {
    let extension = InputCategory::get_extension(&filetype);
    let file_path = format!("/tmp/anyrun_format{}", extension);

    if let Err(e) = fs::write(&file_path, content) {
        eprintln!("Failed to write temp file: {}", e);
        return;
    }

    let tools = HashMap::from([
        (".java", "google-java-format"),
        (".rs", "rustfmt"),
        (".dart", "dart format"),
    ]);

    if let Some(tool) = tools.get(extension.as_str()) {
        let output = Command::new(tool).arg(&file_path).output();

        match output {
            Ok(out) if out.status.success() => {
                let formatted = String::from_utf8_lossy(&out.stdout).to_string();
                set_clipboard(formatted).unwrap();
                println!("Formatted and copied {} snippet!", extension);
            }
            _ => {
                set_clipboard(content.to_string()).unwrap();
                println!("Formatter failed; copied raw content instead.");
            }
        }
    } else {
        set_clipboard(content.to_string()).unwrap();
    }

    let _ = fs::remove_file(&file_path);
}
