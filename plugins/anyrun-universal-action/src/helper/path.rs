use crate::action::model::InputCategory;
use anyrun_helper::set_clipboard;
use std::env;

pub fn get_full_path(content: &str, _content_type: InputCategory) {
    let path_str = if content.starts_with('~') {
        content.replacen('~', &env::var("HOME").unwrap_or_default(), 1)
    } else if content.contains("$HOME") {
        content.replace("$HOME", &env::var("HOME").unwrap_or_default())
    } else {
        content.to_string()
    };

    set_clipboard(path_str.to_string()).ok();
}
