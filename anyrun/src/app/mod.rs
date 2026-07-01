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
use gtk::EntryIconPosition;
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

                // Request KDE blur behind the window (runtime-safe)
                if let Some(wt) = surface.downcast_ref::<gdk4_wayland::WaylandToplevel>() {
                    type BlurFn = unsafe extern "C" fn(*mut std::ffi::c_void, i32);
                    if let Ok(lib) = unsafe { libloading::Library::new("libgdk-4.0.so.0") } {
                        if let Ok(func) = unsafe { lib.get::<BlurFn>(b"gdk_wayland_toplevel_set_blur\0") } {
                            let ptr = gtk::glib::translate::ToGlibPtr::<*mut gdk4_wayland::ffi::GdkWaylandToplevel>::to_glib_none(wt).0;
                            unsafe { func(ptr as *mut std::ffi::c_void, 1) };
                        }
                    }
                }

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

            add_controller = gtk::EventControllerFocus {
                connect_leave[sender, config] => move |_| {
                    if config.close_on_unfocus {
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
                gtk::Entry {
                    set_hexpand: true,
                    set_activates_default: false,
                    connect_activate[sender] => move |_| {
                        sender.input(AppMsg::EntryActivated);
                    },
                    connect_changed[sender] => move |entry| {
                        sender.input(AppMsg::EntryChanged(entry.text().into()));
                    },

                        add_controller = gtk::EventControllerKey {
                            connect_key_pressed[sender] => move |_, key, _, modifier| {
                                if key == gtk::gdk::Key::Alt_L || key == gtk::gdk::Key::Alt_R {
                                    sender.input(AppMsg::ShowShortcuts(true));
                                }
                                sender.input(AppMsg::KeyPressed { key, modifier});
                                match key {
                                    gtk::gdk::Key::Tab | gtk::gdk::Key::Up | gtk::gdk::Key::Down => glib::Propagation::Stop,
                                    _ => glib::Propagation::Proceed,
                                }
                            },
                            connect_key_released[sender] => move |_, key, _, _modifier| {
                                if key == gtk::gdk::Key::Alt_L || key == gtk::gdk::Key::Alt_R {
                                    sender.input(AppMsg::ShowShortcuts(false));
                                }
                            }
                        }
                },

                #[name = "_search_progress"]
                gtk::Box {
                    set_css_classes: &["search-progress"],
                    set_hexpand: true,
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
                    matches_list -> gtk::ListBox {
                        set_css_classes: &["matches"],
                        set_hexpand: true,
                        set_can_focus: false,

                        connect_row_selected[sender] => move |_list, row| {
                            if let Some(row) = row {
                                sender.input(AppMsg::RowSelected(row.index() as usize));
                            }
                        }
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
                        set_label: "↑↓ select · Enter open · Ctrl+R reload · Esc close",
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
        let (model, config, _config_dir, _css_provider, matches_list) =
            Self::init_model(init, &root, sender.clone());

        let widgets = view_output!();

        widgets.entry().set_placeholder_text(Some("Search"));

        widgets
            .entry()
            .set_icon_from_icon_name(EntryIconPosition::Primary, Some("system-search-symbolic"));

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
    pub(super) fn entry(&self) -> &gtk::Entry {
        &self._entry
    }
    pub(super) fn scroll(&self) -> &gtk::ScrolledWindow {
        &self._scroll
    }
    pub(super) fn main_box(&self) -> &gtk::Box {
        &self._main
    }
    pub(super) fn matches_list(&self) -> &gtk::ListBox {
        &self.matches_list
    }
    pub(super) fn footer(&self) -> &gtk::Box {
        &self._footer
    }
    pub(super) fn results_count(&self) -> &gtk::Label {
        &self._results_count
    }
    pub(super) fn search_progress(&self) -> &gtk::Box {
        &self._search_progress
    }
}
