use super::{App, AppWidgets};
use crate::plugin_box::{PluginBoxInput, PluginBoxOutput};
use gtk::prelude::*;
use gtk4 as gtk;
use relm4::prelude::*;

impl App {
    pub(super) fn handle_pending_matches_flush(
        &mut self,
        widgets: &mut AppWidgets,
        root: &gtk::Window,
    ) {
        if self.pending_matches.is_empty() {
            return;
        }

        let updates = std::mem::take(&mut self.pending_matches);
        self.batch_flushing_results = true;
        for (i, plugin_box) in self.plugins.iter().enumerate() {
            if let Some(matches) = updates.get(plugin_box.plugin_info.name.as_str()) {
                self.plugins
                    .send(i, PluginBoxInput::Matches(matches.clone()));
            }
        }
        self.batch_flushing_results = false;

        self.apply_results_layout(widgets, root, self.settled_once);
        self.settled_once = true;
        self.clear_pending_visual_state(widgets);

        let matches = self.combined_matches();
        if let Some((plugin, plugin_match)) = matches.first() {
            plugin.matches.widget().select_row(Some(&plugin_match.row));
        }

        if self.config.max_entries.is_some() {
            self.plugins.broadcast(PluginBoxInput::MaybeHide);
        }

        self.sync_shortcuts(widgets);
    }

    pub(super) fn handle_plugin_output(
        &mut self,
        widgets: &mut AppWidgets,
        output: PluginBoxOutput,
        _sender: ComponentSender<Self>,
        root: &gtk::Window,
    ) {
        match output {
            PluginBoxOutput::MatchesLoaded => {
                if self.batch_flushing_results {
                    return;
                }
                self.apply_results_layout(widgets, root, self.settled_once);

                let matches = self.combined_matches();
                if let Some((plugin, plugin_match)) = matches.first() {
                    plugin.matches.widget().select_row(Some(&plugin_match.row));
                }

                if self.config.max_entries.is_some() {
                    self.plugins.broadcast(PluginBoxInput::MaybeHide);
                }

                self.sync_shortcuts(widgets);
            }
            PluginBoxOutput::RowSelected(index, row_idx) => {
                for (i, plugin) in self.plugins.iter().enumerate() {
                    if i != index.current_index() {
                        plugin
                            .matches
                            .widget()
                            .select_row(Option::<&gtk::ListBoxRow>::None);
                    }
                }
                if let Some(row_idx) = row_idx {
                    let mut global_idx = 0;
                    for (i, plugin) in self.plugins.iter().enumerate() {
                        if i < index.current_index() {
                            global_idx += plugin.matches.len();
                        } else {
                            break;
                        }
                    }
                    self.selected_index = global_idx + row_idx;
                }
            }
        }
    }

    fn apply_results_layout(
        &mut self,
        widgets: &mut AppWidgets,
        root: &gtk::Window,
        _allow_animation: bool,
    ) {
        let max_entries = self.config.max_entries;

        if let Some(max_entries) = max_entries {
            let matches = self.combined_matches();
            for (_plugin, plugin_match) in matches.iter().skip(max_entries as usize) {
                plugin_match.row.set_visible(false);
            }
        }

        // if is_empty {
        //     widgets.scroll().set_min_content_height(0);
        //     widgets.scroll().set_max_content_height(0);
        //     widgets.scroll().set_visible(false);
        //     return;
        // }

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
