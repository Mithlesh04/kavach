// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What the compiler is told about a file: read from `kavach.toml`, and
//! passed on to the checker.

use crate::Config;
use std::path::Path;

/// What the compiler is told about a file, usually from `kavach.toml`
/// (see [`Config::options_for`]).
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// `unsafe = true` in `[package]` (spec 6.4).
    pub allow_unsafe: bool,
    /// The dependencies allowed to use `unsafe` (spec 10.12).
    pub unsafe_packages: Vec<String>,
    /// Each dependency with the capabilities it is granted (spec 10.12).
    pub granted: Vec<(String, Vec<String>)>,
    /// Each package's default capabilities, `""` for the program's own;
    /// one left out has `["Console"]` (spec 8.1, rule 7).
    pub defaults: Vec<(String, Vec<String>)>,
    pub limits: crate::Limits,
    /// Whether `kavach.toml` has a `[log.otlp]` collector (14.12), which an
    /// agent's `Audit(otlp: true)` needs (spec 8.6).
    pub log_otlp: bool,
    /// Compiled as a library, as `kavach build --lib` does (spec 10.10):
    /// it may offer names with `pub(out)` even outside `src/lib.kav`.
    pub library: bool,
}

impl Options {
    /// What the checker is told.
    pub(crate) fn checker(&self) -> kavach_checker::Options {
        kavach_checker::Options {
            allow_unsafe: self.allow_unsafe,
            unsafe_packages: self.unsafe_packages.clone(),
            granted: self.granted.clone(),
            defaults: self.defaults.clone(),
        }
    }
}

impl Config {
    /// The compiler options for `file`, with its own line limit if
    /// `[limits.files]` gives it one.
    pub fn options_for(&self, file: &Path) -> Options {
        let log_otlp = self.log.otlp.is_some();
        let mut limits = self.limits.clone();
        if let Some(root) = &self.root {
            let full = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
            if let Ok(rel) = full.strip_prefix(root) {
                let rel = rel.to_string_lossy().replace('\\', "/");
                if let Some(n) = self.limits.files.get(&rel) {
                    limits.file_lines = *n;
                }
            }
        }
        let packages: Vec<crate::deps::Package> = self
            .root
            .as_deref()
            .map(|r| crate::deps::resolve(r, false))
            .and_then(Result::ok)
            .unwrap_or_default();
        let unsafe_packages = packages
            .iter()
            .filter(|p| p.allow_unsafe)
            .map(|p| p.name.clone())
            .collect();
        let defaults = crate::default_caps::defaults(&self.capabilities, &packages);
        let granted = packages.into_iter().map(|p| (p.name, p.granted)).collect();
        Options {
            allow_unsafe: self.package.allow_unsafe,
            unsafe_packages,
            granted,
            defaults,
            limits,
            log_otlp,
            library: false,
        }
    }
}
