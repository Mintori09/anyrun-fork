use super::{App, AppMsg, AppWidgets, PostRunAction, SendInvocation, DEFAULT_CSS};
use crate::config::{Action, Config};
use crate::plugin_box::{LocalAction, MatchSource, PluginMatchInput};
use anyrun_provider_ipc as ipc;
use anyrun_provider_ipc::QueryPhase;
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use gtk4 as gtk;
use relm4::prelude::*;
use std::fs;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::SystemTime;

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

                        let eligible_matches: Vec<_> = self
                            .matches
                            .iter()
                            .enumerate()
                            .filter(|(_, m)| {
                                if !m.row.get_visible() {
                                    return false;
                                }
                                if let Some(bounds) = m.row.compute_bounds(widgets.matches_list()) {
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

                if let Some(action) = self.resolve_action_for_key(key, modifier) {
                    sender.input(AppMsg::Action(action));
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
                            if let Some(app) = root.application() {
                                app.quit();
                            }
                        }

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
                            let visible_matches: Vec<_> = self
                                .matches
                                .iter()
                                .enumerate()
                                .filter(|(_, m)| m.row.get_visible())
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
                                    } else if idx == 0 {
                                        visible_matches.len() - 1
                                    } else {
                                        idx - 1
                                    }
                                }
                                None => 0,
                            };

                            let (global_idx, _) = visible_matches[next_visible_idx];
                            global_idx
                        };

                        self.selected_index = global_idx;
                        self.sync_ui_selection(widgets);
                    }
                    Action::Select => {
                        if let Some(plugin_match) = self.matches.get(self.selected_index) {
                            match &plugin_match.source {
                                MatchSource::Provider | MatchSource::Recent => {
                                    let info = plugin_match.plugin_info.clone();
                                    let content = plugin_match.content.clone();

                                    let _ = self.tx.try_send(ipc::Request::Handle {
                                        plugin: info,
                                        selection: content,
                                    });
                                }
                                MatchSource::Prefix { prefix } => {
                                    widgets.entry().set_text(prefix);
                                    widgets.entry().grab_focus_without_selecting();
                                }
                                MatchSource::Action {
                                    action,
                                    plugin,
                                    selection,
                                } => match action {
                                    LocalAction::DefaultOpen => {
                                        let _ = self.tx.try_send(ipc::Request::Handle {
                                            plugin: plugin.clone(),
                                            selection: selection.clone(),
                                        });
                                    }
                                    LocalAction::CopyTitle => {
                                        root.clipboard().set_text(selection.title.as_str());
                                        sender.input(AppMsg::Action(Action::Close));
                                    }
                                    LocalAction::CopyDescription => {
                                        if let abi_stable::std_types::ROption::RSome(desc) =
                                            &selection.description
                                        {
                                            root.clipboard().set_text(desc.as_str());
                                        }
                                        sender.input(AppMsg::Action(Action::Close));
                                    }
                                },
                            }
                        }
                    }
                    Action::OpenActions => {
                        self.show_action_menu(widgets, root);
                    }
                    Action::ReloadConfig => {
                        if let Some(config_dir) = &self.config_dir {
                            // 1. Reload config.ron
                            let new_config = match fs::read(format!("{config_dir}/config.ron")) {
                                Ok(content) => {
                                    ron::de::from_bytes(&content).unwrap_or_else(|why| {
                                        eprintln!("[anyrun] Failed to parse config: {why}");
                                        Config::default()
                                    })
                                }
                                Err(why) => {
                                    eprintln!("[anyrun] Failed to read config: {why}");
                                    Config::default()
                                }
                            };
                            self.config = Arc::new(new_config);

                            // 2. Reload style.css
                            match fs::read_to_string(format!("{config_dir}/style.css")) {
                                Ok(css) => {
                                    self.css_provider.load_from_string(&css);
                                    self.last_css_load = Some(SystemTime::now());
                                }
                                Err(why) => {
                                    eprintln!("[anyrun] Failed to load CSS: {why}");
                                    self.css_provider.load_from_string(DEFAULT_CSS);
                                    self.last_css_load = Some(SystemTime::now());
                                }
                            }
                        }

                        // 3. Clear stale state
                        self.pending_matches.clear();
                        self.pending_flush_scheduled = false;
                        self.batch_flushing_results = false;
                        self.plugin_health.clear();
                        self.selected_index = 0;

                        // 4. Reload plugins with new config
                        let _ = self.tx.try_send(ipc::Request::ReloadPlugins {
                            plugins: self
                                .config
                                .plugins
                                .iter()
                                .map(|p| p.to_string_lossy().to_string())
                                .collect(),
                        });

                        // 5. Re-search with current input
                        let _ = self.tx.try_send(ipc::Request::Query {
                            text: self.current_input.clone(),
                            phase: QueryPhase::Settling,
                            plugins: Vec::new(),
                            timeout_ms: self.config.search_ux.plugin_timeout_ms,
                            slow_ms: self.config.search_ux.slow_plugin_ms,
                        });

                        // 6. Re-apply window geometry
                        if let Some(surface) = root.surface() {
                            let display = gtk::prelude::WidgetExt::display(root);
                            if let Some(monitor) = display.monitor_at_surface(&surface) {
                                let geo = monitor.geometry();
                                self.handle_show(
                                    widgets,
                                    geo.width() as u32,
                                    geo.height() as u32,
                                    root,
                                );
                            }
                        }
                    }
                }
            }
            AppMsg::EntryChanged(text) => {
                self.selected_index = 0;
                self.plugin_health.clear();
                widgets.scroll().vadjustment().set_value(0.0);
                widgets.matches_list().unselect_all();
                self.last_scroll_top = 0.0;
                self.last_visible_count = 0;
                self.last_flush_hash = 1;
                self.current_input = text.clone();
                self.search_epoch = self.search_epoch.wrapping_add(1);
                self.settle_animation_epoch = None;
                self.settled_once = false;
                self.settle_query_sent = false;
                self.pending_matches.clear();
                self.pending_flush_scheduled = false;
                self.pending_settle_epoch = Some(self.search_epoch);
                self.settling_plugins_sent.clear();

                if text == self.config.search_ux.prefix_discovery_trigger {
                    if let Some(cancellable) = self.search_cancellable.take() {
                        cancellable.cancel();
                    }
                    self.show_prefix_discovery(widgets, root);
                    return;
                }

                if text.trim().is_empty() && !self.config.show_results_immediately {
                    if let Some(cancellable) = self.search_cancellable.take() {
                        cancellable.cancel();
                    }
                    self.matches.guard().clear();
                    self.finish_local_rows(widgets, root);
                    self.request_empty_state();
                    return;
                }

                // Detect rapid typing: skip IPC query if keystroke interval < settle_delay_ms
                let now = std::time::Instant::now();
                let is_rapid = self
                    .last_entry_change
                    .map(|t| {
                        now.duration_since(t)
                            < std::time::Duration::from_millis(
                                self.config.search_ux.settle_delay_ms,
                            )
                    })
                    .unwrap_or(false);
                self.last_entry_change = Some(now);

                // Show loading bar; keep previous results visible always
                self.set_pending_visual_state(widgets);

                // Cancel any pending query to prevent backlog during rapid typing
                if let Some(cancellable) = self.search_cancellable.take() {
                    cancellable.cancel();
                }

                let cancellable = gio::Cancellable::new();
                self.search_cancellable = Some(cancellable.clone());
                let tx = self.tx.clone();
                let sender_clone = sender.clone();
                let epoch = self.search_epoch;
                let settle_delay = self.config.search_ux.settle_delay_ms;

                if !is_rapid {
                    // Only send Typing query if the user paused long enough
                    let typing_plugins = self.route_plugins(&text);
                    self.settling_plugins_sent
                        .extend(typing_plugins.iter().cloned());

                    let _ = tx.try_send(ipc::Request::Query {
                        text: text.clone(),
                        phase: QueryPhase::Typing,
                        plugins: typing_plugins,
                        timeout_ms: self.config.search_ux.plugin_timeout_ms,
                        slow_ms: self.config.search_ux.slow_plugin_ms,
                    });
                }

                // Always schedule settle (cancellable prevents stale fire)
                if settle_delay == 0 {
                    if !cancellable.is_cancelled() {
                        sender_clone.input(AppMsg::TriggerSettledQuery(epoch, text));
                    }
                } else {
                    glib::MainContext::default().spawn_local(async move {
                        glib::timeout_future(std::time::Duration::from_millis(settle_delay)).await;

                        if !cancellable.is_cancelled() {
                            sender_clone.input(AppMsg::TriggerSettledQuery(epoch, text));
                        }
                    });
                }
            }
            AppMsg::EntryActivated => {
                if let Some(action) =
                    self.resolve_action_for_key(gdk::Key::Return, gdk::ModifierType::empty())
                {
                    sender.input(AppMsg::Action(action));
                }
            }
            AppMsg::SyncShortcuts => {
                self.sync_shortcuts(widgets);
            }
            AppMsg::ShowShortcuts(show) => {
                self.show_shortcuts = show;
                if show {
                    self.last_scroll_top = -f64::INFINITY;
                    self.sync_shortcuts(widgets);
                } else {
                    for i in 0..self.matches.len() {
                        self.matches.send(i, PluginMatchInput::SetShortcut(None));
                    }
                }
            }
            AppMsg::RowSelected(index) => {
                self.selected_index = index;
            }
            AppMsg::Activate(invocation) => {
                self.handle_activate(widgets, invocation, sender, root);
            }
            AppMsg::ReloadPlugins => {
                sender.input(AppMsg::Action(Action::ReloadConfig));
            }
            AppMsg::FlushPendingMatches(epoch) => {
                if epoch != self.search_epoch {
                    return;
                }

                self.pending_flush_scheduled = false;
                self.handle_pending_matches_flush(widgets, root);
            }
            AppMsg::TriggerSettledQuery(epoch, text) => {
                if epoch != self.search_epoch {
                    return;
                }

                self.settle_query_sent = true;
                let plugins = self.settling_plugins();
                if plugins.is_empty() {
                    return;
                }
                let _ = self.tx.try_send(ipc::Request::Query {
                    text,
                    phase: QueryPhase::Settling,
                    plugins,
                    timeout_ms: self.config.search_ux.plugin_timeout_ms,
                    slow_ms: self.config.search_ux.slow_plugin_ms,
                });
            }
        }
    }
}
