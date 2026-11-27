// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `[capabilities] default` (spec 8.1, rule 7; decision 0148): what every
//! function of a package may use without writing `uses`. Left out, it is
//! `["Console"]`.

use serde::Deserialize;

/// `[capabilities]` in `kavach.toml`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CapabilitiesConfig {
    /// `None` when left out: then `["Console"]`.
    pub default: Option<Vec<String>>,
}

/// The capabilities that are always written by name, never defaults.
const NEVER: [&str; 3] = ["System.critical", "Approvals", "Foreign"];

/// What is wrong with `list` as default capabilities, if anything.
pub(crate) fn wrong(list: &[String]) -> Option<String> {
    for cap in list {
        if let Some(never) = NEVER.iter().find(|n| kavach_resolver::cap_covers(n, cap)) {
            return Some(format!(
                "`{cap}` can't be a default capability: `{never}` is always written by name, in `uses` (spec 8.1)"
            ));
        }
        if !kavach_resolver::CAPABILITIES.contains(&cap.as_str())
            && !crate::grants::NARROW.contains(&cap.as_str())
        {
            let known = kavach_resolver::CAPABILITIES.join(", ");
            return Some(format!(
                "`{cap}` in `default` is not a capability: they are {known}, and `Fs.read` or `Fs.write` for part of `Fs` (spec 8.1)"
            ));
        }
    }
    None
}

/// The error for a wrong `[capabilities] default` in the `kavach.toml`
/// whose text is `text`, at `path`.
pub(crate) fn check(config: &CapabilitiesConfig, text: &str, path: &str) -> Result<(), String> {
    let Some(message) = config.default.as_deref().and_then(wrong) else {
        return Ok(());
    };
    let line = (text.lines())
        .position(|l| l.trim_start().starts_with("default"))
        .map_or(1, |n| n + 1);
    Err(format!("{message}\n  --> {path}:{line}"))
}

/// A dependency's own `[capabilities] default`, from its `kavach.toml`
/// read as `table`; `None` when left out.
pub(crate) fn of_table(table: &toml::Table, name: &str) -> Result<Option<Vec<String>>, String> {
    let Some(value) = table.get("capabilities").and_then(|c| c.get("default")) else {
        return Ok(None);
    };
    let list: Option<Vec<String>> =
        (value.as_array()).and_then(|a| a.iter().map(|v| v.as_str().map(str::to_string)).collect());
    let Some(list) = list else {
        return Err(format!(
            "dependency `{name}`: `default` in `[capabilities]` is a list of capability names, such as `default = [\"Console\"]`"
        ));
    };
    match wrong(&list) {
        Some(message) => Err(format!("dependency `{name}`: {message}")),
        None => Ok(Some(list)),
    }
}

/// Each package with its default capabilities, `""` for the program's
/// own; a package left out has `["Console"]`.
pub(crate) fn defaults(
    own: &CapabilitiesConfig,
    packages: &[crate::deps::Package],
) -> Vec<(String, Vec<String>)> {
    let own = own.default.clone().map(|d| (String::new(), d));
    let deps = (packages.iter()).filter_map(|p| Some((p.name.clone(), p.defaults.clone()?)));
    own.into_iter().chain(deps).collect()
}

#[cfg(test)]
mod tests {
    use super::wrong;

    #[test]
    fn defaults_are_built_in_capabilities_written_by_name_only_when_needed() {
        let list = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        for (given, fine) in [
            (list(&["Console"]), true),
            (list(&[]), true),
            (list(&["Console", "Clock", "Fs.read"]), true),
            (list(&["System.critical"]), false),
            (list(&["System.critical.read"]), false),
            (list(&["Approvals"]), false),
            (list(&["Foreign"]), false),
            (list(&["Printer"]), false),
        ] {
            assert_eq!(wrong(&given).is_none(), fine, "{given:?}");
        }
    }
}
