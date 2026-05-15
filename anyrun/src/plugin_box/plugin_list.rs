use std::sync::Arc;
use abi_stable::std_types::RVec;
use anyrun_interface::{Match, PluginInfo};
use gtk::prelude::*;
use gtk4 as gtk;
use relm4::prelude::*;
use crate::config::Config;
use super::match_row::{PluginMatch, PluginMatchInput};

pub struct PluginBox {
    pub plugin_info: PluginInfo,
    pub matches: FactoryVecDeque<PluginMatch>,
    pub(super) config: Arc<Config>,
    pub(super) visible: bool,
    pub(super) enabled: bool,
}

#[derive(Debug, Clone)]
pub enum PluginBoxInput {
    Matches(RVec<Match>),
    Enable(bool),
    /// Sent when there is a possibility that the plugin may need to hide, aka
    /// all its matches have already been hidden
    MaybeHide,
    UpdateShortcuts(Vec<Option<usize>>),
}

#[derive(Debug)]
pub enum PluginBoxOutput {
    MatchesLoaded,
    RowSelected(<PluginBox as FactoryComponent>::Index, Option<usize>),
}

#[relm4::factory(pub)]
impl FactoryComponent for PluginBox {
    type Init = (PluginInfo, Arc<Config>);
    type Input = PluginBoxInput;
    type Output = PluginBoxOutput;
    type CommandOutput = (u64, RVec<Match>);
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            #[watch]
            set_visible: self.visible,
            set_css_classes: &["plugin"],

            gtk::Box {
                set_visible: false,
                set_css_classes: &["plugin", "info"],
                set_orientation: gtk::Orientation::Vertical,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_expand: false,

                    gtk::Image {
                        set_css_classes: &["plugin", "info"],
                        set_icon_name: Some(&self.plugin_info.icon),
                        set_visible: !self.config.hide_icons,
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Start,
                        set_pixel_size: 32,
                    },
                    gtk::Label {
                        set_css_classes: &["plugin", "info"],
                        set_label: &self.plugin_info.name,
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Center,
                    }
                }
            },

            #[local_ref]
            matches -> gtk::ListBox {
                set_css_classes: &["plugin"],
                set_hexpand: true,
                connect_row_selected[index] => move |_list, row| {
                    if let Some(row) = row {
                        sender.output(PluginBoxOutput::RowSelected(index.clone(), Some(row.index() as usize))).unwrap();
                    }
                }
            }
        }
    }

    fn init_widgets(
        &mut self,
        index: &Self::Index,
        _root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let matches = self.matches.widget();
        let widgets = view_output!();
        widgets
    }

    fn init_model(
        (plugin_info, config): Self::Init,
        _index: &Self::Index,
        _sender: FactorySender<Self>,
    ) -> Self {
        let matches = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .detach();

        Self {
            plugin_info,
            matches,
            config,
            visible: false,
            enabled: true,
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: FactorySender<Self>,
    ) {
        match message {
            PluginBoxInput::Matches(matches) => {
                if !self.enabled {
                    return;
                }

                self.visible = !matches.is_empty();
                {
                    let mut guard = self.matches.guard();

                    guard.clear();

                    for _match in matches {
                        guard.push_back((
                            _match,
                            self.config.clone(),
                            plugin_type_label(&self.plugin_info.name).to_string(),
                        ));
                    }
                }
                sender.output(PluginBoxOutput::MatchesLoaded).unwrap();
            }
            PluginBoxInput::Enable(enable) => {
                self.enabled = enable;
                self.visible = enable;

                if !enable {
                    self.matches.guard().clear();
                }
            }
            PluginBoxInput::MaybeHide => {
                let mut hide = true;

                for plugin_match in self.matches.iter() {
                    if plugin_match.row.get_visible() {
                        hide = false;
                        break;
                    }
                }

                self.visible = !hide;
            }
            PluginBoxInput::UpdateShortcuts(shortcuts) => {
                for (i, shortcut) in shortcuts.into_iter().enumerate() {
                    if i < self.matches.len() {
                        self.matches
                            .send(i, PluginMatchInput::SetShortcut(shortcut));
                    }
                }
            }
        }

        self.update_view(widgets, sender);
    }
}

pub fn plugin_type_label(plugin_name: &str) -> &'static str {
    match plugin_name {
        "Applications" => "App",
        "KDE Settings" | "Bluetooth Control" => "Settings",
        "Shell Wrapper" | "Shell Wrapper Once" | "Calc" | "Universal Action" | "Sync Manager" => {
            "Action"
        }
        "Browser Tabs" | "Web Search" => "Web",
        "Find Files" | "Zoxide Fuzzy" => "File",
        "KDE Klipper" => "Clipboard",
        "Translate" => "Translate",
        "Symbols" => "Symbol",
        _ => "Result",
    }
}
