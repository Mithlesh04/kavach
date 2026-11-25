// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `kavach svd chip.svd` (spec 6.5): a Kavach module of typed register
//! blocks from a CMSIS-SVD file, as `svd2rust` makes Rust ones
//! (decision 0145). `xml.rs` reads the XML, `model.rs` the device in
//! it, `emit.rs` writes the module, `names.rs` makes names Kavach's.

mod emit;
mod model;
mod names;
mod xml;

/// The Kavach module for the SVD file `text`; `source` is the file's
/// name, for the module's first line. Or what is wrong with the file.
pub fn generate(text: &str, source: &str) -> Result<String, String> {
    let root = xml::parse(text)?;
    let device = model::device(&root)?;
    emit::module(&device, source)
}
