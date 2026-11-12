//! `std.base64` (spec 14.20): bytes as text and back, in the standard
//! alphabet or the URL-safe one (RFC 4648). Decoding is strict, so one
//! value has one text: signed tokens rely on that.

/// Why text isn't base64 (`Base64Error`).
#[derive(Debug, PartialEq)]
pub enum Base64Fail {
    /// A character outside the alphabet, at this character index.
    BadCharacter(usize),
    /// A length no encoding gives.
    BadLength,
    /// `=` where it doesn't belong, or missing where it must be.
    BadPadding,
    /// Bits after the last byte that aren't zero.
    ExtraBits,
}

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// `data` as base64: URL-safe when `url`, with `=` padding when `pad`.
pub fn encode(data: &[u8], url: bool, pad: bool) -> String {
    let digits = if url { URL } else { STANDARD };
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(digits[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
        if pad {
            for _ in chunk.len()..3 {
                out.push('=');
            }
        }
    }
    out
}

/// The bytes `text` encodes, in the alphabet `url` picks. Padding may be
/// left out; when it is there, it must be right.
pub fn decode(text: &str, url: bool) -> Result<Vec<u8>, Base64Fail> {
    let digits = if url { URL } else { STANDARD };
    let chars: Vec<char> = text.chars().collect();
    let pads = chars.iter().rev().take_while(|c| **c == '=').count();
    let body = &chars[..chars.len() - pads];
    if let Some(i) = body.iter().position(|c| *c == '=') {
        return Err(if pads > 0 || i + 1 < body.len() {
            Base64Fail::BadPadding
        } else {
            Base64Fail::BadCharacter(i)
        });
    }
    let mut values = Vec::with_capacity(body.len());
    for (i, c) in body.iter().enumerate() {
        let v = u8::try_from(*c)
            .ok()
            .and_then(|b| digits.iter().position(|d| *d == b))
            .ok_or(Base64Fail::BadCharacter(i))?;
        values.push(v as u32);
    }
    if body.len() % 4 == 1 {
        return Err(Base64Fail::BadLength);
    }
    if pads > 0 && (pads > 2 || !(body.len() + pads).is_multiple_of(4)) {
        return Err(Base64Fail::BadPadding);
    }
    let mut out = Vec::with_capacity(values.len() * 3 / 4);
    for group in values.chunks(4) {
        let n = group
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, v)| n | v << (18 - 6 * i));
        let bytes = group.len() - 1;
        // The bits past the last whole byte must be zero.
        if n & ((1 << (24 - 8 * bytes)) - 1) != 0 {
            return Err(Base64Fail::ExtraBits);
        }
        for i in 0..bytes {
            out.push((n >> (16 - 8 * i)) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc_4648_vectors_go_both_ways() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (plain, coded) in cases {
            assert_eq!(encode(plain.as_bytes(), false, true), coded);
            assert_eq!(decode(coded, false).unwrap(), plain.as_bytes());
            let bare = coded.trim_end_matches('=');
            assert_eq!(encode(plain.as_bytes(), false, false), bare);
            assert_eq!(decode(bare, false).unwrap(), plain.as_bytes());
        }
        assert_eq!(encode(&[0xfb, 0xff], true, false), "-_8");
        assert_eq!(decode("-_8", true).unwrap(), [0xfb, 0xff]);
    }

    #[test]
    fn decoding_is_strict() {
        assert_eq!(decode("Zm9v!", false), Err(Base64Fail::BadCharacter(4)));
        assert_eq!(decode("-_8", false), Err(Base64Fail::BadCharacter(0)));
        assert_eq!(decode("Zm9vY", false), Err(Base64Fail::BadLength));
        assert_eq!(decode("Zg=", false), Err(Base64Fail::BadPadding));
        assert_eq!(decode("Zg===", false), Err(Base64Fail::BadPadding));
        assert_eq!(decode("Z=g=", false), Err(Base64Fail::BadPadding));
        assert_eq!(decode("Zh==", false), Err(Base64Fail::ExtraBits));
    }
}
