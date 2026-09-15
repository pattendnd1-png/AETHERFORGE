use sanctuary_core::SanctuaryError;
use std::fmt;

pub const CASC_KEY_BYTES: usize = 16;
pub const LOCAL_INDEX_KEY_BYTES: usize = 9;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EncodingKey([u8; CASC_KEY_BYTES]);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EncodingKeyPrefix([u8; LOCAL_INDEX_KEY_BYTES]);

impl EncodingKey {
    pub fn from_bytes(bytes: [u8; CASC_KEY_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn parse_hex(value: &str) -> Result<Self, SanctuaryError> {
        Ok(Self(parse_hex_array::<CASC_KEY_BYTES>(
            value,
            "encoding key",
        )?))
    }

    pub fn as_bytes(&self) -> &[u8; CASC_KEY_BYTES] {
        &self.0
    }

    pub fn prefix(self) -> EncodingKeyPrefix {
        let mut prefix = [0u8; LOCAL_INDEX_KEY_BYTES];
        prefix.copy_from_slice(&self.0[..LOCAL_INDEX_KEY_BYTES]);
        EncodingKeyPrefix(prefix)
    }

    pub fn to_hex(self) -> String {
        encode_hex(&self.0)
    }
}

impl EncodingKeyPrefix {
    pub fn from_bytes(bytes: [u8; LOCAL_INDEX_KEY_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn parse_hex(value: &str) -> Result<Self, SanctuaryError> {
        Ok(Self(parse_hex_array::<LOCAL_INDEX_KEY_BYTES>(
            value,
            "encoding-key prefix",
        )?))
    }

    pub fn as_bytes(&self) -> &[u8; LOCAL_INDEX_KEY_BYTES] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        encode_hex(&self.0)
    }
}

impl fmt::Debug for EncodingKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("EncodingKey")
            .field(&self.to_hex())
            .finish()
    }
}

impl fmt::Display for EncodingKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

impl fmt::Debug for EncodingKeyPrefix {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("EncodingKeyPrefix")
            .field(&self.to_hex())
            .finish()
    }
}

impl fmt::Display for EncodingKeyPrefix {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

fn parse_hex_array<const N: usize>(value: &str, label: &str) -> Result<[u8; N], SanctuaryError> {
    let value = value.trim();
    let expected = N * 2;
    if value.len() != expected {
        return Err(SanctuaryError::ContentStoreUnreadable(format!(
            "{label} must be {expected} hexadecimal characters, got {}",
            value.len()
        )));
    }

    let bytes = value.as_bytes();
    let mut output = [0u8; N];
    for (index, slot) in output.iter_mut().enumerate() {
        let high = decode_nibble(bytes[index * 2]).ok_or_else(|| {
            SanctuaryError::ContentStoreUnreadable(format!(
                "{label} contains a non-hexadecimal character"
            ))
        })?;
        let low = decode_nibble(bytes[index * 2 + 1]).ok_or_else(|| {
            SanctuaryError::ContentStoreUnreadable(format!(
                "{label} contains a non-hexadecimal character"
            ))
        })?;
        *slot = (high << 4) | low;
    }
    Ok(output)
}

fn decode_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_key_round_trips_hex_and_prefix() {
        let key = EncodingKey::parse_hex("00112233445566778899aabbccddeeff").unwrap();
        assert_eq!(key.to_hex(), "00112233445566778899aabbccddeeff");
        assert_eq!(key.prefix().to_hex(), "001122334455667788");
    }

    #[test]
    fn key_parser_rejects_wrong_length_and_non_hex() {
        assert!(EncodingKey::parse_hex("00").is_err());
        assert!(EncodingKey::parse_hex("gg112233445566778899aabbccddeeff").is_err());
        assert!(EncodingKeyPrefix::parse_hex("0011223344556677zz").is_err());
    }
}
