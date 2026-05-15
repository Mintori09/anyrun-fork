use super::{App, AppInit, AppWidgets, DaemonContext, SendInvocation};
use crate::config::{PrefixRoute, TypingVisual};
use crate::plugin_box::PluginMatchInput;
use gtk::prelude::*;
use gtk4 as gtk;
use relm4::{ComponentBuilder, ComponentController};
use std::collections::HashSet;

impl App {
    pub fn launch(
        app: &gtk::Application,
        app_init: AppInit,
        invocation: Option<SendInvocation>,
        daemon_context: Option<std::rc::Rc<DaemonContext>>,
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

    pub(super) fn sync_ui_selection(&self, widgets: &mut AppWidgets) {
        if self.matches.is_empty() {
            return;
        }

        let listbox = widgets.matches_list();
        listbox.unselect_all();

        if let Some(plugin_match) = self.matches.get(self.selected_index) {
            let row = &plugin_match.row;
            listbox.select_row(Some(row));

            let adj = widgets.scroll().vadjustment();
            if let Some(bounds) = row.compute_bounds(widgets.matches_list()) {
                let y = bounds.y() as f64;
                let row_height = bounds.height() as f64;
                let current_value = adj.value();
                let page_size = adj.page_size();

                let viewport_top = current_value;
                let viewport_bottom = current_value + page_size;

                if y < viewport_top {
                    adj.set_value(y);
                } else if y + row_height > viewport_bottom {
                    adj.set_value(y + row_height - page_size);
                }
            }
        }
        widgets.entry().grab_focus_without_selecting();
    }

    pub(super) fn sync_shortcuts(&mut self, widgets: &AppWidgets) {
        let visible_count = self.matches.iter().filter(|m| m.row.get_visible()).count();
        widgets
            .results_count()
            .set_label(&format!("{visible_count} results"));
        widgets.footer().set_visible(true);

        if self.skip_animations {
            return;
        }

        let mut count = 0;
        let adj = widgets.scroll().vadjustment();
        let scroll_top = adj.value();

        for (i, m) in self.matches.iter().enumerate() {
            if !m.row.get_visible() {
                self.matches.send(i, PluginMatchInput::SetShortcut(None));
                continue;
            }

            let is_eligible = if let Some(bounds) = m.row.compute_bounds(widgets.matches_list()) {
                let y = bounds.y() as f64;
                y + (bounds.height() as f64) > scroll_top
            } else {
                true
            };

            if is_eligible {
                count += 1;
                self.matches.send(
                    i,
                    PluginMatchInput::SetShortcut(if count <= 10 { Some(count) } else { None }),
                );
            } else {
                self.matches.send(i, PluginMatchInput::SetShortcut(None));
            }
        }
    }

    pub(super) fn route_plugins(&self, input: &str) -> Vec<String> {
        if let Some(route) = self.match_prefix_route(input) {
            return route.plugins.clone();
        }

        if input.trim().is_empty() {
            return self.non_prefix_plugins();
        }

        if self.config.search_ux.bare_text_fast_lane.is_empty() {
            return self.plugin_names.clone();
        }

        self.config.search_ux.bare_text_fast_lane.clone()
    }

    pub(super) fn settling_plugins(&self) -> Vec<String> {
        let fast_lane: HashSet<_> = self.settling_plugins_sent.iter().cloned().collect();
        let prefixed: HashSet<String> = self
            .config
            .search_ux
            .prefix_routes
            .iter()
            .flat_map(|route| route.plugins.iter().cloned())
            .collect();

        self.plugin_names
            .iter()
            .cloned()
            .filter(|name| !fast_lane.contains(name))
            .filter(|name| !self.current_input.trim().is_empty() || !prefixed.contains(name))
            .collect()
    }

    pub(super) fn set_pending_visual_state(&mut self, widgets: &AppWidgets) {
        match self.config.search_ux.typing_visual {
            TypingVisual::DimPrevious => widgets.scroll().add_css_class("search-pending"),
            TypingVisual::KeepPrevious => widgets.scroll().remove_css_class("search-pending"),
            TypingVisual::Clear => {
                widgets.scroll().remove_css_class("search-pending");
                self.matches.guard().clear();
            }
        }
    }

    pub(super) fn clear_pending_visual_state(&self, widgets: &AppWidgets) {
        widgets.scroll().remove_css_class("search-pending");
    }

    fn match_prefix_route(&self, input: &str) -> Option<&PrefixRoute> {
        self.config
            .search_ux
            .prefix_routes
            .iter()
            .filter(|route| !route.prefix.is_empty() && input.starts_with(&route.prefix))
            .max_by_key(|route| route.prefix.len())
    }

    pub(super) fn partial_prefix_plugins(&self) -> Option<HashSet<String>> {
        let matched: HashSet<String> = self
            .config
            .search_ux
            .prefix_routes
            .iter()
            .filter(|route| {
                !route.prefix.is_empty()
                    && (self.current_input.starts_with(&route.prefix)
                        || route.prefix.starts_with(&self.current_input))
            })
            .flat_map(|route| route.plugins.iter().cloned())
            .collect();

        if matched.is_empty() {
            None
        } else {
            Some(matched)
        }
    }

    pub(super) fn non_prefix_plugins(&self) -> Vec<String> {
        let prefixed: HashSet<String> = self
            .config
            .search_ux
            .prefix_routes
            .iter()
            .flat_map(|route| route.plugins.iter().cloned())
            .collect();
        let candidates = if self.config.search_ux.bare_text_fast_lane.is_empty() {
            self.plugin_names.clone()
        } else {
            self.config.search_ux.bare_text_fast_lane.clone()
        };
        candidates
            .into_iter()
            .filter(|n| !prefixed.contains(n))
            .collect()
    }
}
