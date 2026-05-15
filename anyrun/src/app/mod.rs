mod init;
mod methods;
mod types;
mod update;
mod update_cmd;
mod update_plugin;
mod update_show;

pub use types::*;

use crate::config;
use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use gtk4_layer_shell::LayerShell;
use libadwaita as adw;
use relm4::prelude::*;
use std::rc::Rc;

#[relm4::component(pub)]
impl Component for App {
    type Input = AppMsg;
    type Output = ();
    type Init = (AppInit, Option<SendInvocation>, Option<Rc<DaemonContext>>);
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
                set_size_request: (800, -1),
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

                    #[local_ref]
                    plugins_widget -> gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_can_focus: false,
                        set_css_classes: &["matches"],
                        set_hexpand: true,
                    }
                },

                #[name = "_footer"]
                gtk::Box {
                    set_css_classes: &["footer"],
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::Fill,
                    set_hexpand: true,
                    set_visible: false,

                    #[name = "_results_count"]
                    gtk::Label {
                        set_css_classes: &["footer", "count"],
                        set_halign: gtk::Align::Start,
                        set_hexpand: true,
                        set_xalign: 0.0,
                    },

                    #[name = "_footer_hints"]
                    gtk::Label {
                        set_css_classes: &["footer", "hints"],
                        set_halign: gtk::Align::End,
                        set_xalign: 1.0,
                        set_label: "↑↓ select · Enter open · Esc close",
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let (model, config, _config_dir, _css_provider, plugins_widget) = Self::init_model(init, &root, sender.clone());

        let widgets = view_output!();
        widgets.entry().set_placeholder_text(Some("Search"));

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
    pub(super) fn entry(&self) -> &gtk::Text {
        &self._entry
    }
    pub(super) fn scroll(&self) -> &gtk::ScrolledWindow {
        &self._scroll
    }
    pub(super) fn main_box(&self) -> &gtk::Box {
        &self._main
    }
    pub(super) fn plugins_box(&self) -> &gtk::Box {
        &self.plugins_widget
    }
    pub(super) fn footer(&self) -> &gtk::Box {
        &self._footer
    }
    pub(super) fn results_count(&self) -> &gtk::Label {
        &self._results_count
    }
}
