// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `kavach.lock` (spec 10.12): reading the commit it records for each git
//! dependency, and writing it for the dependencies found, with each path
//! dependency's folder relative to the project.

use crate::deps::Package;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The git commits `kavach.lock` records: by name, the repository and
/// commit.
pub(crate) fn read_lock(root: &Path) -> BTreeMap<String, (String, String)> {
    let Ok(text) = std::fs::read_to_string(root.join("kavach.lock")) else {
        return BTreeMap::new();
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return BTreeMap::new();
    };
    let packages = table.get("package").and_then(|p| p.as_array());
    packages
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let s = |k: &str| p.get(k)?.as_str().map(str::to_string);
            Some((s("name")?, (s("git")?, s("commit")?)))
        })
        .collect()
}

/// `kavach.lock` for `packages`: one entry per dependency, in name order.
/// In a workspace, the git entries of other members (`kept`) stay.
pub(crate) fn lock_text(
    root: &Path,
    packages: &[Package],
    kept: Option<&BTreeMap<String, (String, String)>>,
) -> String {
    let mut out = String::from(
        "# The exact source of each dependency (spec 10.12). Written by kavach: commit it, don't edit it.\n",
    );
    let mut sorted: Vec<Package> = packages.to_vec();
    for (name, (url, commit)) in kept.into_iter().flatten() {
        if !sorted.iter().any(|p| &p.name == name) {
            sorted.push(Package {
                name: name.clone(),
                root: PathBuf::new(),
                allow_unsafe: false,
                granted: Vec::new(),
                git: Some((url.clone(), commit.clone())),
            });
        }
    }
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    for p in &sorted {
        out.push_str(&format!("\n[[package]]\nname = \"{}\"\n", p.name));
        match &p.git {
            Some((url, commit)) => {
                out.push_str(&format!("git = \"{url}\"\ncommit = \"{commit}\"\n"))
            }
            None => out.push_str(&format!("path = \"{}\"\n", relative(root, &p.root))),
        }
    }
    out
}

/// `path` without Windows' `\\?\` prefix, which `canonicalize` adds.
pub(crate) fn plain(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text))
}

/// `to` written relative to the folder `from`, with `..` where needed.
fn relative(from: &Path, to: &Path) -> String {
    let (from, to) = (plain(from), plain(to));
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 {
        return to.to_string_lossy().replace('\\', "/");
    }
    let mut parts: Vec<String> = vec!["..".into(); a.len() - common];
    parts.extend(
        b[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().to_string()),
    );
    if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    }
}
