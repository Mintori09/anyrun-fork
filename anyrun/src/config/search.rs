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
    pub plugin_timeout_ms: u64,
    pub slow_plugin_ms: u64,
    pub prefix_discovery_trigger: String,
    pub empty_state: EmptyStateConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct EmptyStateConfig {
    pub enabled: bool,
    pub recent_limit: usize,
}

impl Default for EmptyStateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            recent_limit: 8,
        }
    }
}

impl Default for SearchUxConfig {
    fn default() -> Self {
        Self {
            settle_delay_ms: 200,
            flush_delay_ms: 50,
            typing_visual: TypingVisual::DimPrevious,
            bare_text_fast_lane: Vec::new(),
            prefix_routes: Vec::new(),
            plugin_timeout_ms: 800,
            slow_plugin_ms: 250,
            prefix_discovery_trigger: "?".to_string(),
            empty_state: EmptyStateConfig::default(),
        }
    }
}
