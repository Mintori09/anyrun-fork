mod actions;
mod category;
mod helper;
mod registry;
mod validate;

use abi_stable::std_types::ROption::{RNone, RSome};
use abi_stable::std_types::{RString, RVec};
use anyrun_helper::get_clipboard;
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::fs::{self};
use std::path::PathBuf;

use crate::actions::UniversalAction;
use crate::category::InputCategory;
use crate::registry::get_internal_actions;

#[derive(Deserialize, Debug)]
struct Action {
    name: String,
    command: String,
    data_type: InputCategory,
}

#[derive(Deserialize, Debug, Default)]
struct Config {
    #[serde(default = "default_prefix")]
    prefix: String,
    actions: Vec<Action>,
    #[serde(default = "default_max_entries")]
    max_entries: usize,
}

fn default_prefix() -> String {
    ":ua ".into()
}
fn default_max_entries() -> usize {
    5
}

pub struct State {
    filetype: InputCategory,
    config: Config,
    actions: Vec<UniversalAction>,
    matcher: SkimMatcherV2,
    clipboard: String,
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("universal_action.ron");

    let config: Config = fs::read_to_string(&config_path)
        .map_err(|e| format!("IO Error: {}", e))
        .and_then(|content| ron::from_str(&content).map_err(|e| format!("RON Error: {}", e)))
        .unwrap_or_else(|_err| Config {
            prefix: default_prefix(),
            actions: Vec::new(),
            max_entries: 5,
        });

    let mut actions = get_internal_actions();
    let config_actions: Vec<UniversalAction> = config
        .actions
        .iter()
        .map(|a| UniversalAction {
            name: a.name.clone(),
            name_lowercase: a.name.to_lowercase(),
            target: actions::ActionTarget::Shell(a.command.clone()),
            category: a.data_type.clone(),
            validator: None,
        })
        .collect();

    actions.extend(config_actions);

