use crate::{
    config::{Action, Config},
    plugin_box::PluginBox,
    Args,
};
use gtk::{gdk, gio};
use gtk4 as gtk;
use libadwaita as adw;
use relm4::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct App {
    pub(super) config: Arc<Config>,
    pub(super) invocation: Option<SendInvocation>,
    pub(super) plugins: FactoryVecDeque<PluginBox>,
    pub(super) post_run_action: PostRunAction,
    pub(super) tx: mpsc::Sender<anyrun_provider_ipc::Request>,
    pub(super) css_provider: gtk::CssProvider,
    pub(super) selected_index: usize,
    pub(super) selected_plugin_index: Option<usize>,
    pub(super) config_dir: Option<String>,
    pub(super) is_daemon: bool,
    pub(super) search_cancellable: Option<gio::Cancellable>,
    pub(super) height_animation: Option<adw::TimedAnimation>,
    pub(super) last_css_load: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone)]
pub struct SendInvocation(pub gio::DBusMethodInvocation);
unsafe impl Send for SendInvocation {}
unsafe impl Sync for SendInvocation {}

pub const DEFAULT_CSS: &str = include_str!("../../res/style.css");

#[derive(Deserialize, Serialize)]
pub enum PostRunAction {
    Stdout(Vec<u8>),
    None,
}

#[derive(Debug)]
pub enum AppMsg {
    Show {
        width: u32,
        height: u32,
    },
    KeyPressed {
        key: gdk::Key,
        modifier: gdk::ModifierType,
    },
    Action(Action),
    EntryChanged(String),
    PluginOutput(crate::plugin_box::PluginBoxOutput),
    Activate(Option<SendInvocation>),
    SyncShortcuts,
    ReloadPlugins,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct AppInit {
    pub args: Args,
    pub stdin: Vec<u8>,
    pub env: Vec<(String, String)>,
}

pub struct DaemonContext {
    pub config: Arc<Config>,
    pub config_dir: Option<String>,
    pub css_provider: gtk::CssProvider,
    pub socket_path: PathBuf,
}
