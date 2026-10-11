//! Binary data encodings used by the reserved `base64` and `hex` text languages.

use crate::prelude_internal::*;

/// Owned binary data, the Rust counterpart of [`PrimitiveValue::Bytes`].
///
/// `Vec<u8>` already means "array of integers", so binary data uses this dedicated type.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes(pub Vec<u8>);

impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(bytes: Bytes) -> Self {
        bytes.0
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Encodings that turn a tagged text literal into binary data.
///
/// `` base64`SGVsbG8=` `` and `` hex`48656c6c6f` `` (and code blocks tagged the same way)
/// produce [`PrimitiveValue::Bytes`] instead of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BytesEncoding {
    Base64,
    Hex,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BytesDecodeError {
    #[error("invalid {encoding} character `{character}`")]
    InvalidCharacter {
        encoding: &'static str,
        character: char,
    },
    #[error("hex data must have an even number of digits")]
    OddHexLength,
    #[error("base64 data length must be a multiple of 4 (with `=` padding)")]
    InvalidBase64Length,
    #[error("invalid base64 padding")]
    InvalidBase64Padding,
}

impl BytesEncoding {
    /// Returns the encoding reserved for the given language tag.
    pub fn from_language(language: &Language) -> Option<Self> {
        match language {
            Language::Other(lang) => match lang.as_ref() {
                "base64" => Some(Self::Base64),
                "hex" => Some(Self::Hex),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn language_tag(self) -> &'static str {
        match self {
            Self::Base64 => "base64",
            Self::Hex => "hex",
        }
    }

    /// Decodes `text`, ignoring ASCII whitespace (so multi-line code blocks are allowed).
    pub fn decode(self, text: &str) -> Result<Vec<u8>, BytesDecodeError> {
        let compact: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
        match self {
            Self::Base64 => decode_base64(&compact),
            Self::Hex => decode_hex(&compact),
        }
    }

    pub fn encode(self, bytes: &[u8]) -> String {
        match self {
            Self::Base64 => encode_base64(bytes),
            Self::Hex => encode_hex(bytes),
        }
    }
}

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_value(c: u8) -> Result<u8, BytesDecodeError> {
    match c {
        b'A'..=b'Z' => Ok(c - b'A'),
        b'a'..=b'z' => Ok(c - b'a' + 26),
        b'0'..=b'9' => Ok(c - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(BytesDecodeError::InvalidCharacter {
            encoding: "base64",
            character: char::from(c),
        }),
    }
}

fn decode_base64(input: &[u8]) -> Result<Vec<u8>, BytesDecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(BytesDecodeError::InvalidBase64Length);
    }
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let chunk_count = input.len() / 4;
    for (i, chunk) in input.as_chunks::<4>().0.iter().enumerate() {
        let is_last = i + 1 == chunk_count;
        let padding = chunk.iter().rev().take_while(|&&c| c == b'=').count();
        if padding > 2 || (padding > 0 && !is_last) {
            return Err(BytesDecodeError::InvalidBase64Padding);
        }
        let mut acc: u32 = 0;
        for &c in &chunk[..4 - padding] {
            acc = (acc << 6) | u32::from(base64_value(c)?);
        }
        acc <<= 6 * padding as u32;
        let bytes = acc.to_be_bytes();
        let decoded = &bytes[1..4 - padding];
        // Reject non-canonical encodings whose discarded bits are not zero.
        let discarded = &bytes[4 - padding..];
        if discarded.iter().any(|&b| b != 0) {
            return Err(BytesDecodeError::InvalidBase64Padding);
        }
        out.extend_from_slice(decoded);
    }
    Ok(out)
}

fn encode_base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut buf = [0u8; 3];
        buf[..chunk.len()].copy_from_slice(chunk);
        let acc = u32::from_be_bytes([0, buf[0], buf[1], buf[2]]);
        for i in 0..4 {
            if i <= chunk.len() {
                let index = (acc >> (18 - 6 * i)) & 0x3f;
                out.push(char::from(BASE64_ALPHABET[index as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn hex_value(c: u8) -> Result<u8, BytesDecodeError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(BytesDecodeError::InvalidCharacter {
            encoding: "hex",
            character: char::from(c),
        }),
    }
}

fn decode_hex(input: &[u8]) -> Result<Vec<u8>, BytesDecodeError> {
    if !input.len().is_multiple_of(2) {
        return Err(BytesDecodeError::OddHexLength);
    }
    input
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[high, low]| Ok((hex_value(high)? << 4) | hex_value(low)?))
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 0xf)]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trip() {
        for (raw, encoded) in [
            (&b""[..], ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg=="),
            (b"Hello", "SGVsbG8="),
        ] {
            assert_eq!(BytesEncoding::Base64.encode(raw), encoded);
            assert_eq!(BytesEncoding::Base64.decode(encoded).unwrap(), raw);
        }
    }

    #[test]
    fn base64_ignores_whitespace() {
        assert_eq!(
            BytesEncoding::Base64.decode("SGVs\n  bG8=\n").unwrap(),
            b"Hello"
        );
    }

    #[test]
    fn base64_errors() {
        assert_eq!(
            BytesEncoding::Base64.decode("SGVsbG8"),
            Err(BytesDecodeError::InvalidBase64Length)
        );
        assert_eq!(
            BytesEncoding::Base64.decode("Zg==Zm8="),
            Err(BytesDecodeError::InvalidBase64Padding)
        );
        assert_eq!(
            BytesEncoding::Base64.decode("Zh=="),
            Err(BytesDecodeError::InvalidBase64Padding)
        );
        assert_eq!(
            BytesEncoding::Base64.decode("Zm9-"),
            Err(BytesDecodeError::InvalidCharacter {
                encoding: "base64",
                character: '-'
            })
        );
    }

    #[test]
    fn hex_round_trip() {
        assert_eq!(BytesEncoding::Hex.encode(b"Hello"), "48656c6c6f");
        assert_eq!(BytesEncoding::Hex.decode("48656C6c6f").unwrap(), b"Hello");
        assert_eq!(BytesEncoding::Hex.decode("48 65\n6c").unwrap(), b"Hel");
        assert_eq!(
            BytesEncoding::Hex.decode("486"),
            Err(BytesDecodeError::OddHexLength)
        );
        assert_eq!(
            BytesEncoding::Hex.decode("4g"),
            Err(BytesDecodeError::InvalidCharacter {
                encoding: "hex",
                character: 'g'
            })
        );
    }

    #[test]
    fn reserved_languages() {
        assert_eq!(
            BytesEncoding::from_language(&Language::Other("base64".into())),
            Some(BytesEncoding::Base64)
        );
        assert_eq!(
            BytesEncoding::from_language(&Language::Other("hex".into())),
            Some(BytesEncoding::Hex)
        );
        assert_eq!(
            BytesEncoding::from_language(&Language::Other("rust".into())),
            None
        );
        assert_eq!(BytesEncoding::from_language(&Language::Implicit), None);
    }
}
