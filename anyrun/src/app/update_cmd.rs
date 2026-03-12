use crate::{
    config::Action,
    plugin_box::PluginBoxInput,
};
use anyrun_interface::HandleResult;
use anyrun_provider_ipc as ipc;
use gtk::{gdk, glib};
use gtk4 as gtk;
use gtk::prelude::*;
use relm4::prelude::*;
use std::io::{self, Write};
use super::{App, AppMsg, PostRunAction, AppWidgets};

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
                let mut guard = self.plugins.guard();
                for info in info {
                    guard.push_back((info, self.config.clone()));
                }
            }
            ipc::Response::Matches { plugin, matches } => {
                let i = self
                    .plugins
                    .iter()
                    .enumerate()
                    .find_map(|(i, plugin_box)| {
                        if plugin_box.plugin_info == plugin {
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .unwrap();

                self.plugins.send(i, PluginBoxInput::Matches(matches));
            }
            ipc::Response::Handled { plugin, result } => match result {
                HandleResult::Close => sender.input(AppMsg::Action(Action::Close)),
                HandleResult::Refresh(exclusive) => {
                    let _ = self.tx.try_send(ipc::Request::Query {
                        text: widgets.entry().text().into(),
                    });
                    if exclusive {
                        for (i, plugin_box) in self.plugins.iter().enumerate() {
                            if plugin_box.plugin_info != plugin {
                                self.plugins.send(i, PluginBoxInput::Enable(false));
                            }
                        }
                    } else {
                        self.plugins.broadcast(PluginBoxInput::Enable(true));
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
