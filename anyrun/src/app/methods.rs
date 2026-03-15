use crate::plugin_box::{PluginBox, PluginBoxInput, PluginMatch};
use gtk4 as gtk;
use gtk::prelude::*;
use relm4::{prelude::*, ComponentBuilder, ComponentController, Sender};
use super::{App, AppInit, AppMsg, DaemonContext, SendInvocation, AppWidgets};

impl App {
    pub fn launch(
        app: &gtk::Application,
        app_init: AppInit,
        invocation: Option<SendInvocation>,
        daemon_context: Option<std::sync::Arc<DaemonContext>>,
    ) -> relm4::Controller<App> {
        let builder = ComponentBuilder::<App>::default();

        let connector = builder.launch((app_init, invocation, daemon_context));

        let mut controller = connector.detach();
        let window = controller.widget();
        app.add_window(window);
        window.set_visible(false);
        controller.detach_runtime();
        controller
    }

    pub(super) fn sync_ui_selection(
        &self,
        widgets: &mut AppWidgets,
        matches: &[(&PluginBox, &PluginMatch)],
    ) -> Option<usize> {
        if matches.is_empty() {
            return self.selected_plugin_index;
        }

        // Only deselect the previously selected plugin instead of all plugins
        if let Some(old_idx) = self.selected_plugin_index {
            if let Some(old_plugin) = self.plugins.get(old_idx) {
                old_plugin
                    .matches
                    .widget()
                    .select_row(Option::<&gtk4::ListBoxRow>::None);
            }
        }

        let mut new_plugin_index = self.selected_plugin_index;

        if let Some((plugin, plugin_match)) = matches.get(self.selected_index) {
            let listbox = plugin.matches.widget();
            let row = &plugin_match.row;
            listbox.select_row(Some(row));

            // Determine which plugin is now selected
            new_plugin_index = self
                .plugins
                .iter()
                .enumerate()
                .find(|(_, p)| std::ptr::eq(*p, *plugin))
                .map(|(i, _)| i);

            let adj = widgets.scroll().vadjustment();

            if let Some(bounds) = row.compute_bounds(widgets.scroll()) {
                let y = bounds.y() as f64;
                let row_height = bounds.height() as f64;

                let current_value = adj.value();
                let page_size = adj.page_size();

                if y < 0.0 {
                    adj.set_value(current_value + y);
                } else if y + row_height > page_size {
                    adj.set_value(current_value + (y + row_height - page_size));
                }
            }
        }
        widgets.entry().grab_focus_without_selecting();
        new_plugin_index
    }

    pub(super) fn combined_matches(&self) -> Vec<(&PluginBox, &PluginMatch)> {
        let total_matches: usize = self.plugins.iter().map(|p| p.matches.len()).sum();
        let mut matches = Vec::with_capacity(total_matches);

        for plugin in self.plugins.iter() {
            for plugin_match in plugin.matches.iter() {
                matches.push((plugin, plugin_match));
            }
        }
        matches
    }

    pub(super) fn sync_shortcuts(&self, widgets: &AppWidgets) {
        let mut count = 0;
        let adj = widgets.scroll().vadjustment();
        let scroll_top = adj.value();

        for (i, plugin) in self.plugins.iter().enumerate() {
            let mut shortcuts = Vec::new();
            for m in plugin.matches.iter() {
                if !m.row.get_visible() {
                    shortcuts.push(None);
                    continue;
                }

                // A match is only given a shortcut if it's below or at the current scroll top
                // We use compute_bounds relative to the plugins container to get the absolute Y within the scroll area
                let is_eligible = if let Some(bounds) = m.row.compute_bounds(widgets.plugins_box()) {
                    let y = bounds.y() as f64;
                    // If the row's bottom is below the scroll top, it's visible in viewport
                    y + (bounds.height() as f64) > scroll_top
                } else {
                    // Fallback if bounds can't be computed
                    true
                };

                if is_eligible {
                    count += 1;
                    if count <= 10 {
                        shortcuts.push(Some(count));
                    } else {
                        shortcuts.push(None);
                    }
                } else {
                    shortcuts.push(None);
                }
            }
            self.plugins.send(i, PluginBoxInput::UpdateShortcuts(shortcuts));
        }
    }
}
