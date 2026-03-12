use gtk::{gdk, glib};
use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use gtk4_layer_shell::{Edge, LayerShell};
use relm4::prelude::*;
use std::fs;
use super::{App, AppMsg, PostRunAction, DEFAULT_CSS, AppWidgets};

impl App {
    pub(super) fn handle_show(
        &mut self,
        widgets: &mut AppWidgets,
        mon_width: u32,
        mon_height: u32,
        root: &gtk::Window,
    ) {
        // let half_height = (mon_height / 2) as i32;
        let matches = self.combined_matches();
        if matches.is_empty() {
            widgets.scroll().set_min_content_height(0);
            widgets.scroll().set_max_content_height(0);
            widgets.scroll().set_visible(false);
        } else {
            let target_height = self.config.max_height.to_val(mon_height);
            widgets.scroll().set_visible(true);

            if let Some(anim) = self.height_animation.take() {
                anim.pause();
            }

            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = scroll)]
                widgets.scroll(),
                move |value| {
                    let val = value as i32;
                    let old_max = scroll.max_content_height();
                    if val > old_max {
                        scroll.set_max_content_height(val);
                        scroll.set_min_content_height(val);
                    } else {
                        scroll.set_min_content_height(val);
                        scroll.set_max_content_height(val);
                    }
                }
            ));

            let anim = adw::TimedAnimation::builder()
                .widget(widgets.scroll())
                .duration(200)
                .easing(adw::Easing::EaseOutQuart)
                .value_from(0.0)
                .value_to(target_height as f64)
                .target(&target)
                .build();

            anim.play();
            self.height_animation = Some(anim);
        }

        let width = self.config.width.to_val(mon_width);
        let x = self.config.x.to_val(mon_width) - width / 2;
        let height = self.config.height.to_val(mon_height);
        let y = self.config.y.to_val(mon_height) - height / 2;

        root.set_anchor(Edge::Left, true);
        root.set_anchor(Edge::Top, true);

        if self.config.close_on_click {
            root.set_default_size(mon_width as i32, mon_height as i32);
            widgets.main_box().set_halign(gtk::Align::Fill);
            widgets.main_box().set_margin_start(x);
            widgets.main_box().set_margin_top(y);
            widgets.main_box().set_margin_end(mon_width as i32 - x - width);
            widgets
                .main_box()
                .set_margin_bottom(mon_height as i32 - y - height);
        } else {
            root.set_default_size(width, height);
            root.child().unwrap().set_size_request(width, height);
            root.set_margin(Edge::Left, x);
            root.set_margin(Edge::Top, y);
        }
        root.set_opacity(1.0); // Continuation of the Sway hack
        widgets.entry().grab_focus_without_selecting();

        // If show_results_immediately is enabled, trigger initial search with empty input
        if self.config.show_results_immediately {
            let _ = self.tx.try_send(anyrun_provider_ipc::Request::Query {
                text: String::new(),
            });
        }
    }

    pub(super) fn handle_activate(
        &mut self,
        widgets: &mut AppWidgets,
        invocation: Option<super::SendInvocation>,
        sender: ComponentSender<Self>,
        root: &gtk::Window,
    ) {
        self.invocation = invocation;
        self.post_run_action = PostRunAction::None;
        widgets.entry().set_text("");
        widgets.scroll().set_min_content_height(0);
        widgets.scroll().set_max_content_height(0);
        widgets.scroll().set_visible(false);

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
            &gtk::prelude::WidgetExt::display(root),
            &self.css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );

        root.set_visible(true);
        widgets.entry().grab_focus_without_selecting();

        // Re-trigger geometry calculation (AppMsg::Show)
        if let Some(surface) = root.surface() {
            let display = gtk::prelude::WidgetExt::display(root);
            if let Some(monitor) = display.monitor_at_surface(&surface) {
                let geometry: gdk::Rectangle = monitor.geometry();
                sender.input(AppMsg::Show {
                    width: geometry.width() as u32,
                    height: geometry.height() as u32,
                });
            }
        }

        if self.is_daemon {
            let _ = self.tx.try_send(anyrun_provider_ipc::Request::Query { text: String::new() });
        }
    }
}
