//! Unicode normal forms (UAX #15) for `std.text` (spec 14.9, decision
//! 0102): `text.nfc`, `nfd`, `nfkc` and `nfkd`, the same in the
//! interpreter and in native code.

use unicode_normalization::UnicodeNormalization;

/// `s` in normal form `form`: 0 NFC, 1 NFD, 2 NFKC, 3 NFKD.
pub fn normalize(s: &str, form: i64) -> String {
    match form {
        0 => s.nfc().collect(),
        1 => s.nfd().collect(),
        2 => s.nfkc().collect(),
        _ => s.nfkd().collect(),
    }
}

/// The number `normalize` takes for function `name` of `std.text`.
pub fn form_of(name: &str) -> Option<i64> {
    match name {
        "nfc" => Some(0),
        "nfd" => Some(1),
        "nfkc" => Some(2),
        "nfkd" => Some(3),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn each_form_gives_unicodes_spelling() {
        let composed = "\u{e9}"; // é, one code point
        let decomposed = "e\u{301}"; // e and a combining accent
        // (text, form, expected)
        let cases = [
            (decomposed, 0, composed),
            (composed, 1, decomposed),
            ("\u{fb01}le", 2, "file"),                 // the ﬁ ligature
            ("\u{ff11}\u{ff12}", 2, "12"),             // full-width digits
            ("\u{fb01}", 0, "\u{fb01}"),               // NFC keeps a ligature
            ("\u{1e9b}\u{323}", 3, "s\u{323}\u{307}"), // UAX #15's own example
            ("plain", 2, "plain"),
        ];
        for (text, form, want) in cases {
            assert_eq!(normalize(text, form), want, "{text:?} in form {form}");
        }
    }
}
