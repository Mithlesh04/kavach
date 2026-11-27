// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Which optional passes run, read once from `KAVACH_PASSES` (spec
//! 10.11) when a program is lowered. The passes a program needs to
//! behave as written always run; only the ones that make it faster can
//! be turned off, to compare a program built with and without them.
//!
//! `KAVACH_PASSES` is a comma-separated list, read left to right from
//! "every pass on": `all` turns every optional pass on, `none` turns
//! them off, `-name` turns one off and `+name` (or `name`) one on. So
//! `none,+sroa` runs only `sroa`, and `-sroa,-fuse` all but two.
//!
//! `moves` is on even with `none`: without it the program behaves the
//! same, but copies a list each time a variable passes it on, so a loop
//! that grows a list takes quadratic time. `-moves` turns it off too.

/// The passes that only make a program faster, in the order they run.
pub const OPTIONAL: [&str; 20] = [
    "unrecurse",
    "inline",
    "inout_stash",
    "checked_fits",
    "gpu_stay",
    "gpu_later",
    "devirt",
    "unused",
    "tuple_lets",
    "loop_fields",
    "sroa",
    "coalesce",
    "loop_lens",
    "jam",
    "fuse",
    "version",
    "ranges",
    "prefetch",
    "par",
    "counters",
];

/// The optional passes that run, and the functions `sroa` may split
/// records in (`KAVACH_SROA_ONLY`; all when `None`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassSet {
    off: Vec<&'static str>,
    sroa_only: Option<Vec<String>>,
}

impl Default for PassSet {
    /// Every pass on.
    fn default() -> Self {
        PassSet {
            off: Vec::new(),
            sroa_only: None,
        }
    }
}

impl PassSet {
    /// The passes `KAVACH_PASSES` and `KAVACH_SROA_ONLY` ask for. A name
    /// that is no pass is left out, with a warning naming the passes.
    pub fn from_env() -> PassSet {
        let mut set = match std::env::var("KAVACH_PASSES") {
            Ok(text) => PassSet::parse(&text).unwrap_or_else(|(set, unknown)| {
                eprintln!(
                    "warning: KAVACH_PASSES: no pass is named {}; the passes are `moves`, {}",
                    unknown.join(", "),
                    OPTIONAL.map(|p| format!("`{p}`")).join(", ")
                );
                set
            }),
            Err(_) => PassSet::default(),
        };
        set.sroa_only = std::env::var("KAVACH_SROA_ONLY")
            .ok()
            .map(|names| names.split(',').map(str::to_string).collect());
        set
    }

    /// Reads a `KAVACH_PASSES` value. On a name that is no pass, the set
    /// without it, and each such name.
    pub fn parse(text: &str) -> Result<PassSet, (PassSet, Vec<String>)> {
        let mut set = PassSet::default();
        let mut unknown = Vec::new();
        for item in text.split(',').map(str::trim).filter(|i| !i.is_empty()) {
            match item {
                "all" => set.off.clear(),
                "none" => set.off = OPTIONAL.to_vec(),
                _ => {
                    let (on, name) = match item.strip_prefix('-') {
                        Some(name) => (false, name),
                        None => (true, item.strip_prefix('+').unwrap_or(item)),
                    };
                    let Some(&known) = OPTIONAL.iter().chain(&["moves"]).find(|p| **p == name)
                    else {
                        unknown.push(format!("`{name}`"));
                        continue;
                    };
                    set.off.retain(|p| *p != known);
                    if !on {
                        set.off.push(known);
                    }
                }
            }
        }
        if unknown.is_empty() {
            Ok(set)
        } else {
            Err((set, unknown))
        }
    }

    /// Whether the pass `name` runs.
    pub fn on(&self, name: &str) -> bool {
        !self.off.contains(&name)
    }

    /// Whether `sroa` runs in the function named `function`.
    pub fn sroa_in(&self, function: &str) -> bool {
        self.on("sroa")
            && (self.sroa_only.as_ref()).is_none_or(|names| names.iter().any(|n| n == function))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_turn_passes_on_and_off_left_to_right() {
        let cases: [(&str, &[&str], &[&str]); 7] = [
            ("", &OPTIONAL, &[]),
            ("all", &OPTIONAL, &[]),
            ("none", &["moves"], &OPTIONAL),
            (
                "-sroa,-fuse",
                &["inline", "moves", "ranges"],
                &["sroa", "fuse"],
            ),
            ("none,+sroa", &["sroa", "moves"], &["inline", "fuse"]),
            ("none, sroa, -moves", &["sroa"], &["inline", "moves"]),
            ("none,all", &OPTIONAL, &[]),
        ];
        for (text, on, off) in cases {
            let set = PassSet::parse(text).expect(text);
            for p in on {
                assert!(set.on(p), "{text:?}: {p} should run");
            }
            for p in off {
                assert!(!set.on(p), "{text:?}: {p} should not run");
            }
        }
    }

    #[test]
    fn a_name_that_is_no_pass_is_reported() {
        let (set, unknown) = PassSet::parse("-sroa,-prune,fast").unwrap_err();
        assert_eq!(unknown, ["`prune`", "`fast`"]);
        assert!(!set.on("sroa"));
        assert!(set.on("fuse"));
    }

    #[test]
    fn sroa_only_names_the_functions() {
        let mut set = PassSet::default();
        assert!(set.sroa_in("main"));
        set.sroa_only = Some(vec!["step".into()]);
        assert!(set.sroa_in("step") && !set.sroa_in("main"));
        set.off.push("sroa");
        assert!(!set.sroa_in("step"));
    }
}
