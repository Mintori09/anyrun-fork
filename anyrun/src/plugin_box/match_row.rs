use crate::config::Config;
use abi_stable::std_types::ROption;
use anyrun_interface::{Match, PluginInfo};
use gtk::{pango, prelude::*};
use gtk4 as gtk;
use relm4::prelude::*;
use std::{path::PathBuf, sync::Arc};

pub struct PluginMatch {
    pub content: Match,
    pub plugin_info: PluginInfo,
    pub row: gtk::ListBoxRow,
    pub(super) config: Arc<Config>,
    pub(super) type_label: String,
    pub(super) _shortcut: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum PluginMatchInput {
    SetShortcut(Option<usize>),
}

#[relm4::factory(pub)]
impl FactoryComponent for PluginMatch {
    type Init = (Match, Arc<Config>, String, PluginInfo);
    type Input = PluginMatchInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;
    view! {
        gtk::ListBoxRow {
            set_css_classes: &["match"],
            set_height_request: 32,
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 10,
                set_css_classes: &["match"],
                set_hexpand: true,

                #[name = "_icon"]
                gtk::Image {
                    set_pixel_size: 28,
                    set_visible: false,
                    set_css_classes: &["match"]
                },

                #[name = "_text"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_css_classes: &["match", "text-fields"],
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_vexpand: true,

                    gtk::Label {
                        set_css_classes: &["match", "title"],
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Center,
                        set_xalign: 0.0,
                        set_wrap: true,
                        set_natural_wrap_mode: gtk::NaturalWrapMode::Word,
                        set_wrap_mode: pango::WrapMode::WordChar,
                        set_use_markup: self.content.use_pango,
                        set_label: &self.content.title,
                    },

                    #[name = "_description"]
                    gtk::Label {
                        set_css_classes: &["match", "description"],
                        set_wrap: true,
                        set_xalign: 0.0,
                        set_use_markup: self.content.use_pango,
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Center,
                    }
                },

                gtk::Label {
                    #[watch]
                    set_label: &self.type_label,
                    set_visible: !self.type_label.is_empty(),
                    set_css_classes: &["match", "type-label"],
                    set_halign: gtk::Align::End,
                    set_valign: gtk::Align::Center,
                }
            }
        }
    }

    fn init_widgets(
        &mut self,
        _index: &Self::Index,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        self.row = root;

        if !self.config.hide_icons {
            if let ROption::RSome(icon) = &self.content.icon {
                widgets._icon.set_visible(true);
                let path = PathBuf::from(icon.to_string());
                if path.is_absolute() {
                    widgets._icon.set_from_file(Some(path));
                } else {
                    widgets._icon.set_icon_name(Some(icon));
                }
            }
        }

        match &self.content.description {
            ROption::RSome(desc) => widgets._description.set_label(desc),
            ROption::RNone => widgets._description.set_visible(false),
        }

        widgets
    }

    fn init_model(
        (content, config, type_label, plugin_info): Self::Init,
        _index: &Self::Index,
        _sender: FactorySender<Self>,
    ) -> Self {
        let row = gtk::ListBoxRow::default();

        Self {
            row,
            content,
            config,
            type_label,
            plugin_info,
            _shortcut: None,
        }
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {
            PluginMatchInput::SetShortcut(shortcut) => {
                self._shortcut = shortcut;
            }
        }
    }
}
