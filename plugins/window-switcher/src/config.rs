use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub prefix: String,
    pub max_entries: usize,
    pub show_results_immediately: bool,
    pub cache_ttl_secs: u64,
    pub exclude_classes: Vec<String>,
    pub backend: Option<String>,
    pub backend_probe_order: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "".into(),
            max_entries: 15,
            show_results_immediately: true,
            cache_ttl_secs: 5,
            exclude_classes: vec!["plasmashell".into()],
            backend: None,
            backend_probe_order: crate::backends::default_probe_order(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn config_defaults_include_backend_fields() {
        let cfg = Config::default();
        assert!(cfg.backend.is_none());
        assert_eq!(cfg.backend_probe_order[0], "kwin");
        assert!(cfg.backend_probe_order.iter().any(|b| b == "gnome"));
    }

    #[test]
    fn config_parses_backend_fields() {
        let cfg: Config = ron::from_str(
            r#"Config(
                backend: Some("sway"),
                backend_probe_order: ["gnome", "sway"],
            )"#,
        )
        .expect("valid config");

        assert_eq!(cfg.backend.as_deref(), Some("sway"));
        assert_eq!(cfg.backend_probe_order, vec!["gnome", "sway"]);
    }
}
