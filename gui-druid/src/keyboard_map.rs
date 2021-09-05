use chip8_system::keyboard::Key;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct KeyboardMap {
    keys: HashMap<String, u8>,
}

impl Default for KeyboardMap {
    fn default() -> Self {
        let keys = [
            ("0".to_string(), 0x0),
            ("1".to_string(), 0x1),
            ("2".to_string(), 0x2),
            ("3".to_string(), 0x3),
            ("4".to_string(), 0x4),
            ("5".to_string(), 0x5),
            ("6".to_string(), 0x6),
            ("7".to_string(), 0x7),
            ("8".to_string(), 0x8),
            ("9".to_string(), 0x9),
            ("a".to_string(), 0xA),
            ("b".to_string(), 0xB),
            ("c".to_string(), 0xC),
            ("d".to_string(), 0xD),
            ("e".to_string(), 0xE),
            ("f".to_string(), 0xF),
        ]
        .iter()
        .cloned()
        .collect();
        Self { keys }
    }
}

impl KeyboardMap {
    pub fn from_bytes(b: &[u8]) -> Result<Self, toml::de::Error> {
        toml::from_slice(b)
    }

    pub fn key(&self, s: &str) -> Option<Key> {
        self.keys.get(s).and_then(|&v| Key::from(v))
    }
}

pub fn load_profiles() -> HashMap<String, KeyboardMap> {
    let mut profiles = HashMap::new();
    profiles.insert(
        "default".to_string(),
        KeyboardMap::from_bytes(include_bytes!("../keyboard-profiles/default.toml"))
            .expect("Unable to load profile"),
    );
    profiles.insert(
        "qwerty".to_string(),
        KeyboardMap::from_bytes(include_bytes!("../keyboard-profiles/qwerty.toml"))
            .expect("Unable to load profile"),
    );
    profiles.insert(
        "azerty".to_string(),
        KeyboardMap::from_bytes(include_bytes!("../keyboard-profiles/azerty.toml"))
            .expect("Unable to load profile"),
    );

    profiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bytes() {
        let b = b"[keys]\n0 = 0x0\n1 = 0x1\n";
        let m = KeyboardMap::from_bytes(b).unwrap();

        assert!(matches!(m.key("0"), Some(Key::Key0)));
        assert!(matches!(m.key("1"), Some(Key::Key1)));
    }
}
