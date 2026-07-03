use super::{App, AppWidgets};
use crate::plugin_box::{plugin_type_label, MatchSource};
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

        // Build interleaved list directly from pending_matches, using self.plugin_names for order
        let pending = &self.pending_matches;
        let interleaved: Vec<(anyrun_interface::Match, String)> = {
            let mut result = Vec::new();

            if let Some(prefix_set) = self.partial_prefix_plugins() {
                for name in &self.plugin_names {
                    if !prefix_set.contains(name.as_str()) {
                        continue;
                    }
                    if let Some(ms) = pending.get(name) {
                        for m in ms.iter() {
                            result.push((m.clone(), name.clone()));
                        }
                    }
                }
                for name in &self.plugin_names {
                    if prefix_set.contains(name.as_str()) {
                        continue;
                    }
                    if let Some(ms) = pending.get(name) {
                        for m in ms.iter() {
                            result.push((m.clone(), name.clone()));
                        }
                    }
                }
            } else {
                for name in &self.plugin_names {
                    if let Some(ms) = pending.get(name) {
                        for m in ms.iter() {
                            result.push((m.clone(), name.clone()));
                        }
                    }
                }
            }

            result
        };

        // Populate the single flat list
        {
            let mut guard = self.matches.guard();
            guard.clear();
            for (m, name) in &interleaved {
                let info = self
                    .plugin_info_map
                    .get(name.as_str())
                    .cloned()
                    .unwrap_or_else(|| anyrun_interface::PluginInfo {
                        name: name.clone().into(),
                        icon: String::new().into(),
                    });
                guard.push_back((
                    m.clone(),
                    self.config.clone(),
                    plugin_type_label(name),
                    info,
                    MatchSource::Provider,
                ));
            }
        }

        self.apply_results_layout(widgets, root);

        // Show empty state when settle completed and no results for a non-empty query
        let has_results = !self.matches.is_empty();
        let query_non_empty = !self.current_input.trim().is_empty();
        if has_results {
            widgets.empty_state().set_visible(false);
        }

        if self.settle_query_sent {
            self.settle_query_sent = false;
            self.clear_pending_visual_state(widgets);
            // Show empty state only after settle phase completes with no results
            if !has_results && query_non_empty {
                widgets.empty_state().set_visible(true);
            }
        }

        if !self.matches_entered {
            self.matches_entered = true;
            widgets.matches_list().add_css_class("matches-entered");
        }

        self.selected_index = 0;
        widgets.scroll().vadjustment().set_value(0.0);
        self.sync_ui_selection(widgets);

        self.sync_shortcuts(widgets);
    }

    pub(super) fn apply_results_layout(&mut self, widgets: &mut AppWidgets, root: &gtk::Window) {
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
