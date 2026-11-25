// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A CMSIS-SVD file's device, peripherals, registers and fields, read
//! from its XML. A register's size, access and reset value come from the
//! register, else its peripheral, else the device. A register with `dim`
//! becomes one register for each index. Clusters and enumerated values
//! aren't read: each is named in `skipped`.

use crate::xml::Element;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

impl Access {
    pub fn reads(self) -> bool {
        self != Access::WriteOnly
    }

    pub fn writes(self) -> bool {
        self != Access::ReadOnly
    }
}

pub struct Device {
    pub name: String,
    pub peripherals: Vec<Peripheral>,
}

pub struct Peripheral {
    pub name: String,
    pub description: String,
    pub base: u64,
    /// The peripheral whose registers this one has, when it has none of
    /// its own.
    pub derived_from: Option<String>,
    pub registers: Vec<Register>,
    /// What wasn't read, for a comment in the module.
    pub skipped: Vec<String>,
}

pub struct Register {
    pub name: String,
    pub description: String,
    pub offset: u64,
    pub size: u32,
    pub access: Access,
    pub reset: u64,
    pub fields: Vec<Field>,
}

#[derive(Clone)]
pub struct Field {
    pub name: String,
    pub description: String,
    pub lsb: u32,
    pub width: u32,
    pub access: Access,
}

/// Size, access and reset value, as the device or a peripheral gives
/// them to what is inside it.
#[derive(Clone, Copy)]
struct Defaults {
    size: u32,
    access: Access,
    reset: u64,
}

/// A number as SVD writes it: `0x1F`, `#0101` (binary) or `31`.
pub fn number(text: &str) -> Option<u64> {
    let t = text.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else if let Some(bin) = t.strip_prefix('#').or_else(|| t.strip_prefix("0b")) {
        u64::from_str_radix(bin, 2).ok()
    } else {
        t.parse().ok()
    }
}

fn access(text: &str) -> Result<Access, String> {
    Ok(match text.trim() {
        "read-only" => Access::ReadOnly,
        "write-only" | "writeOnce" => Access::WriteOnly,
        "read-write" | "read-writeOnce" => Access::ReadWrite,
        other => return Err(format!("unknown access `{other}`")),
    })
}

/// The number in child `name` of `e`, if there is one.
fn number_of(e: &Element, name: &str, whose: &str) -> Result<Option<u64>, String> {
    match e.text_of(name) {
        None => Ok(None),
        Some(t) => number(&t)
            .map(Some)
            .ok_or_else(|| format!("{whose}: `<{name}>` isn't a number: `{t}`")),
    }
}

fn defaults(e: &Element, outer: Defaults, whose: &str) -> Result<Defaults, String> {
    Ok(Defaults {
        size: number_of(e, "size", whose)?.map_or(outer.size, |n| n as u32),
        access: match e.text_of("access") {
            Some(a) => access(&a).map_err(|w| format!("{whose}: {w}"))?,
            None => outer.access,
        },
        reset: number_of(e, "resetValue", whose)?.unwrap_or(outer.reset),
    })
}

/// The device described by `root`, the SVD file's `<device>`.
pub fn device(root: &Element) -> Result<Device, String> {
    if root.name != "device" {
        return Err(format!(
            "the root element is `<{}>`, not `<device>`",
            root.name
        ));
    }
    let name = root.text_of("name").unwrap_or_default();
    let start = Defaults {
        size: 32,
        access: Access::ReadWrite,
        reset: 0,
    };
    let outer = defaults(root, start, "the device")?;
    let mut peripherals = Vec::new();
    for p in root
        .child("peripherals")
        .iter()
        .flat_map(|ps| ps.all("peripheral"))
    {
        peripherals.push(peripheral(p, outer)?);
    }
    Ok(Device { name, peripherals })
}

fn peripheral(p: &Element, outer: Defaults) -> Result<Peripheral, String> {
    let name = p.text_of("name").ok_or("a peripheral has no `<name>`")?;
    let base =
        number_of(p, "baseAddress", &name)?.ok_or_else(|| format!("{name}: no `<baseAddress>`"))?;
    let inner = defaults(p, outer, &name)?;
    let mut registers = Vec::new();
    let mut skipped = Vec::new();
    for r in p
        .child("registers")
        .iter()
        .flat_map(|rs| rs.children.iter())
    {
        match r.name.as_str() {
            "register" => registers.extend(register(r, inner, &name, &mut skipped)?),
            "cluster" => skipped.push(format!("cluster {}", r.text_of("name").unwrap_or_default())),
            _ => {}
        }
    }
    let derived_from = p
        .attr("derivedFrom")
        .filter(|_| registers.is_empty())
        .map(str::to_string);
    Ok(Peripheral {
        description: p.text_of("description").unwrap_or_default(),
        name,
        base,
        derived_from,
        registers,
        skipped,
    })
}

