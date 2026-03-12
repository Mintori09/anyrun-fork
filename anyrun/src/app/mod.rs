mod methods;
mod types;
mod update;
mod update_cmd;
mod update_plugin;
mod update_show;

pub use types::*;

use crate::{
    config::{self, Config},
    plugin_box::PluginBox,
    provider,
};
use anyrun_provider_ipc as ipc;
use gtk::{gio, glib};
use gtk4 as gtk;
use libadwaita as adw;
use adw::prelude::*;
use gtk4_layer_shell::LayerShell;
use relm4::prelude::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

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
                        sender.input(AppMsg::Action(crate::config::Action::Close));
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
                                gtk::gdk::Key::Tab => glib::Propagation::Stop,
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

                    connect_realize[sender] => move |scroll| {
                        let adj = scroll.vadjustment();
                        let sender = sender.clone();
                        adj.connect_value_changed(move |_| {
                            sender.input(AppMsg::SyncShortcuts);
                        });
                    },

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

        let plugins_factory = FactoryVecDeque::<crate::plugin_box::PluginBox>::builder()
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
            height_animation: None,
            last_css_load: Some(std::time::SystemTime::now()),
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
        self.handle_window_msg(widgets, message, sender.clone(), root);
        self.update_view(widgets, sender);
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.handle_cmd_msg(widgets, message, sender.clone(), root);
        self.update_view(widgets, sender);
    }
}

impl AppWidgets {
    pub(super) fn entry(&self) -> &gtk::Text { &self._entry }
    pub(super) fn scroll(&self) -> &gtk::ScrolledWindow { &self._scroll }
    pub(super) fn main_box(&self) -> &gtk::Box { &self._main }
    pub(super) fn plugins_box(&self) -> &gtk::Box { &self.plugins }
}
