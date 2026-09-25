//! Bounded form decoding shared by the on-device setup page and host tests.
use alloc::{string::String, vec::Vec};

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
}
