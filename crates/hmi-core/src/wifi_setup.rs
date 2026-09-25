//! Bounded form decoding shared by the on-device setup page and host tests.
use alloc::{string::String, vec, vec::Vec};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Credentials {
    pub ssid: String,
    pub password: String,
}

impl Credentials {
    pub fn validate(self) -> Option<Self> {
        if self.ssid.is_empty()
            || self.ssid.len() > 32
            || self.password.len() > 63
            || (!self.password.is_empty() && self.password.len() < 8)
            || self.ssid.bytes().any(|b| b < 32 || b == 127)
            || self.password.bytes().any(|b| b < 32 || b == 127)
        {
            None
        } else {
            Some(self)
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(3 + self.ssid.len() + self.password.len());
        result.extend([1, self.ssid.len() as u8, self.password.len() as u8]);
        result.extend_from_slice(self.ssid.as_bytes());
        result.extend_from_slice(self.password.as_bytes());
        result
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 3
            || bytes[0] != 1
            || bytes.len() != 3 + bytes[1] as usize + bytes[2] as usize
        {
            return None;
        }
        let ssid = String::from_utf8(bytes[3..3 + bytes[1] as usize].to_vec()).ok()?;
        let password = String::from_utf8(bytes[3 + bytes[1] as usize..].to_vec()).ok()?;
        Self { ssid, password }.validate()
    }
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn decode(bytes: &[u8]) -> Option<String> {
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => result.push(b' '),
            b'%' => {
                let a = hex(*bytes.get(i + 1)?)?;
                let b = hex(*bytes.get(i + 2)?)?;
                result.push((a << 4) | b);
                i += 2;
            }
            b => result.push(b),
        }
        i += 1;
    }
    String::from_utf8(result).ok()
}

pub fn parse_form(body: &[u8]) -> Option<Credentials> {
    if body.len() > 256 {
        return None;
    }
    let mut ssid = None;
    let mut password = None;
    for pair in body.split(|&b| b == b'&') {
        let separator = pair.iter().position(|&b| b == b'=')?;
        let (key, rest) = pair.split_at(separator);
        let value = &rest[1..];
        match key {
            b"ssid" if ssid.is_none() => ssid = Some(decode(value)?),
            b"password" if password.is_none() => password = Some(decode(value)?),
            _ => return None,
        }
    }
    Credentials {
        ssid: ssid?,
        password: password?,
    }
    .validate()
}

/// At most three user-saved networks, plus the compiled fallbacks.
/// A newly connected network takes priority without erasing older entries.
pub struct Profiles {
    saved: Vec<Credentials>,
    fallbacks: Vec<Credentials>,
}

impl Profiles {
    pub fn new(fallback: Credentials) -> Self {
        Self::with_fallbacks(vec![fallback])
    }

    pub fn with_fallbacks(fallbacks: Vec<Credentials>) -> Self {
        Self {
            saved: Vec::new(),
            fallbacks: fallbacks
                .into_iter()
                .filter_map(Credentials::validate)
                .collect(),
        }
    }

    pub fn decode(blob: &[u8], fallbacks: Vec<Credentials>) -> Self {
        let mut profiles = Self::with_fallbacks(fallbacks);
        if blob.first() == Some(&1) {
            if let Some(legacy) = Credentials::decode(blob) {
                profiles.promote(legacy);
            }
            return profiles;
        }
        if blob.len() < 2 || blob[0] != 2 || blob[1] > 3 {
            return profiles;
        }
        let mut offset = 2;
        let mut entries = Vec::new();
        for _ in 0..blob[1] {
            let Some(header) = blob.get(offset..offset + 2) else {
                return profiles;
            };
            let length = 3 + header[0] as usize + header[1] as usize;
            let mut encoded = Vec::with_capacity(length);
            encoded.push(1);
            encoded.extend_from_slice(&blob[offset..offset + 2]);
            offset += 2;
            let Some(content) = blob.get(offset..offset + length - 3) else {
                return profiles;
            };
            encoded.extend_from_slice(content);
            let Some(entry) = Credentials::decode(&encoded) else {
                return profiles;
            };
            entries.push(entry);
            offset += length - 3;
        }
        if offset != blob.len() {
            return profiles;
        }
        // Reverse because promote inserts at the front.
        for entry in entries.into_iter().rev() {
            profiles.promote(entry);
        }
        profiles
    }

