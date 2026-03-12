use crate::config::{Action, Keybind};
use anyrun_provider_ipc as ipc;
use gtk::{gdk, gio, glib};
use gtk4 as gtk;
use gtk::prelude::*;
use relm4::prelude::*;
use std::io::{self, Write};
use super::{App, AppMsg, PostRunAction, SendInvocation, AppWidgets};

impl App {
    pub(super) fn handle_window_msg(
        &mut self,
        widgets: &mut AppWidgets,
        message: AppMsg,
        sender: ComponentSender<Self>,
        root: &gtk::Window,
    ) {
        match message {
            AppMsg::Show {
                width: mon_width,
                height: mon_height,
            } => {
                self.handle_show(widgets, mon_width, mon_height, root);
            }
            AppMsg::KeyPressed { key, modifier } => {
                if modifier.contains(gdk::ModifierType::ALT_MASK) {
                    let digit = match key {
                        gdk::Key::_1 => Some(1),
                        gdk::Key::_2 => Some(2),
                        gdk::Key::_3 => Some(3),
                        gdk::Key::_4 => Some(4),
                        gdk::Key::_5 => Some(5),
                        gdk::Key::_6 => Some(6),
                        gdk::Key::_7 => Some(7),
                        gdk::Key::_8 => Some(8),
                        gdk::Key::_9 => Some(9),
                        gdk::Key::_0 => Some(10),
                        _ => None,
                    };
                    if let Some(n) = digit {
                        let adj = widgets.scroll().vadjustment();
                        let scroll_top = adj.value();

                        let matches = self.combined_matches();
                        let eligible_matches: Vec<_> = matches
                            .iter()
                            .enumerate()
                            .filter(|(_, (_, m))| {
                                if !m.row.get_visible() {
                                    return false;
                                }
                                if let Some(bounds) = m.row.compute_bounds(widgets.plugins_box()) {
                                    let y = bounds.y() as f64;
                                    y + (bounds.height() as f64) > scroll_top
                                } else {
                                    true
                                }
                            })
                            .collect();

                        if n <= eligible_matches.len() {
                            let (global_idx, _) = eligible_matches[n - 1];
                            self.selected_index = global_idx;
                            sender.input(AppMsg::Action(Action::Select));
                            return;
                        }
                    }
                }

                if let Some(Keybind { action, .. }) = self.config.keybinds.iter().find(|keybind| {
                    keybind.key == key
                        && keybind.ctrl == modifier.contains(gdk::ModifierType::CONTROL_MASK)
                        && keybind.alt == modifier.contains(gdk::ModifierType::ALT_MASK)
                        && keybind.shift == modifier.contains(gdk::ModifierType::SHIFT_MASK)
                }) {
                    sender.input(AppMsg::Action(*action));
                }
            }
            AppMsg::Action(action) => {
                match action {
                    Action::Close => {
                        if let Some(SendInvocation(invocation)) = self.invocation.clone() {
                            invocation.return_value(Some(
                                &(serde_json::to_vec(&self.post_run_action).unwrap(),).to_variant(),
                            ));
                        } else if !self.is_daemon {
                            match &self.post_run_action {
                                PostRunAction::Stdout(bytes) => {
                                    io::stdout().lock().write_all(bytes).unwrap()
                                }
                                PostRunAction::None => (),
                            }
                            root.application().unwrap().quit();
                        }
                        // Unload the style so a new one can be loaded on next show
                        gtk::style_context_remove_provider_for_display(
                            &gtk::prelude::WidgetExt::display(root),
                            &self.css_provider,
                        );
                        if self.is_daemon {
                            root.set_visible(false);
                            self.invocation = None;
                        } else {
                            root.close();
                        }
                        // FIXME: Make sure the worker has actually correctly shut down before
                        // exiting
                        if !self.is_daemon {
                            let _ = self.tx.blocking_send(ipc::Request::Quit);
                        }
                    }
                    Action::Down | Action::Up => {
                        let global_idx = {
                            let matches = self.combined_matches();
                            let visible_matches: Vec<_> = matches
                                .iter()
                                .enumerate()
                                .filter(|(_, (_, m))| m.row.get_visible())
                                .collect();

                            if visible_matches.is_empty() {
                                return;
                            }

                            // Find current visible index
                            let current_visible_idx = visible_matches
                                .iter()
                                .position(|(idx, _)| *idx == self.selected_index);

                            let next_visible_idx = match current_visible_idx {
                                Some(idx) => {
                                    if matches!(action, Action::Down) {
                                        (idx + 1) % visible_matches.len()
                                    } else {
                                        if idx == 0 {
                                            visible_matches.len() - 1
                                        } else {
                                            idx - 1
                                        }
                                    }
                                }
                                None => 0,
                            };

                            let (global_idx, _) = visible_matches[next_visible_idx];
                            global_idx
                        };

                        self.selected_index = global_idx;

                        let matches = self.combined_matches();
                        self.selected_plugin_index = self.sync_ui_selection(widgets, &matches);
                    }
                    Action::Select => {
                        let matches = self.combined_matches();
                        if let Some((plugin, plugin_match)) = matches.get(self.selected_index) {
                            let info = plugin.plugin_info.clone();
                            let content = plugin_match.content.clone();

                            drop(matches);

                            let _ = self.tx.try_send(ipc::Request::Handle {
                                plugin: info,
                                selection: content,
                            });
                        }
                    }
                }
            }
            AppMsg::EntryChanged(text) => {
                self.selected_index = 0;
                if let Some(cancellable) = self.search_cancellable.take() {
                    cancellable.cancel();
                }

                let cancellable = gio::Cancellable::new();
                self.search_cancellable = Some(cancellable.clone());

                let tx = self.tx.clone();

                glib::MainContext::default().spawn_local(async move {
                    glib::timeout_future(std::time::Duration::from_millis(150)).await;

                    if !cancellable.is_cancelled() {
                        let _ = tx.try_send(ipc::Request::Query { text });
                    }
                });
            }
            AppMsg::PluginOutput(output) => {
                self.handle_plugin_output(widgets, output, sender, root);
            }
            AppMsg::SyncShortcuts => {
                self.sync_shortcuts(widgets);
            }
            AppMsg::Activate(invocation) => {
                self.handle_activate(widgets, invocation, sender, root);
            }
        }
    }
}
