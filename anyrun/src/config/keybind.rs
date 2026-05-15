use gtk4::gdk;
use serde::{de::Visitor, Deserialize, Deserializer};

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum Action {
    Close,
    Select,
    Up,
    Down,
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