    pub fn promote(&mut self, entry: Credentials) {
        self.saved.retain(|old| old.ssid != entry.ssid);
        self.saved.insert(0, entry);
        self.saved.truncate(3);
    }

    pub fn ordered(&self) -> Vec<Credentials> {
        let mut networks = self.saved.clone();
        for fallback in &self.fallbacks {
            if !networks.iter().any(|item| item.ssid == fallback.ssid) {
                networks.push(fallback.clone());
            }
        }
        networks
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut blob = vec![2, self.saved.len() as u8];
        for entry in &self.saved {
            let encoded = entry.encode();
            blob.extend_from_slice(&encoded[1..]);
        }
        blob
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_form_accepts_escaped_network_and_rejects_bad_values() {
        let parsed = parse_form(b"ssid=Example_2G&password=space+and%25more").unwrap();
        assert_eq!(parsed.ssid, "Example_2G");
        assert_eq!(parsed.password, "space and%more");
        for bad in [
            b"ssid=&password=password".as_slice(),
            b"ssid=x&password=short",
            b"ssid=x&password=%00hi12345",
            b"ssid=x&password=%GGhi12345",
            b"ssid=x&ssid=y&password=12345678",
            b"password=12345678&extra=z&ssid=x",
        ] {
            assert!(parse_form(bad).is_none());
        }
        assert!(parse_form(&vec![b'x'; 257]).is_none());
        assert_eq!(parse_form(b"ssid=guest&password=").unwrap().password, "");
        let blob = parsed.encode();
        assert_eq!(Credentials::decode(&blob), Some(parsed));
        assert!(Credentials::decode(&blob[..blob.len() - 1]).is_none());
        assert!(Credentials::decode(&[0, 1, 0, b'a']).is_none());
    }

    #[test]
    fn new_home_network_preserves_previous_one_and_roundtrips() {
        let original = Credentials {
            ssid: "Original_2G".into(),
            password: "original-pass".into(),
        };
        let alternate = Credentials {
            ssid: "ExampleHome_2G".into(),
            password: "another-pass".into(),
        };
        let mut profiles = Profiles::new(original.clone());
        assert_eq!(profiles.ordered(), [original.clone()]);
        profiles.promote(alternate.clone());
        assert_eq!(profiles.ordered(), [alternate.clone(), original.clone()]);
        let restored = Profiles::decode(&profiles.encode(), vec![original.clone()]);
        assert_eq!(restored.ordered(), profiles.ordered());
        profiles.promote(original.clone());
        assert_eq!(profiles.ordered(), [original.clone(), alternate]);
    }

    #[test]
    fn old_format_and_corruption_do_not_delete_compiled_fallback() {
        let fallback = Credentials {
            ssid: "factory".into(),
            password: "factorypass".into(),
        };
        let saved = Credentials {
            ssid: "saved".into(),
            password: "savedpass".into(),
        };
        assert_eq!(
            Profiles::decode(&saved.encode(), vec![fallback.clone()]).ordered(),
            [saved.clone(), fallback.clone()]
        );
        for invalid in [&[][..], &[2, 4], &[2, 1, 4, 8, 0, 1], &[9, 0]] {
            assert_eq!(
                Profiles::decode(invalid, vec![fallback.clone()]).ordered(),
                [fallback.clone()]
            );
        }
        let mut profiles = Profiles::new(fallback);
        for i in 0..6 {
            profiles.promote(Credentials {
                ssid: format!("test{i}"),
                password: "12345678".into(),
            });
        }
        assert_eq!(profiles.ordered().len(), 4);
    }

    #[test]
    fn two_compiled_networks_and_saved_one_survive_reorder() {
        let primary = Credentials {
            ssid: "main".into(),
            password: "mainpassword".into(),
        };
        let secondary = Credentials {
            ssid: "backup".into(),
            password: "backuppassword".into(),
        };
        let added = Credentials {
            ssid: "guest".into(),
            password: "guestpassword".into(),
        };
        let mut profiles = Profiles::with_fallbacks(vec![primary.clone(), secondary.clone()]);
        assert_eq!(profiles.ordered(), [primary.clone(), secondary.clone()]);
        profiles.promote(added.clone());
        assert_eq!(
            Profiles::decode(&profiles.encode(), vec![primary.clone(), secondary.clone()])
                .ordered(),
            [added, primary, secondary]
        );
    }
}
