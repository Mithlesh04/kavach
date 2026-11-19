// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! How C passes a record by value, on the machine the program is built
//! for (spec 10.7): the rules of Windows x64 and of x86-64 System V
//! (Linux, macOS on Intel). The lowering copies the record into C memory
//! and passes it as these rules say; the backends follow `descriptor`.

use crate::ir::Type;

/// The calling rules of this machine's C, or `None` where records can't
/// go by value yet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rules {
    Win64,
    SysV64,
}

impl Rules {
    pub fn here() -> Option<Rules> {
        if cfg!(all(windows, target_arch = "x86_64")) {
            Some(Rules::Win64)
        } else if cfg!(all(unix, target_arch = "x86_64")) {
            Some(Rules::SysV64)
        } else {
            None
        }
    }
}

/// An eightbyte of a record, in a whole-number or a floating register.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Part {
    Int,
    Float,
}

/// How a record goes to C.
#[derive(Clone, Debug, PartialEq)]
pub enum Passed {
    /// Its eightbytes, each in a register.
    Parts(Vec<Part>),
    /// The address of a copy (Windows, for sizes other than 1, 2, 4, 8).
    Copy,
    /// A copy on the stack (System V, for more than 16 bytes).
    Stack,
}

/// How a record comes back from C.
#[derive(Clone, Debug, PartialEq)]
pub enum Returned {
    /// In one or two registers.
    Parts(Vec<Part>),
    /// Written where a hidden first argument points.
    Hidden,
}

/// A record laid out as C has it: its size and each field's type and
/// offset.
pub struct Layout<'a> {
    pub size: i128,
    pub fields: &'a [(Type, i128)],
}

impl Layout<'_> {
    /// System V's class of each eightbyte: a float register only when
    /// every field in it is a float.
    fn parts(&self) -> Vec<Part> {
        let count = ((self.size + 7) / 8) as usize;
        (0..count)
            .map(|k| {
                let (lo, hi) = (k as i128 * 8, k as i128 * 8 + 8);
                let all_floats = (self.fields.iter())
                    .filter(|(_, at)| *at >= lo && *at < hi)
                    .all(|(t, _)| matches!(t, Type::F32 | Type::F64));
                if all_floats { Part::Float } else { Part::Int }
            })
            .collect()
    }
}

/// Registers System V has for arguments: whole numbers, floats.
const SYSV_REGISTERS: (usize, usize) = (6, 8);

/// How each argument goes, given `args` (each a record's layout or a
/// plain value's type) and whether a hidden result pointer comes first.
pub fn passed(rules: Rules, args: &[Result<&Layout, Type>], hidden: bool) -> Vec<Option<Passed>> {
    let mut used = (usize::from(hidden), 0);
    args.iter()
        .map(|a| match (rules, a) {
            (_, Err(t)) => {
                match t {
                    Type::F32 | Type::F64 => used.1 += 1,
                    _ => used.0 += 1,
                }
                None
            }
            (Rules::Win64, Ok(l)) => Some(match l.size {
                1 | 2 | 4 | 8 => Passed::Parts(vec![Part::Int]),
                _ => Passed::Copy,
            }),
            (Rules::SysV64, Ok(l)) if l.size > 16 => Some(Passed::Stack),
            (Rules::SysV64, Ok(l)) => {
                let parts = l.parts();
                let ints = parts.iter().filter(|p| **p == Part::Int).count();
                let floats = parts.len() - ints;
                // A record goes whole in registers or whole on the stack.
                if used.0 + ints > SYSV_REGISTERS.0 || used.1 + floats > SYSV_REGISTERS.1 {
                    Some(Passed::Stack)
                } else {
                    used = (used.0 + ints, used.1 + floats);
                    Some(Passed::Parts(parts))
                }
            }
        })
        .collect()
}

/// How a record result comes back.
pub fn returned(rules: Rules, l: &Layout) -> Returned {
    match rules {
        Rules::Win64 => match l.size {
            1 | 2 | 4 | 8 => Returned::Parts(vec![Part::Int]),
            _ => Returned::Hidden,
        },
        Rules::SysV64 if l.size > 16 => Returned::Hidden,
        Rules::SysV64 => Returned::Parts(l.parts()),
    }
}

fn letter(p: Part) -> char {
    match p {
        Part::Int => 'i',
        Part::Float => 'f',
    }
}

/// The text the backends read for a call (`RtOp::CallC`): the result,
/// `|`, then one word per operand after the target, joined by `,`.
/// Result: `v` (nothing), `n` (a plain value) or its parts (`i`, `f`,
/// `if`, …) written to the `m` operand. Operands: `n` a plain argument,
/// `i` or `f` one part of a record, `s<size>` the address of a copy that
/// goes on the stack, `h<size>` the hidden result pointer, `m` where the
/// result's parts go (not an argument).
pub fn descriptor(result: &str, operands: &[String]) -> String {
    format!("{result}|{}", operands.join(","))
}

/// The result word for `returned`.
pub fn result_word(r: &Returned) -> String {
    match r {
        Returned::Parts(parts) => parts.iter().map(|p| letter(*p)).collect(),
        Returned::Hidden => "v".into(),
    }
}

/// The operand words for one argument passed as `p`.
pub fn operand_words(p: &Passed, size: i128) -> Vec<String> {
    match p {
        Passed::Parts(parts) => parts.iter().map(|p| letter(*p).to_string()).collect(),
        Passed::Copy => vec!["n".into()],
        Passed::Stack => vec![format!("s{size}")],
    }
}

#[cfg(test)]
mod tests;
