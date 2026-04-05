use super::{App, AppWidgets};
use crate::plugin_box::{PluginBoxInput, PluginBoxOutput};
use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;
use relm4::prelude::*;

impl App {
    pub(super) fn handle_plugin_output(
        &mut self,
        widgets: &mut AppWidgets,
        output: PluginBoxOutput,
        _sender: ComponentSender<Self>,
        root: &gtk::Window,
    ) {
        match output {
            PluginBoxOutput::MatchesLoaded => {
                let is_empty = self.plugins.iter().all(|p| p.matches.is_empty());
                let max_entries = self.config.max_entries;

                if let Some(max_entries) = max_entries {
                    let matches = self.combined_matches();
                    for (_plugin, plugin_match) in matches.iter().skip(max_entries as usize) {
                        plugin_match.row.set_visible(false);
                    }
                }

                if is_empty {
                    if let Some(anim) = self.height_animation.take() {
                        anim.pause();
                    }

                    let current_height = widgets.scroll().min_content_height();

                    if current_height > 0 {
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
                                if val <= 0 {
                                    scroll.set_visible(false);
                                }
                            }
                        ));

                        let anim = adw::TimedAnimation::builder()
                            .widget(widgets.scroll())
                            .duration(200)
                            .easing(adw::Easing::EaseOutQuart)
                            .value_from(current_height as f64)
                            .value_to(0.0)
                            .target(&target)
                            .build();

                        anim.play();
                        self.height_animation = Some(anim);
                    } else {
                        widgets.scroll().set_min_content_height(0);
                        widgets.scroll().set_max_content_height(0);
                        widgets.scroll().set_visible(false);
                    }
                } else {
                    let mon_height = if let Some(surface) = root.surface() {
                        let display = gtk::prelude::WidgetExt::display(root);
                        display
                            .monitor_at_surface(&surface)
                            .map(|m| m.geometry().height() as u32)
                            .unwrap_or(1080)
                    } else {
                        1080
                    };

                    let max_height = self.config.max_height.to_val(mon_height);

                    // Calculate the natural height of the content
                    // We use the plugins box which contains all plugin boxes
                    let (_, natural_height) = widgets.plugins_box().preferred_size();
                    let target_height = natural_height.height().min(max_height);

                    widgets.scroll().set_visible(true);

                    if let Some(anim) = self.height_animation.take() {
                        anim.pause();
                    }

                    let current_height = widgets.scroll().min_content_height();

                    // If the change is small, don't animate to avoid jitter
                    if (current_height - target_height).abs() < 2 {
                        widgets.scroll().set_min_content_height(target_height);
                        widgets.scroll().set_max_content_height(target_height);
                    } else {
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
                            .value_from(current_height as f64)
                            .value_to(target_height as f64)
                            .target(&target)
                            .build();

                        anim.play();
                        self.height_animation = Some(anim);
                    }
                }

                let matches = self.combined_matches();
                if let Some((plugin, plugin_match)) = matches.first() {
                    plugin.matches.widget().select_row(Some(&plugin_match.row));
                }

                if max_entries.is_some() {
                    self.plugins.broadcast(PluginBoxInput::MaybeHide);
                }

                self.sync_shortcuts(widgets);
            }
            PluginBoxOutput::RowSelected(index, row_idx) => {
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
        }
    }
}
