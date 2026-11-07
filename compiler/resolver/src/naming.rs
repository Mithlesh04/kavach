//! Naming styles (spec 1.5): `snake_case` for values, `PascalCase` for
//! types, and a warning for names that mix look-alike scripts.

/// The `snake_case` spelling of `name`, if it isn't already snake_case.
pub(crate) fn snake_case_fix(name: &str) -> Option<String> {
    if !name.chars().any(char::is_uppercase) {
        return None;
    }
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// The `PascalCase` spelling of `name`, if it isn't already PascalCase.
/// Scripts without capital letters (such as Devanagari) are accepted.
pub(crate) fn pascal_case_fix(name: &str) -> Option<String> {
    let starts_lower = name.chars().next().is_some_and(char::is_lowercase);
    if !starts_lower && !name.contains('_') {
        return None;
    }
    let fixed: String = name
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let first: String = chars
                .next()
                .map(|c| c.to_uppercase().collect())
                .unwrap_or_default();
            first + chars.as_str()
        })
        .collect();
    Some(fixed)
}

/// The scripts of the letters in `name`, if it mixes more than one.
pub(crate) fn mixed_scripts(name: &str) -> Option<(&'static str, &'static str)> {
    let mut first: Option<&'static str> = None;
    for script in name.chars().filter_map(script) {
        match first {
            None => first = Some(script),
            Some(f) if f != script => return Some((f, script)),
            Some(_) => {}
        }
    }
    None
}

fn script(c: char) -> Option<&'static str> {
    if !c.is_alphabetic() {
        return None;
    }
    Some(match c as u32 {
        0x0000..=0x024F | 0x1E00..=0x1EFF => "Latin",
        0x0370..=0x03FF | 0x1F00..=0x1FFF => "Greek",
        0x0400..=0x052F => "Cyrillic",
        0x0530..=0x058F => "Armenian",
        0x0590..=0x05FF => "Hebrew",
        0x0600..=0x06FF => "Arabic",
        0x0900..=0x097F => "Devanagari",
        0x0980..=0x0DFF => "Indic",
        0x0E00..=0x0E7F => "Thai",
        // Japanese mixes kana and kanji in one word, so they count as one.
        0x3040..=0x30FF | 0x4E00..=0x9FFF => "CJK",
        0xAC00..=0xD7AF => "Hangul",
        _ => "other",
    })
}
