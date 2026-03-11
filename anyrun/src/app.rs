use crate::{
    config::{self, Action, Config, Keybind},
    plugin_box::{PluginBox, PluginBoxInput, PluginBoxOutput, PluginMatch},
    provider, Args,
};
use anyrun_interface::HandleResult;
use anyrun_provider_ipc as ipc;
use gtk::{gdk, gio, glib, prelude::*};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, LayerShell};
use relm4::{prelude::*, ComponentBuilder, ComponentController, Sender};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::{
    fs,
    io::{self, Write},
    sync::Arc,
};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct SendInvocation(pub gio::DBusMethodInvocation);
unsafe impl Send for SendInvocation {}
unsafe impl Sync for SendInvocation {}

pub const DEFAULT_CSS: &str = include_str!("../res/style.css");

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
    PluginOutput(PluginBoxOutput),
    Activate(Option<SendInvocation>),
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

pub struct App {
    config: Arc<Config>,
    invocation: Option<SendInvocation>,
    plugins: FactoryVecDeque<PluginBox>,
    post_run_action: PostRunAction,
    tx: mpsc::Sender<anyrun_provider_ipc::Request>,
    css_provider: gtk::CssProvider,
    selected_index: usize,
    selected_plugin_index: Option<usize>,
    config_dir: Option<String>,
    is_daemon: bool,
    search_cancellable: Option<gio::Cancellable>,
}

impl App {
    pub fn launch(
        app: &gtk::Application,
        app_init: AppInit,
        invocation: Option<SendInvocation>,
        daemon_context: Option<Arc<DaemonContext>>,
    ) -> Sender<AppMsg> {
        let builder = ComponentBuilder::<App>::default();

        let connector = builder.launch((app_init, invocation, daemon_context));

        let mut controller = connector.detach();
        let window = controller.widget();
        app.add_window(window);
        window.set_visible(false);
        controller.detach_runtime();
        controller.sender().clone()
    }

    fn sync_ui_selection(
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

            let adj = widgets._scroll.vadjustment();

            if let Some(bounds) = row.compute_bounds(&widgets._scroll) {
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
        widgets._entry.grab_focus_without_selecting();
        new_plugin_index
    }

    fn combined_matches(&self) -> Vec<(&PluginBox, &PluginMatch)> {
        let total_matches: usize = self.plugins.iter().map(|p| p.matches.len()).sum();
        let mut matches = Vec::with_capacity(total_matches);

        for plugin in self.plugins.iter() {
            for plugin_match in plugin.matches.iter() {
                matches.push((plugin, plugin_match));
            }
        }
        matches
    }

    fn sync_shortcuts(&self) {
        let mut count = 0;
        for (i, plugin) in self.plugins.iter().enumerate() {
            let mut shortcuts = Vec::new();
            for _ in plugin.matches.iter() {
                count += 1;
                if count <= 10 {
                    shortcuts.push(Some(count));
                } else {
                    shortcuts.push(None);
                }
            }
            self.plugins.send(i, PluginBoxInput::UpdateShortcuts(shortcuts));
        }
    }
}

#[relm4::component(pub)]
impl Component for App {
    type Input = AppMsg;
    type Output = ();
    type Init = (AppInit, Option<SendInvocation>, Option<Arc<DaemonContext>>);
    type CommandOutput = anyrun_provider_ipc::Response;

