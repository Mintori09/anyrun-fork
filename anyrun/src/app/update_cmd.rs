use super::{App, AppMsg, AppWidgets, PostRunAction};
use crate::config::Action;
use anyrun_interface::HandleResult;
use anyrun_provider_ipc as ipc;
use anyrun_provider_ipc::QueryPhase;
use gtk::prelude::*;
use gtk::{gdk, glib};
use gtk4 as gtk;
use relm4::prelude::*;
use std::io::{self, Write};

impl App {
    pub(super) fn handle_cmd_msg(
        &mut self,
        widgets: &mut AppWidgets,
        message: ipc::Response,
        sender: ComponentSender<Self>,
        root: &gtk::Window,
    ) {
        match message {
            ipc::Response::Ready { info } => {
                self.plugin_names.clear();
                self.plugin_info_map.clear();
                for info in info {
                    self.plugin_names.push(info.name.to_string());
                    self.plugin_info_map.insert(info.name.to_string(), info);
                }
            }
            ipc::Response::Matches { plugin, matches } => {
                self.pending_matches
                    .insert(plugin.name.to_string(), matches);

                if !self.pending_flush_scheduled {
                    self.pending_flush_scheduled = true;
                    let epoch = self.search_epoch;
                    let sender = sender.clone();
                    let delay = self.config.search_ux.flush_delay_ms;

                    glib::MainContext::default().spawn_local(async move {
                        glib::timeout_future(std::time::Duration::from_millis(delay)).await;
                        sender.input(AppMsg::FlushPendingMatches(epoch));
                    });
                }
            }
            ipc::Response::Handled { plugin: _, result } => match result {
                HandleResult::Close => sender.input(AppMsg::Action(Action::Close)),
                HandleResult::Refresh(exclusive) => {
                    let _ = self.tx.try_send(ipc::Request::Query {
                        text: widgets.entry().text().into(),
                        phase: QueryPhase::Settling,
                        plugins: Vec::new(),
                    });
                    if exclusive {
                        // With a flat list, clear matches when exclusive refresh is requested
                        self.matches.guard().clear();
                    }
                }
                HandleResult::Copy(rvec) => {
                    let vec = rvec.to_vec();
                    let mime = tree_magic_mini::from_u8(&rvec);
                    if match mime {
                        "TEXT" | "STRING" | "UTF8_STRING" => true,
                        mime if mime.starts_with("text/") => true,
                        _ => false,
                    } {
                        root.clipboard().set_text(&String::from_utf8_lossy(&rvec));
                    } else {
                        let content = gdk::ContentProvider::for_bytes(
                            mime,
                            &glib::Bytes::from_owned(vec.clone()),
                        );
                        if let Err(why) = root.clipboard().set_content(Some(&content)) {
                            eprintln!("[anyrun] Error setting clipboard content: {why}");
                        }
                    }
                    sender.input(AppMsg::Action(Action::Close));
                }
                HandleResult::Stdout(rvec) => {
                    io::stdout().lock().write_all(&rvec).unwrap();
                    self.post_run_action = PostRunAction::Stdout(rvec.into());
                    sender.input(AppMsg::Action(Action::Close));
                }
            },
        }
    }
}
