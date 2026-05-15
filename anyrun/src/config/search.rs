use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct PrefixRoute {
    pub prefix: String,
    pub plugins: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub enum TypingVisual {
    #[default]
    DimPrevious,
    KeepPrevious,
    Clear,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct SearchUxConfig {
    pub settle_delay_ms: u64,
    pub flush_delay_ms: u64,
    pub typing_visual: TypingVisual,
    pub bare_text_fast_lane: Vec<String>,
    pub prefix_routes: Vec<PrefixRoute>,
}

impl Default for SearchUxConfig {
    fn default() -> Self {
        Self {
            settle_delay_ms: 200,
            flush_delay_ms: 50,
            typing_visual: TypingVisual::DimPrevious,
            bare_text_fast_lane: Vec::new(),
            prefix_routes: Vec::new(),
        }
    }
}