    view! {
        gtk::Window {
            init_layer_shell: (),
            set_layer: match config.layer {
                config::Layer::Background => gtk4_layer_shell::Layer::Background,
                config::Layer::Bottom => gtk4_layer_shell::Layer::Bottom,
                config::Layer::Top => gtk4_layer_shell::Layer::Top,
                config::Layer::Overlay => gtk4_layer_shell::Layer::Overlay,
            },
            set_keyboard_mode: match config.keyboard_mode {
                config::KeyboardMode::Exclusive => gtk4_layer_shell::KeyboardMode::Exclusive,
                config::KeyboardMode::OnDemand => gtk4_layer_shell::KeyboardMode::OnDemand,
            },
            set_namespace: Some("anyrun"),
            // This cannot be fully transparent due to a Sway issue (https://github.com/swaywm/sway/issues/8904)
            // so this ugly workaround is the only way to make it work
            // FIXME: This is dumb
            set_opacity: 0.005,
            set_exclusive_zone: if config.ignore_exclusive_zones {
                -1
            } else {
                0
            },

            connect_realize[sender] => move |win| {
                let surface = win.surface().unwrap();
                let sender = sender.clone();
                surface.connect_enter_monitor(move |_, monitor| {
                    sender.input(AppMsg::Show {
                        width: monitor.geometry().width() as u32,
                        height: monitor.geometry().height() as u32,
                    });
                });
            },

            add_controller = gtk::GestureClick {
                connect_pressed[sender, config] => move |_, _, _, _| {
                    if config.close_on_click {
                        sender.input(AppMsg::Action(Action::Close));
                    }
                }
            },

            #[name = "_main"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_halign: gtk::Align::Center,
                set_vexpand: false,
                set_hexpand: true,
                set_css_classes: &["main"],

                #[name = "_entry"]
                  gtk::Text {
                  set_hexpand: true,
                  set_activates_default: false,
                  connect_changed[sender] => move |entry| {
                      sender.input(AppMsg::EntryChanged(entry.text().into()));
                  },

                    add_controller = gtk::EventControllerKey {
                        connect_key_pressed[sender] => move |_, key, _, modifier| {
                            sender.input(AppMsg::KeyPressed { key, modifier});
                            match key {
                                gdk::Key::Tab => glib::Propagation::Stop,
                                _ => glib::Propagation::Proceed,
                            }
                        }
                    }
                },
                #[name = "_scroll"]
                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hexpand: true,
                    set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),

                    #[local]
                    plugins -> gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_can_focus: false,
                        set_css_classes: &["matches"],
                        set_hexpand: true,
                    }
                }
            }
        }
    }

    fn init(
        (app_init, invocation, daemon_context): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let (config, config_dir, css_provider) = if let Some(ctx) = daemon_context.as_ref() {
            gtk::style_context_add_provider_for_display(
                &root.upcast_ref::<gtk::Widget>().display(),
                &ctx.css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            );
            (
                ctx.config.clone(),
                ctx.config_dir.clone(),
                ctx.css_provider.clone(),
            )
        } else {
            let user_dir = env::var("XDG_CONFIG_HOME")
                .map(|c| format!("{c}/anyrun"))
                .or_else(|_| env::var("HOME").map(|h| format!("{h}/.config/anyrun")))
                .unwrap();
            let config_dir = app_init
                .args
                .config_dir
                .clone()
                .map(Some)
                .unwrap_or_else(|| {
                    if PathBuf::from(&user_dir).exists() {
                        Some(user_dir.clone())
                    } else {
                        ipc::CONFIG_DIRS
                            .iter()
                            .map(|path| path.to_string())
                            .find(|path| PathBuf::from(path).exists())
                    }
                });

            let css_provider = gtk::CssProvider::new();

            let mut config = if let Some(config_dir) = &config_dir {
                match fs::read_to_string(format!("{config_dir}/style.css")) {
                    Ok(style) => {
                        css_provider.load_from_string(&style);
                    }
                    Err(why) => {
                        eprintln!("[anyrun] Failed to load CSS: {why}");
                        css_provider.load_from_string(DEFAULT_CSS);
                    }
                }
                match fs::read(format!("{config_dir}/config.ron")) {
                    Ok(content) => ron::de::from_bytes(&content).unwrap_or_else(|why| {
                        eprintln!(
                            "[anyrun] Failed to parse config file, using default values: {why}"
                        );
                        Config::default()
                    }),
                    Err(why) => {
                        eprintln!(
                            "[anyrun] Failed to read config file, using default values: {why}"
                        );
                        Config::default()
                    }
                }
            } else {
                eprintln!("[anyrun] No config found in any searched paths");
                css_provider.load_from_string(DEFAULT_CSS);
                Config::default()
            };

            gtk::style_context_add_provider_for_display(
                &root.upcast_ref::<gtk::Widget>().display(),
                &css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            );

            config.merge_opt(app_init.args.config.clone());

            (Arc::new(config), config_dir, css_provider)
        };

        let plugins = gtk::Box::builder().build();

        let plugins_factory = FactoryVecDeque::<PluginBox>::builder()
            .launch(plugins.clone())
            .forward(sender.input_sender(), AppMsg::PluginOutput);

        let (tx, rx) = mpsc::channel(64);

        let socket_path = daemon_context.as_ref().map(|ctx| ctx.socket_path.clone());

        sender.spawn_command(glib::clone!(
            #[strong]
            config,
            #[strong]
            config_dir,
            #[strong(rename_to = stdin)]
            app_init.stdin,
            #[strong(rename_to = env)]
            app_init.env,
            move |sender| {
                if let Some(socket_path) = socket_path {
                    if let Err(why) = provider::worker_connect(socket_path, rx, sender) {
                        eprintln!("[anyrun] IPC worker failed to connect: {why}");
                    }
                } else {
                    if let Err(why) =
                        provider::worker_spawn(config, config_dir, rx, sender, stdin, env)
                    {
                        eprintln!("[anyrun] IPC worker returned an error: {why}");
                    }
                }
            }
        ));

        let widgets = view_output!();
        let model = Self {
            invocation,
            config,
            plugins: plugins_factory,
            post_run_action: PostRunAction::None,
            tx,
            css_provider,
            config_dir,
            selected_index: 0,
            selected_plugin_index: None,
            is_daemon: daemon_context.is_some(),
            search_cancellable: None,
        };

        if model.is_daemon {
            let _ = model.tx.try_send(ipc::Request::Reset);
        }

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppMsg::Show {
                width: mon_width,
                height: mon_height,
            } => {
                // let half_height = (mon_height / 2) as i32;
                let matches = self.combined_matches();
                if matches.is_empty() {
                    widgets._scroll.set_min_content_height(0);
                    widgets._scroll.set_max_content_height(0);
                    widgets._scroll.set_visible(false);
                } else {
                    let max_height = self.config.max_height.to_val(mon_height);
                    widgets._scroll.set_max_content_height(max_height);
                    widgets._scroll.set_min_content_height(max_height);
                    widgets._scroll.set_visible(true);
                }

                let width = self.config.width.to_val(mon_width);
                let x = self.config.x.to_val(mon_width) - width / 2;
                let height = self.config.height.to_val(mon_height);
                let y = self.config.y.to_val(mon_height) - height / 2;

                root.set_anchor(Edge::Left, true);
                root.set_anchor(Edge::Top, true);

                if self.config.close_on_click {
                    root.set_default_size(mon_width as i32, mon_height as i32);
                    widgets._main.set_halign(gtk::Align::Fill);
                    widgets._main.set_margin_start(x);
                    widgets._main.set_margin_top(y);
                    widgets._main.set_margin_end(mon_width as i32 - x - width);
                    widgets
                        ._main
                        .set_margin_bottom(mon_height as i32 - y - height);
                } else {
                    root.set_default_size(width, height);
                    root.child().unwrap().set_size_request(width, height);
                    root.set_margin(Edge::Left, x);
                    root.set_margin(Edge::Top, y);
                }
                root.set_opacity(1.0); // Continuation of the Sway hack
                widgets._entry.grab_focus_without_selecting();

                // If show_results_immediately is enabled, trigger initial search with empty input
                if self.config.show_results_immediately {
                    let _ = self.tx.try_send(anyrun_provider_ipc::Request::Query {
                        text: String::new(),
                    });
                }
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
                        let matches = self.combined_matches();
                        if n <= matches.len() {
                            self.selected_index = n - 1;
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
                            &root.upcast_ref::<gtk::Widget>().display(),
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
                        // Compute len without allocating a full Vec
                        let len: usize = self.plugins.iter().map(|p| p.matches.len()).sum();
                        if len == 0 {
                            return;
                        }

                        if matches!(action, Action::Down) {
                            self.selected_index = (self.selected_index + 1) % len;
                        } else {
                            self.selected_index = if self.selected_index == 0 {
                                len - 1
                            } else {
                                self.selected_index - 1
                            };
                        }

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
            AppMsg::PluginOutput(PluginBoxOutput::MatchesLoaded) => {
                let matches = self.combined_matches();
                if matches.is_empty() {
                    widgets._scroll.set_min_content_height(0);
                    widgets._scroll.set_max_content_height(0);
                    widgets._scroll.set_visible(false);
                } else {
                    // Use a reasonable height or the configured max_height
                    // We need to know the monitor height to calculate max_height.
                    // For now, let's try to get it from the window surface if possible, 
                    // or just use a fallback if not yet shown.
                    let mon_height = if let Some(surface) = root.surface() {
                        let display = root.upcast_ref::<gtk::Widget>().display();
                        display.monitor_at_surface(&surface).map(|m| m.geometry().height()).unwrap_or(1080)
                    } else {
                        1080
                    } as u32;
                    
                    let max_height = self.config.max_height.to_val(mon_height);
                    widgets._scroll.set_max_content_height(max_height);
                    widgets._scroll.set_min_content_height(max_height);
                    widgets._scroll.set_visible(true);
                }

                if let Some((plugin, plugin_match)) = matches.first() {
                    plugin.matches.widget().select_row(Some(&plugin_match.row));
                }

                if let Some(max_entries) = self.config.max_entries {
                    for (_plugin, plugin_match) in matches.iter().skip(max_entries as usize) {
                        plugin_match.row.set_visible(false);
                    }
                    self.plugins.broadcast(PluginBoxInput::MaybeHide);
                }

                self.sync_shortcuts();
            }
            AppMsg::PluginOutput(PluginBoxOutput::RowSelected(index, row_idx)) => {
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
            AppMsg::Activate(invocation) => {
                self.invocation = invocation;
                self.post_run_action = PostRunAction::None;
                widgets._entry.set_text("");
                widgets._scroll.set_min_content_height(0);
                widgets._scroll.set_max_content_height(0);
                widgets._scroll.set_visible(false);

                // Re-load CSS if in daemon mode to support hot-reload
                if self.is_daemon {
                    if let Some(config_dir) = &self.config_dir {
                        match fs::read_to_string(format!("{config_dir}/style.css")) {
                            Ok(style) => {
                                self.css_provider.load_from_string(&style);
                            }
                            Err(_) => {
                                self.css_provider.load_from_string(DEFAULT_CSS);
                            }
                        }
                    }
                }

                // Re-apply style provider because it might have been removed on Close
                gtk::style_context_add_provider_for_display(
                    &root.upcast_ref::<gtk::Widget>().display(),
                    &self.css_provider,
                    gtk::STYLE_PROVIDER_PRIORITY_USER,
                );

                root.set_visible(true);
                widgets._entry.grab_focus_without_selecting();

                // Re-trigger geometry calculation (AppMsg::Show)
                if let Some(surface) = root.surface() {
                    let display = root.upcast_ref::<gtk::Widget>().display();
                    if let Some(monitor) = display.monitor_at_surface(&surface) {
                        let geometry: gdk::Rectangle = monitor.geometry();
                        sender.input(AppMsg::Show {
                            width: geometry.width() as u32,
                            height: geometry.height() as u32,
                        });
                    }
                }

                if self.is_daemon {
                    let _ = self.tx.try_send(ipc::Request::Reset);
                }
            }
        }
        self.update_view(widgets, sender);
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
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
                        text: widgets._entry.text().into(),
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
        self.update_view(widgets, sender);
    }
}
