use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub prefix: String,
    pub max_entries: usize,
    pub cache_ttl_secs: u64,
    pub exclude_classes: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "w ".into(),
            max_entries: 15,
            cache_ttl_secs: 5,
            exclude_classes: vec!["plasmashell".into()],
        }
    }
}
