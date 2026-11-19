// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Text methods (spec 14.2) that the interpreter and native code share:
//! `capitalize`, `title`, `words`, padding, `truncate`, `is_blank`,
//! `trim_start` and `trim_end`, `last_index_of`, `count`, and removing a
//! prefix or suffix. Positions and lengths count characters.

/// Whether `c` separates words: a space, tab or line break.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// `s` with its first character in upper case.
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Each word with its first character in upper case and the rest in
/// lower case; what is between the words stays as it was.
pub fn title(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut start = true;
    for c in s.chars() {
        if is_space(c) {
            out.push(c);
            start = true;
        } else if start {
            out.extend(c.to_uppercase());
            start = false;
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// The parts of `s` between runs of spaces, tabs and line breaks.
pub fn words(s: &str) -> Vec<&str> {
    s.split(is_space).filter(|w| !w.is_empty()).collect()
}

/// `s` with `fill` added before it (`left`) or after it until it is
/// `width` characters long.
pub fn pad(s: &str, width: usize, fill: char, left: bool) -> String {
    let missing = width.saturating_sub(s.chars().count());
    if missing == 0 {
        return s.to_string();
    }
    let filler: String = std::iter::repeat_n(fill, missing).collect();
    if left {
        filler + s
    } else {
        s.to_string() + &filler
    }
}

/// The most a padded text may be wide (spec 14.2).
pub const MOST_WIDTH: i64 = 1_000_000;

/// At most `max` characters of `s`.
pub fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((byte, _)) => &s[..byte],
        None => s,
    }
}

/// Whether `s` has only spaces, tabs and line breaks, or nothing.
pub fn is_blank(s: &str) -> bool {
    s.chars().all(is_space)
}

/// The character position of the last `part` in `s`.
pub fn last_index_of(s: &str, part: &str) -> Option<usize> {
    s.rfind(part).map(|byte| s[..byte].chars().count())
}

/// How many times `part` is in `s`, without overlaps; an empty `part` is
/// counted between every two characters and at both ends.
pub fn count(s: &str, part: &str) -> usize {
    if part.is_empty() {
        return s.chars().count() + 1;
    }
    s.matches(part).count()
}

/// `s` without `prefix` at its start, if it is there.
pub fn remove_prefix<'s>(s: &'s str, prefix: &str) -> &'s str {
    s.strip_prefix(prefix).unwrap_or(s)
}

/// `s` without `suffix` at its end, if it is there.
pub fn remove_suffix<'s>(s: &'s str, suffix: &str) -> &'s str {
    s.strip_suffix(suffix).unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_method_gives_what_the_spec_says() {
        assert_eq!(capitalize("asha"), "Asha");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("éclair"), "Éclair");
        assert_eq!(title("the tALE of  two"), "The Tale Of  Two");
        assert_eq!(title("they're\tbill"), "They're\tBill");
        assert_eq!(words("  to be\tor\nnot "), ["to", "be", "or", "not"]);
        assert!(words("").is_empty());
        assert_eq!(pad("7", 3, '0', true), "007");
        assert_eq!(pad("ab", 4, '.', false), "ab..");
        assert_eq!(pad("hello", 3, ' ', true), "hello");
        assert_eq!(pad("é", 3, '-', true), "--é", "counts characters");
        assert_eq!(truncate("Kavach", 3), "Kav");
        assert_eq!(truncate("hi", 5), "hi");
        assert_eq!(truncate("नमस्ते", 2), "नम");
        assert!(is_blank(" \t\n") && is_blank(""));
        assert!(!is_blank(" a "));
        assert_eq!(last_index_of("a-b-c", "-"), Some(3));
        assert_eq!(last_index_of("é-é", "é"), Some(2), "a character position");
        assert_eq!(last_index_of("abc", "x"), None);
        assert_eq!(count("banana", "ana"), 1);
        assert_eq!(count("banana", "a"), 3);
        assert_eq!(count("abc", ""), 4);
        assert_eq!(remove_prefix("v1.2", "v"), "1.2");
        assert_eq!(remove_prefix("1.2", "v"), "1.2");
        assert_eq!(remove_suffix("main.kav", ".kav"), "main");
    }
}
