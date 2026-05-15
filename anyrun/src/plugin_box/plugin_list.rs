pub fn plugin_type_label(plugin_name: &str) -> &'static str {
    match plugin_name {
        "Applications" => "App",
        "KDE Settings" | "Bluetooth Control" => "Settings",
        "Shell Wrapper" | "Shell Wrapper Once" | "Calc" | "Universal Action" | "Sync Manager" => {
            "Action"
        }
        "Browser Tabs" | "Web Search" => "Web",
        "Find Files" | "Zoxide Fuzzy" => "File",
        "KDE Klipper" => "Clipboard",
        "Translate" => "Translate",
        "Symbols" => "Symbol",
        _ => "Result",
    }
}