    State {
        filetype: InputCategory::classify_clipboard(),
        clipboard: get_clipboard(),
        config,
        actions,
        matcher: SkimMatcherV2::default(),
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Universal Action".into(),
        icon: "edit-paste-symbolic".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let query_text = match input.strip_prefix(&state.config.prefix) {
        Some(stripped) if !stripped.is_empty() => stripped.trim_start(),
        Some(stripped) => stripped,
        _ => return RVec::new(),
    };

    let query_trimmed = query_text.trim();
    let is_empty_query = query_trimmed.is_empty();

    let common_icon = RSome(state.filetype.get_icon().as_str().into());
    let common_desc = RSome(format!("Run action for {:?}", state.filetype).into());

    let limit = if is_empty_query {
        10
    } else {
        state.config.max_entries
    };

    let mut scores: Vec<(i64, &UniversalAction)> = state
        .actions
        .iter()
        .filter(|a| a.is_match(&state.clipboard, state.filetype.clone()))
        .filter_map(|action| {
            if is_empty_query {
                Some((0, action))
            } else {
                state
                    .matcher
                    .fuzzy_match(&action.name_lowercase, query_trimmed)
                    .map(|score| (score, action))
            }
        })
        .collect();

    if !is_empty_query {
        scores.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    }

    scores
        .into_iter()
        .take(limit)
        .map(|(_, action)| Match {
            title: action.name.clone().into(),
            description: common_desc.clone(),
            icon: common_icon.clone(),
            id: RNone,
            use_pango: false,
        })
        .collect()
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    if let Some(action) = state
        .actions
        .iter()
        .find(|a| a.name == selection.title.to_string())
    {
        action
            .target
            .run_action(state.filetype.clone(), &state.clipboard);
    }

    HandleResult::Close
}

#[test]
fn test_read_config() {
    let config_dir = "/home/mintori/.config/anyrun";
    let config_path = PathBuf::from(config_dir.to_string()).join("universal_action.ron");

    let config: Config = fs::read_to_string(&config_path)
        .map_err(|e| format!("IO Error: {}", e))
        .and_then(|content| ron::from_str(&content).map_err(|e| format!("RON Error: {}", e)))
        .unwrap_or_else(|_err| Config {
            prefix: default_prefix(),
            actions: Vec::new(),
            max_entries: 5,
        });

    let mut actions = get_internal_actions();
    let config_actions: Vec<UniversalAction> = config
        .actions
        .iter()
        .map(|a| UniversalAction {
            name: a.name.clone(),
            name_lowercase: a.name.to_lowercase(),
            target: actions::ActionTarget::Shell(a.command.clone()),
            category: a.data_type.clone(),
            validator: None,
        })
        .collect();

    actions.extend(config_actions);

    let state = State {
        filetype: InputCategory::classify_clipboard(),
        clipboard: get_clipboard(),
        config,
        actions,
        matcher: SkimMatcherV2::default(),
    };

    println!("{:?}", state.config);
}
#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn test_ron_deserialization() {
        // I've fixed the casing of PlainText and added missing commas in this string
        let content = r#"(
    prefix: "",
    max_entries: 10,
    actions: [
        (
            name: "Pretty Print",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | jq .",
        ),
        (
            name: "Convert to YAML",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | yq -p json -o yaml | wl-copy",
        ),
        (
            name: "Convert to XML",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | yq -p json -o xml | wl-copy",
        ),
        (
            name: "Convert to CSV",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' > /tmp/temp.json && ~/.config/shell/bin/cv /tmp/temp.json /tmp/temp.csv && wl-copy < /tmp/temp.csv && rm /tmp/temp.json /tmp/temp.csv",
        ),
        (
            name: "Sort Keys",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | jq -S .",
        ),
        (
            name: "Filter Nulls",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | jq 'del(..|nulls)'",
        ),
        (
            name: "Flatten",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | jq 'flatten'",
        ),
        (
            name: "Get Length",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | jq length",
        ),
        (
            name: "Extract Values",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | jq '.[]'",
        ),
        (
            name: "Base64 Encode",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | base64 | wl-copy",
        ),
        (
            name: "Base64 Decode",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | base64 -d",
        ),
        (
            name: "View in Neovim",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | jq . > /tmp/temp.json && kitty --detach nvim /tmp/temp.json",
        ),
        (
            name: "Escape String",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | jq -R . | wl-copy",
        ),
        (
            name: "Unescape String",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | jq -r . | wl-copy",
        ),
        (
            name: "Count Fields",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | jq 'keys | length'",
        ),
        (
            name: "To TOML",
            data_type: Code(lang: "json"),
            command: "echo '{clip}' | yq -p json -o toml | wl-copy",
        ),
        (
            name: "Format (Prettier)",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | prettier --stdin-filepath input.js | wl-copy",
        ),
        (
            name: "Count Lines",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | wc -l | wl-copy",
        ),
        (
            name: "Create Gist",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | gh gist create -",
        ),
        (
            name: "Open in Carbon",
            data_type: Code(lang: "all"),
            command: "xdg-open 'https://carbon.now.sh/?code={clip}' && sleep 0.5 && kdotool windowactivate $(kdotool search --name \"Firefox|Chrome|Brave|Chromium\" | head -n 1)",
        ),
        (
            name: "Calculate SHA256",
            data_type: Code(lang: "all"),
            command: "echo -n '{clip}' | sha256sum | awk '{print $1}' | wl-copy",
        ),
        (
            name: "Calculate MD5",
            data_type: Code(lang: "all"),
            command: "echo -n '{clip}' | md5sum | awk '{print $1}' | wl-copy",
        ),
        (
            name: "View in VSCode",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' > /tmp/temp_code.txt && code /tmp/temp_code.txt",
        ),
        (
            name: "Base64 Encode",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | base64 | wl-copy",
        ),
        (
            name: "Remove Comments",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | sed 's|//.*||g' | wl-copy",
        ),
        (
            name: "To One Line",
            data_type: PlainText,
            command: "echo '{clip}' | tr -d '\n' | wl-copy",
        ),
        (
            name: "To One Line",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | tr -d '\n' | wl-copy",
        ),
        (
            name: "Uppercase",
            data_type: PlainText,
            command: "echo '{clip}' | tr '[:lower:]' '[:upper:]' | wl-copy",
        ),
        (
            name: "Lowercase",
            data_type: PlainText,
            command: "echo '{clip}' | tr '[:upper:]' '[:lower:]' | wl-copy",
        ),
        (
            name: "Reverse Lines",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | tac | wl-copy",
        ),
        (
            name: "Dedup Lines",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | sort -u | wl-copy",
        ),
        (
            name: "Hex Dump",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | xxd",
        ),
        (
            name: "Extract Strings",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | strings",
        ),
        (
            name: "Check Syntax (JS)",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | node -c",
        ),
        (
            name: "Diff with Clipboard",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' > /tmp/diff_a && wl-paste > /tmp/diff_b && kitty --detach nvim -d /tmp/diff_a /tmp/diff_b",
        ),
        (
            name: "Save Snippet",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' >> ~/Documents/snippets.txt",
        ),
        (
            name: "Count Characters",
            data_type: Code(lang: "all"),
            command: "echo '{clip}' | wc -c",
        ),
        (
            name: "Generate QR Code",
            data_type: System(kind: "url"),
            command: "qrencode -s 10 -o /tmp/qr.png '{clip}' && wl-copy < /tmp/qr.png && gwenview /tmp/qr.png",
        ),
        (
            name: "Open with mpv",
            data_type: System(kind: "url"),
            command: "mpv '{clip}'",
        ),
        (
            name: "Open in Browser",
            data_type: System(kind: "url"),
            command: "xdg-open '{clip}' && sleep 0.5 && kdotool windowactivate $(kdotool search --name \"Firefox|Chrome|Brave|Chromium\" | head -n 1)",
        ),
        (
            name: "Open Browser Private",
            data_type: System(kind: "url"),
            command: "google-chrome --incognito '{clip}'",
        ),
        (
            name: "Fetch Page Title",
            data_type: System(kind: "url"),
            command: "curl -s '{clip}' | grep -oP '(?<=<title>).*?(?=</title>)'",
        ),
        (
            name: "Shorten URL",
            data_type: System(kind: "url"),
            command: "curl -s 'https://is.gd/create.php?format=simple&url={clip}' | wl-copy && notify-send 'URL Shortened' 'Copied to clipboard'",
        ),
        (
            name: "Download (wget)",
            data_type: System(kind: "url"),
            command: "wget '{clip}'",
        ),
        (
            name: "Download (curl)",
            data_type: System(kind: "url"),
            command: "curl -O '{clip}'",
        ),
        (
            name: "Check Headers",
            data_type: System(kind: "url"),
            command: "curl -I '{clip}'",
        ),
        (
            name: "Whois",
            data_type: System(kind: "url"),
            command: "whois '{clip}'",
        ),
        (
            name: "Trace Route",
            data_type: System(kind: "url"),
            command: "traceroute '{clip}'",
        ),
        (
            name: "SSL Check",
            data_type: System(kind: "url"),
            command: "xdg-open 'https://www.ssllabs.com/ssltest/analyze.html?d={clip}' && sleep 0.5 && kdotool windowactivate $(kdotool search --name \"Firefox|Chrome|Brave|Chromium\" | head -n 1)",
        ),
        (
            name: "Get Base Domain",
            data_type: System(kind: "url"),
            command: "echo '{clip}' | awk -F/ '{print $3}' | wl-copy",
        ),
        (
            name: "URL Encode",
            data_type: System(kind: "url"),
            command: "python3 -c \"import urllib.parse; print(urllib.parse.quote('{clip}'))\" | wl-copy",
        ),
        (
            name: "URL Decode",
            data_type: System(kind: "url"),
            command: "python3 -c \"import urllib.parse; print(urllib.parse.unquote('{clip}'))\" | wl-copy",
        ),
        (
            name: "View Source",
            data_type: System(kind: "url"),
            command: "curl '{clip}' | bat -l html",
        ),
        (
            name: "Dump Text",
            data_type: System(kind: "url"),
            command: "lynx -dump '{clip}'",
        ),
        (
            name: "Cast to Device",
            data_type: System(kind: "url"),
            command: "catt cast '{clip}'",
        ),
        (
            name: "Reveal in FileManager",
            data_type: System(kind: "path"),
            command: "xdg-open \"$(dirname '{clip}')\"",
        ),
        (
            name: "Copy Absolute Path",
            data_type: System(kind: "path"),
            command: "realpath '{clip}' | wl-copy",
        ),
        (
            name: "Copy Content",
            data_type: System(kind: "path"),
            command: "cat '{clip}' | wl-copy",
        ),
        (
            name: "Delete (Trash)",
            data_type: System(kind: "path"),
            command: "gio trash '{clip}'",
        ),
        (
            name: "Rename",
            data_type: System(kind: "path"),
            command: "mv '{clip}'",
        ),
        (
            name: "Duplicate",
            data_type: System(kind: "path"),
            command: "cp '{clip}' '{clip}.bak'",
        ),
        (
            name: "Get Size",
            data_type: System(kind: "path"),
            command: "du -h '{clip}'",
        ),
        (
            name: "Get MIME Type",
            data_type: System(kind: "path"),
            command: "file --mime-type -b '{clip}'",
        ),
        (
            name: "Check Permissions",
            data_type: System(kind: "path"),
            command: "stat -c '%A' '{clip}'",
        ),
        (
            name: "Make Executable",
            data_type: System(kind: "path"),
            command: "chmod +x '{clip}'",
        ),
        (
            name: "Zip File",
            data_type: System(kind: "path"),
            command: "zip '{clip}.zip' '{clip}'",
        ),
        (
            name: "Create Tarball",
            data_type: System(kind: "path"),
            command: "tar -czf '{clip}.tar.gz' '{clip}'",
        ),
        (
            name: "Edit in VSCode",
            data_type: System(kind: "path"),
            command: "code '{clip}'",
        ),
        (
            name: "Edit in Nano",
            data_type: System(kind: "path"),
            command: "konsole -e nano '{clip}'",
        ),
        (
            name: "MD5 Checksum",
            data_type: System(kind: "path"),
            command: "md5sum '{clip}'",
        ),
        (
            name: "SHA256 Checksum",
            data_type: System(kind: "path"),
            command: "sha256sum '{clip}'",
        ),
        (
            name: "Cat File",
            data_type: System(kind: "path"),
            command: "cat '{clip}'",
        ),
        (
            name: "Tail File",
            data_type: System(kind: "path"),
            command: "konsole -e tail -f '{clip}'",
        ),
        (
            name: "Count Lines",
            data_type: System(kind: "path"),
            command: "wc -l '{clip}'",
        ),
        (
            name: "Copy Hex",
            data_type: Design(kind: "Color"),
            command: "echo '{clip}' | wl-copy",
        ),
        (
            name: "Preview Color",
            data_type: Design(kind: "Color"),
            command: "convert -size 100x100 xc:'{clip}' /tmp/color_preview.png && xdg-open /tmp/color_preview.png",
        ),
        (
            name: "To RGB",
            data_type: Design(kind: "Color"),
            command: "python3 -c \"import matplotlib.colors; print(matplotlib.colors.to_rgb('{clip}'))\" | wl-copy",
        ),
        (
            name: "To HSL",
            data_type: Design(kind: "Color"),
            command: "convert xc:'{clip}' -colorspace HSL -format '%[pixel:p{0,0}]' info:",
        ),
        (
            name: "Invert Color",
            data_type: Design(kind: "Color"),
            command: "convert xc:'{clip}' -channel RGB -negate -format '%[pixel:p{0,0}]' info:",
        ),
        (
            name: "Darken",
            data_type: Design(kind: "Color"),
            command: "convert xc:'{clip}' -modulate 80,100,100 -format '%[pixel:p{0,0}]' info:",
        ),
        (
            name: "Lighten",
            data_type: Design(kind: "Color"),
            command: "convert xc:'{clip}' -modulate 120,100,100 -format '%[pixel:p{0,0}]' info:",
        ),
        (
            name: "Color-Hex.com",
            data_type: Design(kind: "Color"),
            command: "xdg-open 'https://www.color-hex.com/color/{clip}' && sleep 0.5 && kdotool windowactivate $(kdotool search --name \"Firefox|Chrome|Brave|Chromium\" | head -n 1)",
        ),
        (
            name: "Adobe Color",
            data_type: Design(kind: "Color"),
            command: "xdg-open 'https://color.adobe.com/' && sleep 0.5 && kdotool windowactivate $(kdotool search --name \"Firefox|Chrome|Brave|Chromium\" | head -n 1)",
        ),
        (
            name: "To CMYK",
            data_type: Design(kind: "Color"),
            command: "convert xc:'{clip}' -colorspace CMYK -format '%[pixel:p{0,0}]' info:",
        ),
        (
            name: "To CSS Var",
            data_type: Design(kind: "Color"),
            command: "echo '--color: {clip};' | wl-copy",
        ),
        (
            name: "To Swift UIColor",
            data_type: Design(kind: "Color"),
            command: "echo 'UIColor(hex: \"{clip}\")' | wl-copy",
        ),
        (
            name: "To Android Color",
            data_type: Design(kind: "Color"),
            command: "echo 'Color.parseColor(\"{clip}\")' | wl-copy",
        ),
        (
            name: "Grayscale",
            data_type: Design(kind: "Color"),
            command: "convert xc:'{clip}' -colorspace Gray -format '%[pixel:p{0,0}]' info:",
        ),
        (
            name: "Saturate",
            data_type: Design(kind: "Color"),
            command: "convert xc:'{clip}' -modulate 100,150,100 -format '%[pixel:p{0,0}]' info:",
        ),
        (
            name: "Desaturate",
            data_type: Design(kind: "Color"),
            command: "convert xc:'{clip}' -modulate 100,50,100 -format '%[pixel:p{0,0}]' info:",
        ),
        (
            name: "Red Channel",
            data_type: Design(kind: "Color"),
            command: "echo '{clip}' | cut -c 2-3",
        ),
        (
            name: "Green Channel",
            data_type: Design(kind: "Color"),
            command: "echo '{clip}' | cut -c 4-5",
        ),
        (
            name: "Blue Channel",
            data_type: Design(kind: "Color"),
            command: "echo '{clip}' | cut -c 6-7",
        ),
        (
            name: "Search Google",
            data_type: PlainText,
            command: "xdg-open 'https://www.google.com/search?q={clip}' && sleep 0.5 && kdotool windowactivate $(kdotool search --name \"Firefox|Chrome|Brave|Chromium\" | head -n 1)",
        ),
        (
            name: "Search Google Len",
            data_type: Image,
            command: "rust-script $HOME/.config/shell/scripts/google-len-search.rs",
        ),
        (
            name: "Search YouTube",
            data_type: Plaintext,
            command: "xdg-open 'https://www.youtube.com/results?search_query={clip}' && sleep 0.5 && kdotool windowactivate $(kdotool search --name \"Firefox|Chrome|Brave|Chromium\" | head -n 1)",
        ),
        (
            name: "Word Count",
            data_type: Plaintext,
            command: "count=$(echo '{clip}' | wc -w); notify-send 'Word Count' \"There are $count words in your clipboard.\" -i edit-find",
        ),
        (
            name: "Character Count",
            data_type: Plaintext,
            command: "count=$(echo -n '{clip}' | wc -c); notify-send 'Character Count' \"Total characters: $count\"",
        ),
        (
            name: "KebabCase",
            data_type: Plaintext,
            command: "sh $HOME/.config/anyrun/ua/to_kebabcase.sh '{clip}'",
        ),
        (
            name: "CamelCase",
            data_type: Plaintext,
            command: "sh $HOME/.config/anyrun/ua/to_camelcase.sh '{clip}'",
        ),
        (
            name: "SnakeCase",
            data_type: Plaintext,
            command: "sh $HOME/.config/anyrun/ua/to_snakecase.sh '{clip}'",
        ),
        (
            name: "UpperCase",
            data_type: Plaintext,
            command: "echo '{clip}' | tr '[:lower:]' '[:upper:]' | wl-copy",
        ),
        (
            name: "Open in readtext",
            data_type: Plaintext,
            command: "readtext '{clip}'&",
        ),
        (
            name: "Lowercase",
            data_type: Plaintext,
            command: "echo '{clip}' | tr '[:upper:]' '[:lower:]' | wl-copy",
        ),
        (
            name: "Save to Notes",
            data_type: Plaintext,
            command: "echo '{clip}' >> ~/Desktop/notes.md",
        ),
        (
            name: "Reverse Text",
            data_type: Plaintext,
            command: "echo '{clip}' | rev | wl-copy",
        ),
        (
            name: "Base64 Encode",
            data_type: Plaintext,
            command: "echo '{clip}' | base64 | wl-copy",
        ),
        (
            name: "Gemini - find in internet",
            data_type: Plaintext,
            command: "wl-paste | rust-script $HOME/.config/shell/scripts/gemini-trigger.rs find",
        ),
        (
            name: "Gemini - find in internet",
            data_type: Code(lang: "all"),
            command: "wl-paste | rust-script $HOME/.config/shell/scripts/gemini-trigger.rs find",
        ),
        (
            name: "Gemini",
            data_type: Code(lang: "all"),
            command: "wl-paste | rust-script $HOME/.config/shell/scripts/gemini-trigger.rs",
        ),
        (
            name: "Gemini",
            data_type: Plaintext,
            command: "wl-paste | rust-script $HOME/.config/shell/scripts/gemini-trigger.rs",
        ),
        (
            name: "Gemini - markdown format",
            data_type: Plaintext,
            command: "wl-paste | rust-script $HOME/.config/shell/scripts/gemini-trigger.rs format",
        ),
        (
            name: "Gemini - markdown format",
            data_type: Code(lang: "all"),
            command: "wl-paste | rust-script $HOME/.config/shell/scripts/gemini-trigger.rs format",
        ),
        (
            name: "QR Code (Gwenview)",
            data_type: Plaintext,
            command: "qrencode -o /tmp/qr.png '{clip}' && gwenview /tmp/qr.png",
        ),
        (
            name: "Remove Whitespace",
            data_type: Plaintext,
            command: "echo '{clip}' | tr -d ' ' | wl-copy",
        ),
        (
            name: "Color Notify",
            data_type: Plaintext,
            command: "rust-script $HOME/.config/anyrun/ua/color-notify.rs '{clip}'",
        ),
        (
            name: "Home dir",
            data_type: Plaintext,
            command: "sh $HOME/.config/anyrun/ua/to_homedir.sh '{clip}'",
        ),
        (
            name: "Nvim Edit Clipboard",
            data_type: Code(lang: "all"),
            command: "sh $HOME/.config/anyrun/ua/nvim_edit_clipboard.sh",
        ),
        (
            name: "Nvim Edit Clipboard",
            data_type: Plaintext,
            command: "sh $HOME/.config/anyrun/ua/nvim_edit_clipboard.sh",
        ),
        (
            name: "MarkDown Preview",
            data_type: Plaintext,
            command: "sh $HOME/.config/anyrun/ua/markdown_preview.sh",
        ),
    ],
)"#;

        let result: Result<Config, ron::error::SpannedError> = ron::from_str(content);

        match result {
            Ok(config) => {
                assert_eq!(config.max_entries, 10);
                assert!(!config.actions.is_empty());
                println!("Config parsed successfully!");
            }
            Err(e) => {
                // ron::de::SpannedError implement Display, nó sẽ tự in ra: "line: X, col: Y: message"
                eprintln!("Cấu hình lỗi tại: {}", e);

                // Hoặc in chi tiết hơn nếu bạn muốn can thiệp sâu:
                let line = e.span.start.line;
                let col = e.span.start.col;
                eprintln!("Lỗi cụ thể tại Dòng {}, Cột {}: {:?}", line, col, e.code);

                panic!("RON Parsing failed!");
            }
        }
    }
}
