use crate::{
    actions::{ActionTarget, UniversalAction},
    category::InputCategory,
    helper::detect_and_save::detect_and_save,
    validate::is_youtube,
};

pub fn get_internal_actions() -> Vec<UniversalAction> {
    vec![
        UniversalAction {
            name: "Download Youtube Video".into(),
            name_lowercase: "download video youtube".into(),
            category: InputCategory::Url,
            target: ActionTarget::Shell("kitty -- zsh -c \"yt-dlp '{clip}'\"".into()),
            validator: Some(is_youtube),
        },
        UniversalAction {
            name: "Save to Desktop".into(),
            name_lowercase: "save to desktop".into(),
            category: InputCategory::All,
            target: ActionTarget::Internal(detect_and_save),
            validator: None,
        },
        UniversalAction {
            name: "Open path with neovim".into(),
            name_lowercase: "open path with neovim".into(),
            category: InputCategory::All,
            target: ActionTarget::Shell(
                "kitty -- zsh -c \"nvim $(wl-paste | tr -d '\\n')\"".into(),
            ),
            validator: None,
        },
    ]
}
