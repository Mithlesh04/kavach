// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! SVD names in Kavach's styles (spec 1.5): `GPIOA` is the type `Gpioa`,
//! `MODER0` the method `moder0`. A keyword gets `_` at the end.

/// The keywords of spec 1.6, which can't be names.
const KEYWORDS: &[&str] = &[
    "agent",
    "and",
    "arena",
    "as",
    "assert",
    "break",
    "capability",
    "catch",
    "const",
    "continue",
    "deprecated",
    "else",
    "ensure",
    "example",
    "extend",
    "extern",
    "fail",
    "false",
    "fn",
    "for",
    "if",
    "in",
    "interface",
    "invariant",
    "let",
    "match",
    "none",
    "not",
    "or",
    "pub",
    "require",
    "return",
    "secret",
    "self",
    "spawn",
    "true",
    "try",
    "type",
    "unsafe",
    "use",
    "uses",
    "var",
    "while",
    "with",
];

/// The words of `name`: runs of letters and digits.
fn words(name: &str) -> Vec<String> {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// `snake_case`: `DMA_CCR1` is `dma_ccr1`.
pub fn snake(name: &str) -> String {
    let mut s = words(name).join("_");
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert(0, 'r');
    }
    if KEYWORDS.contains(&s.as_str()) {
        s.push('_');
    }
    s
}

/// `PascalCase`: `TIM1_CC` is `Tim1Cc`.
pub fn pascal(name: &str) -> String {
    let mut s: String = words(name)
        .iter()
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert(0, 'P');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_kavachs_styles() {
        for (svd, s, p) in [
            ("GPIOA", "gpioa", "Gpioa"),
            ("TIM1_CC", "tim1_cc", "Tim1Cc"),
            ("IN", "in_", "In"),
            ("2ND", "r2nd", "P2nd"),
            ("CH[0]", "ch_0", "Ch0"),
        ] {
            assert_eq!((snake(svd).as_str(), pascal(svd).as_str()), (s, p), "{svd}");
        }
    }
}
