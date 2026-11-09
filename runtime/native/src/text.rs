//! The contents of a text value. Short text (up to `INLINE` bytes, which
//! covers most words, names and keys) is stored inside the object itself;
//! longer text in a heap buffer. So splitting text into a million words
//! makes a million objects, not two million allocations.

/// The longest text kept inside the object.
const INLINE: usize = 30;

#[derive(Clone)]
pub enum Text {
    Short { len: u8, bytes: [u8; INLINE] },
    Long(String),
}

impl Text {
    #[allow(unsafe_code)]
    pub fn as_str(&self) -> &str {
        match self {
            // SAFETY: short text is only ever filled with whole `&str`
            // values (`of`, `push_str`), so its bytes are valid UTF-8.
            Text::Short { len, bytes } => unsafe {
                std::str::from_utf8_unchecked(&bytes[..*len as usize])
            },
            Text::Long(s) => s,
        }
    }

    pub fn of(s: &str) -> Text {
        if s.len() > INLINE {
            return Text::Long(s.to_string());
        }
        let mut bytes = [0; INLINE];
        bytes[..s.len()].copy_from_slice(s.as_bytes());
        Text::Short {
            len: s.len() as u8,
            bytes,
        }
    }

    /// Empty text with room for `n` bytes.
    pub fn with_capacity(n: usize) -> Text {
        if n > INLINE {
            Text::Long(String::with_capacity(n))
        } else {
            Text::of("")
        }
    }

    pub fn push_str(&mut self, more: &str) {
        match self {
            Text::Short { len, bytes } if *len as usize + more.len() <= INLINE => {
                let at = *len as usize;
                bytes[at..at + more.len()].copy_from_slice(more.as_bytes());
                *len += more.len() as u8;
            }
            Text::Short { .. } => {
                let mut s = String::with_capacity((self.as_str().len() + more.len()) * 2);
                s.push_str(self.as_str());
                s.push_str(more);
                *self = Text::Long(s);
            }
            Text::Long(s) => s.push_str(more),
        }
    }
}

impl From<String> for Text {
    fn from(s: String) -> Text {
        if s.len() <= INLINE {
            Text::of(&s)
        } else {
            Text::Long(s)
        }
    }
}

impl std::fmt::Write for Text {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_stays_inside_and_grows_out() {
        let mut t = Text::of("word");
        assert!(matches!(t, Text::Short { .. }));
        t.push_str("42");
        assert_eq!(t.as_str(), "word42");
        t.push_str(&"x".repeat(40));
        assert!(matches!(t, Text::Long(_)));
        assert_eq!(t.as_str().len(), 46);
        assert_eq!(Text::from("ünïcödé".to_string()).as_str(), "ünïcödé");
    }
}
