use std::process::Command;

use anyrun_helper::window::{WindowBackend, WindowInfo};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct TreeNode {
    id: i64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    app_id: Option<String>,
    #[serde(default)]
    window: Option<i64>,
    #[serde(default)]
    window_properties: Option<WindowProperties>,
    #[serde(default)]
    nodes: Vec<TreeNode>,
    #[serde(default)]
    floating_nodes: Vec<TreeNode>,
    #[serde(default, rename = "type")]
    node_type: Option<String>,
    #[serde(default)]
    num: Option<i64>,
}

#[derive(Deserialize, Debug)]
struct WindowProperties {
    #[serde(default)]
    class: Option<String>,
    #[serde(default)]
    title: Option<String>,
}

pub struct I3Backend;

fn run_i3_msg(args: &[&str]) -> Option<String> {
    let output = Command::new("i3-msg").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn resolve_app(
    entries: &[freedesktop_desktop_entry::DesktopEntry],
    locales: &[impl AsRef<str>],
    app_id: &str,
) -> (Option<String>, Option<String>, Option<String>) {
    let entry = freedesktop_desktop_entry::find_app_by_id(
        entries,
        freedesktop_desktop_entry::unicase::Ascii::new(app_id),
    );
    match entry {
        Some(e) => (
            e.name(locales).map(|n| n.to_string()),
            e.icon().map(|i| i.to_string()),
            Some(app_id.to_string()),
        ),
        None => (None, None, Some(app_id.to_string())),
    }
}

fn collect_windows(
    node: &TreeNode,
    workspace: Option<String>,
    entries: &[freedesktop_desktop_entry::DesktopEntry],
    locales: &[impl AsRef<str>],
    out: &mut Vec<WindowInfo>,
) {
    let mut current_workspace = workspace;
    if node.node_type.as_deref() == Some("workspace") {
        current_workspace = Some(
            node.name
                .clone()
                .unwrap_or_else(|| format!("Workspace {}", node.num.unwrap_or_default())),
        );
    }

    let app_id = node.app_id.clone().or_else(|| {
        node.window_properties
            .as_ref()
            .and_then(|p| p.class.clone())
    });
    let title = node
        .window_properties
        .as_ref()
        .and_then(|p| p.title.clone())
        .or_else(|| node.name.clone())
        .unwrap_or_default();

    if node.window.is_some() || app_id.is_some() {
        let (app_name, icon, app_id) = app_id
            .as_deref()
            .map(|id| resolve_app(entries, locales, id))
            .unwrap_or((None, None, None));

        out.push(WindowInfo {
            id: node.id.to_string(),
            title,
            app_id,
            workspace: current_workspace.clone(),
            icon,
            app_name,
        });
    }

    for child in &node.nodes {
        collect_windows(child, current_workspace.clone(), entries, locales, out);
    }
    for child in &node.floating_nodes {
        collect_windows(child, current_workspace.clone(), entries, locales, out);
    }
}

fn parse_windows(tree_json: &str) -> Vec<WindowInfo> {
    let root: TreeNode = match serde_json::from_str(tree_json) {
        Ok(node) => node,
        Err(e) => {
            eprintln!("[window-switcher] Failed to parse i3 tree output: {e}");
            return Vec::new();
        }
    };

    let locales = freedesktop_desktop_entry::get_languages_from_env();
    let entries = freedesktop_desktop_entry::desktop_entries(&locales);

    let mut results = Vec::new();
    collect_windows(&root, None, &entries, &locales, &mut results);
    results
}

impl WindowBackend for I3Backend {
    fn list_windows(&self) -> Vec<WindowInfo> {
        let tree = match run_i3_msg(&["-t", "get_tree"]) {
            Some(tree) => tree,
            None => return Vec::new(),
        };
        parse_windows(&tree)
    }

    fn focus_window(&self, id: &str) -> Result<(), String> {
        let selector = format!("[con_id={id}] focus");
        let output = Command::new("i3-msg")
            .args(["-q", &selector])
            .output()
            .map_err(|e| format!("Failed to run i3-msg: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    fn name(&self) -> &'static str {
        "i3"
    }
}

#[cfg(test)]
mod tests {
    use super::parse_windows;

    #[test]
    fn parse_i3_tree_windows_with_workspace() {
        let json = r#"{
            "id": 1,
            "name": "root",
            "nodes": [{
                "id": 10,
                "type": "workspace",
                "name": "2:web",
                "num": 2,
                "nodes": [{
                    "id": 42,
                    "name": "Mozilla Firefox",
                    "window": 420,
                    "window_properties": {"class": "firefox", "title": "Docs"},
                    "nodes": [],
                    "floating_nodes": []
                }],
                "floating_nodes": []
            }],
            "floating_nodes": []
        }"#;

        let windows = parse_windows(json);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].id, "42");
        assert_eq!(windows[0].title, "Docs");
        assert_eq!(windows[0].workspace.as_deref(), Some("2:web"));
    }
}
