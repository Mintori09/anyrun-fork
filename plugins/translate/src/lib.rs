use std::fs;

use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use reqwest::Client;
use serde::Deserialize;
use tokio::runtime::Runtime;

#[derive(Deserialize)]
struct Config {
    prefix: String,
    language_delimiter: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: ":".to_string(),
            language_delimiter: ">".to_string(),
        }
    }
}

struct State {
    config: Config,
    client: Client,
    runtime: Runtime,
    langs: Vec<(&'static str, &'static str)>,
}

#[init]
fn init(config_dir: RString) -> State {
    State {
        config: match fs::read_to_string(format!("{}/translate.ron", config_dir)) {
            Ok(content) => ron::from_str(&content).unwrap_or_default(),
            Err(_) => Config::default(),
        },
        client: Client::new(),
        runtime: Runtime::new().expect("Failed to create tokio runtime"),
        langs: vec![
            ("af", "Afrikaans"),
            ("sq", "Albanian"),
            ("am", "Amharic"),
            ("ar", "Arabic"),
            ("hy", "Armenian"),
            ("az", "Azerbaijani"),
            ("eu", "Basque"),
            ("be", "Belarusian"),
            ("bn", "Bengali"),
            ("bs", "Bosnian"),
            ("bg", "Bulgarian"),
            ("ca", "Catalan"),
            ("ceb", "Cebuano"),
            ("ny", "Chichewa"),
            ("zh-CN", "Chinese-(Simplified)"),
            ("zh-TW", "Chinese-(Traditional)"),
            ("co", "Corsican"),
            ("hr", "Croatian"),
            ("cs", "Czech"),
            ("da", "Danish"),
            ("nl", "Dutch"),
            ("en", "English"),
            ("eo", "Esperanto"),
            ("ee", "Ewe"),
            ("et", "Estonian"),
            ("tl", "Filipino"),
            ("fi", "Finnish"),
            ("fr", "French"),
            ("fy", "Frisian"),
            ("gl", "Galician"),
            ("ka", "Georgian"),
            ("de", "German"),
            ("el", "Greek"),
            ("gu", "Gujarati"),
            ("ht", "Haitian Creole"),
            ("ha", "Hausa"),
            ("haw", "Hawaiian"),
            ("iw", "Hebrew"),
            ("hi", "Hindi"),
            ("hmn", "Hmong"),
            ("hu", "Hungarian"),
            ("is", "Icelandic"),
            ("ig", "Igbo"),
            ("id", "Indonesian"),
            ("ga", "Irish"),
            ("it", "Italian"),
            ("ja", "Japanese"),
            ("jw", "Javanese"),
            ("kn", "Kannada"),
            ("kk", "Kazakh"),
            ("km", "Khmer"),
            ("ko", "Korean"),
            ("ku", "Kurdish-(Kurmanji)"),
            ("ky", "Kyrgyz"),
            ("lo", "Lao"),
            ("la", "Latin"),
            ("lv", "Latvian"),
            ("lt", "Lithuanian"),
            ("lb", "Luxembourgish"),
            ("mk", "Macedonian"),
            ("mg", "Malagasy"),
            ("ms", "Malay"),
            ("ml", "Malayalam"),
            ("mt", "Maltese"),
            ("mi", "Maori"),
            ("mr", "Marathi"),
            ("mn", "Mongolian"),
            ("my", "Myanmar-(Burmese)"),
            ("ne", "Nepali"),
            ("no", "Norwegian"),
            ("ps", "Pashto"),
            ("fa", "Persian"),
            ("pl", "Polish"),
            ("pt", "Portuguese"),
            ("ma", "Punjabi"),
            ("ro", "Romanian"),
            ("ru", "Russian"),
            ("sm", "Samoan"),
            ("gd", "Scots-Gaelic"),
            ("sr", "Serbian"),
            ("st", "Sesotho"),
            ("sn", "Shona"),
            ("sd", "Sindhi"),
            ("si", "Sinhala"),
            ("sk", "Slovak"),
            ("sl", "Slovenian"),
            ("so", "Somali"),
            ("es", "Spanish"),
            ("su", "Sundanese"),
            ("sw", "Swahili"),
            ("sv", "Swedish"),
            ("tg", "Tajik"),
            ("ta", "Tamil"),
            ("te", "Telugu"),
            ("th", "Thai"),
            ("tr", "Turkish"),
            ("uk", "Ukrainian"),
            ("ur", "Urdu"),
            ("uz", "Uzbek"),
            ("vi", "Vietnamese"),
            ("cy", "Welsh"),
            ("xh", "Xhosa"),
            ("yi", "Yiddish"),
            ("yo", "Yoruba"),
            ("zu", "Zulu"),
        ],
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Translate".into(),
        icon: "preferences-desktop-locale".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    if !input.starts_with(&state.config.prefix) {
        return RVec::new();
    }

    let input = &input[state.config.prefix.len()..];
    let (lang_split, text) = match input.split_once(' ') {
        Some(split) => split,
        None => return RVec::new(),
    };

    let (src, dest) = match lang_split.split_once(&state.config.language_delimiter) {
        Some(split) => (Some(split.0), split.1),
        None => (None, lang_split),
    };

    if text.is_empty() {
        return RVec::new();
    }

let dest_lower = dest.to_lowercase();
let src_lower = src.as_ref().map(|s| s.to_lowercase());

let dest_matches = state
    .langs
    .iter() // Iterate by reference to avoid unnecessary cloning
    .filter(|(code, name)| {
        code.to_lowercase().starts_with(&dest_lower) || 
        name.to_lowercase().starts_with(&dest_lower)
    })
    .collect::<Vec<_>>();

let  matches = match src_lower {
    Some(s_lower) => {
        let src_matches = state
            .langs
            .iter()
            .filter(|(code, name)| {
                code.to_lowercase().starts_with(&s_lower) || 
                name.to_lowercase().starts_with(&s_lower)
            })
            .collect::<Vec<_>>();

        src_matches
            .into_iter()
            .flat_map(|src_lang| {
                dest_matches
                    .iter()
                    .map(move |&dest_lang| (Some(src_lang), dest_lang))
            })
            .collect::<Vec<_>>()
    }
    None => {
        dest_matches
            .into_iter()
            .map(|dest_lang| (None, dest_lang))
            .collect::<Vec<_>>()
    }
};


    state.runtime.block_on(async move {
        // Create the futures for fetching the translation results
        let futures = matches
            .into_iter()
            .map(|(src, dest)| async move {
                match src {
                    Some(src) => 
                (dest.1, state.client.get(format!("https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}", src.0, dest.0, text)).send().await),
                    None => (dest.1, state.client.get(format!("https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl={}&dt=t&q={}", dest.0, text)).send().await)
                }
            });
       
        let res = futures::future::join_all(futures) 
            .await;

        res
            .into_iter()
            .filter_map(|(name, res)| res
                .ok()
                .map(|response| futures::executor::block_on(response.json())
                    .ok()
                    .map(|json: serde_json::Value|
                        Match {
                            title: json[0]
                                .as_array()
                                .expect("Malformed JSON!")
                                .iter()
                                .map(|val| val.as_array().expect("Malformed JSON!")[0].as_str()
                                    .expect("Malformed JSON!")
                                ).collect::<Vec<_>>()
                                .join(" ")
                                .into(),
                            description: ROption::RSome(
                                format!(
                                    "{} -> {}",
                                    state.langs.iter()
                                    .find_map(|(code, name)| if *code == json[2].as_str().expect("Malformed JSON!") {
                                            Some(*name)
                                        } else {
                                            None
                                    }).unwrap_or_else(|| json[2].as_str().expect("Malformed JSON!")),
                                    name)
                                .into()),
                            use_pango: false,
                            icon: ROption::RNone,
                            id: ROption::RNone
                        }
                    )
                )
            ).flatten().collect::<RVec<_>>()
    })
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    HandleResult::Copy(selection.title.into_bytes())
}
