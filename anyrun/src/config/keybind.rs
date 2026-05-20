use gtk4::gdk;
use serde::{de::Visitor, Deserialize, Deserializer};

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum Action {
    Close,
    Select,
    OpenActions,
    Up,
    Down,
    ReloadConfig,
}

#[derive(Deserialize, Clone)]
pub struct Keybind {
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(deserialize_with = "Keybind::deserialize_key")]
    pub key: gdk::Key,
    pub action: Action,
}

impl Keybind {
    pub fn matches(&self, key: gdk::Key, modifier: gdk::ModifierType) -> bool {
        key_matches(self.key, key)
            && self.ctrl == modifier.contains(gdk::ModifierType::CONTROL_MASK)
            && self.alt == modifier.contains(gdk::ModifierType::ALT_MASK)
            && self.shift == modifier.contains(gdk::ModifierType::SHIFT_MASK)
    }

    fn deserialize_key<'de, D>(deserializer: D) -> Result<gdk::Key, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyVisitor;

        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = gdk::Key;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A plaintext key in the GDK format")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                gdk::Key::from_name(v).ok_or(E::custom("Key name is not valid"))
            }
        }

        deserializer.deserialize_str(KeyVisitor)
    }
}

pub fn is_enter_key(key: gdk::Key) -> bool {
    matches!(key, gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::ISO_Enter)
}

pub fn key_matches(binding_key: gdk::Key, event_key: gdk::Key) -> bool {
    binding_key == event_key || (is_enter_key(binding_key) && is_enter_key(event_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_aliases_match() {
        assert!(key_matches(gdk::Key::Return, gdk::Key::KP_Enter));
        assert!(key_matches(gdk::Key::KP_Enter, gdk::Key::ISO_Enter));
        assert!(key_matches(gdk::Key::ISO_Enter, gdk::Key::Return));
    }

    #[test]
    fn non_enter_keys_do_not_alias() {
        assert!(!key_matches(gdk::Key::Tab, gdk::Key::Return));
        assert!(!key_matches(gdk::Key::Escape, gdk::Key::KP_Enter));
    }

    #[test]
    fn keybind_modifier_matching_remains_strict() {
        let keybind = Keybind {
            ctrl: true,
            alt: false,
            shift: true,
            key: gdk::Key::Return,
            action: Action::Select,
        };

        assert!(keybind.matches(
            gdk::Key::KP_Enter,
            gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK
        ));
        assert!(!keybind.matches(gdk::Key::Return, gdk::ModifierType::CONTROL_MASK));
        assert!(!keybind.matches(
            gdk::Key::Return,
            gdk::ModifierType::CONTROL_MASK
                | gdk::ModifierType::SHIFT_MASK
                | gdk::ModifierType::ALT_MASK
        ));
    }
}
