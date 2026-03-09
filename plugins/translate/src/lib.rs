use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use tokio::runtime::Runtime;

const GOOGLE_TRANSLATE_API: &str = "https://translate.googleapis.com/translate_a/single?client=gtx";

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

struct Language {
    code: &'static str,
    name: &'static str,
}

struct State {
    config: Config,
    client: Client,
    runtime: Runtime,
    languages: Vec<Language>,
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = format!("{}/translate.ron", config_dir);
    let config = fs::read_to_string(config_path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_default();

    State {
        config,
        client: Client::new(),
        runtime: Runtime::new().expect("Failed to create tokio runtime"),
        languages: get_language_list(),
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

    let query = &input[state.config.prefix.len()..];
    let (target_part, text) = match query.split_once(' ') {
        Some(parts) => parts,
        None => return RVec::new(),
    };

    if text.is_empty() {
        return RVec::new();
    }

    let (source_filter, dest_filter) =
        parse_language_identifiers(target_part, &state.config.language_delimiter);
    let candidate_pairs = resolve_language_pairs(&state.languages, source_filter, dest_filter);

    state.runtime.block_on(async move {
        let requests = candidate_pairs.into_iter().map(|(src, dest)| {
            let url = build_api_url(src.map(|l| l.code), dest.code, text);
            let client = &state.client;
            let dest_name = dest.name;
            async move {
                let response = client.get(url).send().await.ok()?;
                let json: Value = response.json().await.ok()?;
                parse_translation_match(json, dest_name, &state.languages)
            }
        });

        futures::future::join_all(requests)
            .await
            .into_iter()
            .flatten()
            .collect::<RVec<_>>()
    })
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    HandleResult::Copy(selection.title.into_bytes())
}

fn parse_language_identifiers<'a>(input: &'a str, delimiter: &str) -> (Option<&'a str>, &'a str) {
    match input.split_once(delimiter) {
        Some((src, dest)) => (Some(src), dest),
        None => (None, input),
    }
}

fn resolve_language_pairs<'a>(
    registry: &'a [Language],
    src_query: Option<&str>,
    dest_query: &str,
) -> Vec<(Option<&'a Language>, &'a Language)> {
    let dest_matches = filter_languages(registry, dest_query);

    if let Some(src_q) = src_query {
        let src_matches = filter_languages(registry, src_q);
        src_matches
            .into_iter()
            .flat_map(|s| dest_matches.iter().map(move |&d| (Some(s), d)))
            .collect()
    } else {
        dest_matches.into_iter().map(|d| (None, d)).collect()
    }
}

fn filter_languages<'a>(languages: &'a [Language], query: &str) -> Vec<&'a Language> {
    let query_lower = query.to_lowercase();
    languages
        .iter()
        .filter(|l| {
            l.code.to_lowercase().starts_with(&query_lower)
                || l.name.to_lowercase().starts_with(&query_lower)
        })
        .collect()
}

fn build_api_url(src_code: Option<&str>, dest_code: &str, text: &str) -> String {
    let sl = src_code.unwrap_or("auto");
    // dt=t: translation
    // dt=at: alternative translations
    // dt=bd: dictionary (parts of speech)
    // dt=rm: transliteration/pronunciation
    format!(
        "{}&sl={}&tl={}&dt=t&dt=at&dt=bd&dt=rm&q={}",
        GOOGLE_TRANSLATE_API,
        sl,
        dest_code,
        urlencoding::encode(text)
    )
}

fn parse_translation_match(json: Value, dest_name: &str, registry: &[Language]) -> Option<Match> {
    // 1. Extract the main translation text
    let translation = json[0]
        .as_array()?
        .iter()
        .filter_map(|segment| segment[0].as_str())
        .collect::<Vec<_>>()
        .join("");

    let pronunciation = json[0]
        .as_array()
        .and_then(|arr| arr.last())
        .and_then(|last| last[3].as_str())
        .map(|s| format!(" [{}]", s))
        .unwrap_or_default();

    let word_types = json[1]
        .as_array()
        .map(|types| {
            types
                .iter()
                .filter_map(|t| t[0].as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty())
        .map(|s| format!(" ({})", s))
        .unwrap_or_default();

    let detected_src_code = json[2].as_str().unwrap_or("?");
    let src_name = registry
        .iter()
        .find(|l| l.code == detected_src_code)
        .map(|l| l.name)
        .unwrap_or(detected_src_code);

    Some(Match {
        title: translation.into(),
        description: ROption::RSome(
            format!(
                "{} → {} • {} {}  ",
                src_name, dest_name, pronunciation, word_types,
            )
            .into(),
        ),
        use_pango: false,
        icon: ROption::RNone,
        id: ROption::RNone,
    })
}

fn get_language_list() -> Vec<Language> {
    vec![
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
        ("zh-CN", "Chinese (Simp)"),
        ("zh-TW", "Chinese (Trad)"),
        ("co", "Corsican"),
        ("hr", "Croatian"),
        ("cs", "Czech"),
        ("da", "Danish"),
        ("nl", "Dutch"),
        ("en", "English"),
        ("eo", "Esperanto"),
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
        ("ku", "Kurdish"),
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
        ("my", "Burmese"),
        ("ne", "Nepali"),
        ("no", "Norwegian"),
        ("ps", "Pashto"),
        ("fa", "Persian"),
        ("pl", "Polish"),
        ("pt", "Portuguese"),
        ("pa", "Punjabi"),
        ("ro", "Romanian"),
        ("ru", "Russian"),
        ("sm", "Samoan"),
        ("gd", "Scots Gaelic"),
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
    ]
    .into_iter()
    .map(|(code, name)| Language { code, name })
    .collect()
}
