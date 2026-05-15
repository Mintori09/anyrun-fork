use super::{App, AppWidgets};
use crate::plugin_box::plugin_type_label;
use gtk::prelude::*;
use gtk4 as gtk;

impl App {
    pub(super) fn handle_pending_matches_flush(
        &mut self,
        widgets: &mut AppWidgets,
        root: &gtk::Window,
    ) {
        if self.pending_matches.is_empty() {
            return;
        }

        let plugin_names: Vec<String> = self.pending_matches.keys().cloned().collect();

        struct PluginEntry {
            name: String,
            matches: Vec<anyrun_interface::Match>,
        }

        let entries: Vec<PluginEntry> = plugin_names
            .iter()
            .filter_map(|name| {
                self.pending_matches.get(name).map(|ms| PluginEntry {
                    name: name.clone(),
                    matches: ms.iter().cloned().collect(),
                })
            })
            .collect();

        let total: usize = entries.iter().map(|e| e.matches.len()).sum();

        // Round-robin interleave
        let mut interleaved: Vec<(anyrun_interface::Match, String)> = Vec::with_capacity(total);
        let mut idx = 0;
        loop {
            let mut added = false;
            for entry in &entries {
                if idx < entry.matches.len() {
                    interleaved.push((entry.matches[idx].clone(), entry.name.clone()));
                    added = true;
                }
            }
            if !added {
                break;
            }
            idx += 1;
        }

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
                guard.push_back((
                    m,
                    self.config.clone(),
                    plugin_type_label(&name).to_string(),
                    info,
                ));
            }
        }

        self.apply_results_layout(widgets, root);
        self.clear_pending_visual_state(widgets);

        self.selected_index = 0;
        widgets.scroll().vadjustment().set_value(0.0);
        if let Some(plugin_match) = self.matches.iter().next() {
            widgets.matches_list().select_row(Some(&plugin_match.row));
        }

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
