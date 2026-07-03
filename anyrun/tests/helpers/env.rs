use std::collections::HashMap;

pub fn headless_env_map() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("GDK_BACKEND".into(), "headless".into());
    env.insert("GSK_RENDERER".into(), "cairo".into());
    env.insert("GTK_A11Y".into(), "none".into());
    env.insert("NO_AT_BRIDGE".into(), "1".into());
    env
}

pub fn set_headless_env() {
    std::env::set_var("GDK_BACKEND", "headless");
    std::env::set_var("GSK_RENDERER", "cairo");
    std::env::set_var("GTK_A11Y", "none");
    std::env::set_var("NO_AT_BRIDGE", "1");
}
