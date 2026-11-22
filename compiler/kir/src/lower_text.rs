// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Text methods (spec 14.2) that each lower to one op: those that came
//! from `std.text` (`capitalize`, `words`, padding, …) and those added
//! with them. `pad_left` and `pad_right` fill with a space when `fill:`
//! isn't given. Also a `Char`'s kind checks.

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::Lowerer;
use crate::rt_op::RtOp;
use kavach_runtime_core::text_methods::CHAR_CHECKS;

impl Lowerer<'_> {
    /// `s.name(…)` on text, with `values` the text then the arguments;
    /// `None` when `name` isn't one of these methods.
    pub(crate) fn text_method(
        &mut self,
        name: &str,
        mut values: Vec<ir::Expr>,
        ty: Type,
        line: u32,
    ) -> Option<ir::Expr> {
        let op = match name {
            "trim_start" => RtOp::TextTrimStart,
            "trim_end" => RtOp::TextTrimEnd,
            "capitalize" => RtOp::TextCapitalize,
            "title" => RtOp::TextTitle,
            "is_blank" => RtOp::TextIsBlank,
            "words" => RtOp::TextWords,
            "last_index_of" => RtOp::TextLastIndexOf,
            "count" => RtOp::TextCount,
            "remove_prefix" => RtOp::TextRemovePrefix,
            "remove_suffix" => RtOp::TextRemoveSuffix,
            "pad_left" => RtOp::TextPadLeft,
            "pad_right" => RtOp::TextPadRight,
            _ => return None,
        };
        if matches!(op, RtOp::TextPadLeft | RtOp::TextPadRight) && values.len() == 2 {
            values.push(self.at(K::Char(' '), Type::Char, line));
        }
        Some(self.at(K::Rt(op, values), ty, line))
    }

    /// `c.is_digit()` and the other kind checks of a `Char` (spec 14.2,
    /// *Characters*), with `values` the character; `None` when `name`
    /// isn't one of them.
    pub(crate) fn char_check(
        &mut self,
        name: &str,
        mut values: Vec<ir::Expr>,
        ty: Type,
        line: u32,
    ) -> Option<ir::Expr> {
        let check = CHAR_CHECKS.iter().position(|n| *n == name)?;
        values.push(self.at(K::Int(check as i128), Type::I64, line));
        Some(self.at(K::Rt(RtOp::TextCharIs, values), ty, line))
    }
}
