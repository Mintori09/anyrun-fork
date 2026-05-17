use super::{App, AppWidgets};
use crate::plugin_box::plugin_type_label;
use gtk::prelude::*;
use gtk4 as gtk;
use std::hash::{Hash, Hasher};

impl App {
    pub(super) fn handle_pending_matches_flush(
        &mut self,
        widgets: &mut AppWidgets,
        root: &gtk::Window,
    ) {
        if self.pending_matches.is_empty() {
            return;
        }

        // Compute hash of incoming results; skip flush if unchanged
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        for (name, ms) in &self.pending_matches {
            name.hash(&mut hasher);
            for m in ms.iter() {
                m.title.hash(&mut hasher);
            }
        }
        let hash = hasher.finish();
        if hash == self.last_flush_hash {
            return;
        }
        self.last_flush_hash = hash;

        let pending = &self.pending_matches;

        let plugin_names: Vec<String> = self
            .plugin_names
            .iter()
            .filter(|name| pending.contains_key(*name))
            .cloned()
            .collect();

        struct PluginEntry {
            name: String,
            matches: Vec<anyrun_interface::Match>,
        }

        let entries: Vec<PluginEntry> = plugin_names
            .iter()
            .filter_map(|name| {
                pending.get(name).map(|ms| PluginEntry {
                    name: name.clone(),
                    matches: ms.iter().cloned().collect(),
                })
            })
            .collect();

        // Group by plugin: prefix-matched first, then the rest
        let interleaved: Vec<(anyrun_interface::Match, String)> = {
            let mut result = Vec::new();

            if let Some(prefix_set) = self.partial_prefix_plugins() {
                for entry in &entries {
                    if prefix_set.contains(&entry.name) {
                        for m in &entry.matches {
                            result.push((m.clone(), entry.name.clone()));
                        }
                    }
                }
                for entry in &entries {
                    if !prefix_set.contains(&entry.name) {
                        for m in &entry.matches {
                            result.push((m.clone(), entry.name.clone()));
                        }
                    }
                }
            } else {
                for entry in &entries {
                    for m in &entry.matches {
                        result.push((m.clone(), entry.name.clone()));
                    }
                }
            }

            result
        };

        // Populate the single flat list
        {
            let mut guard = self.matches.guard();
            guard.clear();
            for (m, name) in interleaved {
                let info = self.plugin_info_map.get(&name).cloned().unwrap_or_else(|| {
                    anyrun_interface::PluginInfo {
                        name: name.clone().into(),
                        icon: String::new().into(),
                    }
                });
                guard.push_back((m, self.config.clone(), plugin_type_label(&name), info));
            }
        }

        self.apply_results_layout(widgets, root);
        self.clear_pending_visual_state(widgets);

        if !self.matches_entered {
            self.matches_entered = true;
            widgets.matches_list().add_css_class("matches-entered");
        }

        self.selected_index = 0;
        widgets.scroll().vadjustment().set_value(0.0);
        self.sync_ui_selection(widgets);

        self.sync_shortcuts(widgets);
    }

    fn apply_results_layout(&mut self, widgets: &mut AppWidgets, root: &gtk::Window) {
        let max_entries = self.config.max_entries;

        if let Some(max_entries) = max_entries {
            for (i, plugin_match) in self.matches.iter().enumerate() {
                if i >= max_entries as usize {
                    plugin_match.row.set_visible(false);
                }
            }
        }

        let mon_height = if let Some(surface) = root.surface() {
            let display = gtk::prelude::WidgetExt::display(root);
            display
                .monitor_at_surface(&surface)
                .map(|m| m.geometry().height() as u32)
                .unwrap_or(1080)
        } else {
            1080
        };

        let target_height = self.config.height.to_val(mon_height);

        widgets.scroll().set_min_content_height(target_height);
        widgets.scroll().set_max_content_height(target_height);
        widgets.scroll().set_visible(true);
    }
}