/// The register `r`, or one for each index when it has `dim`.
fn register(
    r: &Element,
    outer: Defaults,
    peripheral: &str,
    skipped: &mut Vec<String>,
) -> Result<Vec<Register>, String> {
    let name = r
        .text_of("name")
        .ok_or_else(|| format!("{peripheral}: a register has no `<name>`"))?;
    let whose = format!("{peripheral}.{name}");
    let offset = number_of(r, "addressOffset", &whose)?
        .ok_or_else(|| format!("{whose}: no `<addressOffset>`"))?;
    let d = defaults(r, outer, &whose)?;
    if ![8, 16, 32, 64].contains(&d.size) {
        return Err(format!("{whose}: a register of {} bits", d.size));
    }
    let mut fields = Vec::new();
    for f in r.child("fields").iter().flat_map(|fs| fs.all("field")) {
        let field = field(f, d, &whose)?;
        if f.child("enumeratedValues").is_some() {
            skipped.push(format!("the enumerated values of {name}.{}", field.name));
        }
        fields.push(field);
    }
    let one = |name: String, offset: u64| Register {
        name,
        description: r.text_of("description").unwrap_or_default(),
        offset,
        size: d.size,
        access: d.access,
        reset: d.reset,
        fields: fields.clone(),
    };
    let Some(count) = number_of(r, "dim", &whose)? else {
        return Ok(vec![one(name, offset)]);
    };
    let step = number_of(r, "dimIncrement", &whose)?
        .ok_or_else(|| format!("{whose}: `<dim>` without `<dimIncrement>`"))?;
    let indexes = match r.text_of("dimIndex") {
        Some(list) => {
            dim_indexes(&list).ok_or_else(|| format!("{whose}: `<dimIndex>` `{list}`"))?
        }
        None => (0..count).map(|i| i.to_string()).collect(),
    };
    if indexes.len() as u64 != count {
        return Err(format!(
            "{whose}: `<dimIndex>` doesn't have {count} indexes"
        ));
    }
    let named = |i: &str| name.replace("[%s]", i).replace("%s", i);
    Ok(indexes
        .iter()
        .enumerate()
        .map(|(k, i)| one(named(i), offset + step * k as u64))
        .collect())
}

/// `0-3` or `A,B,C`.
fn dim_indexes(list: &str) -> Option<Vec<String>> {
    if let Some((a, b)) = list.split_once('-') {
        let (a, b): (u64, u64) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
        return Some((a..=b).map(|i| i.to_string()).collect());
    }
    Some(list.split(',').map(|s| s.trim().to_string()).collect())
}

fn field(f: &Element, reg: Defaults, register: &str) -> Result<Field, String> {
    let name = f
        .text_of("name")
        .ok_or_else(|| format!("{register}: a field has no `<name>`"))?;
    let whose = format!("{register}.{name}");
    let (lsb, width) = if let Some(range) = f.text_of("bitRange") {
        let inner = range.trim().trim_start_matches('[').trim_end_matches(']');
        let (msb, lsb) = inner
            .split_once(':')
            .and_then(|(m, l)| Some((m.trim().parse::<u32>().ok()?, l.trim().parse::<u32>().ok()?)))
            .ok_or_else(|| format!("{whose}: `<bitRange>` `{range}`"))?;
        (lsb, (msb + 1).saturating_sub(lsb))
    } else if let (Some(lsb), Some(msb)) =
        (number_of(f, "lsb", &whose)?, number_of(f, "msb", &whose)?)
    {
        (lsb as u32, (msb as u32 + 1).saturating_sub(lsb as u32))
    } else {
        let lsb = number_of(f, "bitOffset", &whose)?
            .ok_or_else(|| format!("{whose}: no `<bitOffset>`, `<lsb>` or `<bitRange>`"))?;
        (
            lsb as u32,
            number_of(f, "bitWidth", &whose)?.unwrap_or(1) as u32,
        )
    };
    if width == 0 || lsb + width > reg.size {
        return Err(format!(
            "{whose}: bits {lsb}–{} don't fit a register of {} bits",
            lsb + width.max(1) - 1,
            reg.size
        ));
    }
    let access = match f.text_of("access") {
        Some(a) => access(&a).map_err(|w| format!("{whose}: {w}"))?,
        None => reg.access,
    };
    Ok(Field {
        description: f.text_of("description").unwrap_or_default(),
        name,
        lsb,
        width,
        access,
    })
}
