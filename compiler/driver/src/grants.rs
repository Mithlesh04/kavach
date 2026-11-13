//! What a dependency is granted (spec 10.12): whole capabilities, or the
//! parts of one that `uses` can name, such as `Fs.read`.

/// The parts of a capability a dependency can be granted alone, as
/// `uses` takes them.
pub(crate) const NARROW: [&str; 2] = ["Fs.read", "Fs.write"];

/// What of `cap`, asked for in a dependency's `can`, the package above
/// it was granted (`above`): all of it, a part of it (`Fs.read` of `Fs`),
/// or nothing.
pub(crate) fn meet(cap: &str, above: &[String]) -> Vec<String> {
    let base = cap.split_once('.').map_or(cap, |(b, _)| b);
    if above.iter().any(|a| a == cap || a == base) {
        return vec![cap.to_string()];
    }
    let part = |a: &&String| a.split_once('.').is_some_and(|(b, _)| b == cap);
    above.iter().filter(part).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::meet;

    #[test]
    fn a_dependency_gets_no_more_than_the_package_above_it() {
        let above = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let cases: &[(&str, &[&str], &[&str])] = &[
            ("Net", &["Net", "Env"], &["Net"]),
            ("Net", &["Env"], &[]),
            ("Fs.read", &["Fs"], &["Fs.read"]),
            ("Fs", &["Fs.read"], &["Fs.read"]),
            ("Fs.write", &["Fs.read"], &[]),
            ("Fs", &["Fs.read", "Fs.write"], &["Fs.read", "Fs.write"]),
        ];
        for (cap, given, want) in cases {
            assert_eq!(
                meet(cap, &above(given)),
                above(want),
                "{cap} under {given:?}"
            );
        }
    }
}
