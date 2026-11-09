//! Approval requests answered in code in native programs (spec 14.16,
//! decision 0072), from `kavach_runtime_core::agents`, as the interpreter
//! does: found and answered in the project's agents folder, which the
//! compiler passes first.

use crate::heap::{alloc_slots, new_str, release, shape, str_of};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::agents::{Answer, AnswerError, answer_request, request_fields};
use std::path::PathBuf;

/// `a` is the agents folder (owned, empty outside a project); for an
/// answer, `b` is `[id, "yes" or "no", by, reason]` (owned).
pub(crate) fn run(op: RtOp, a: i64, b: i64, code: i64) -> i64 {
    let folder = Some(str_of(a)).filter(|f| !f.is_empty()).map(PathBuf::from);
    release(a);
    if op == RtOp::AgentApprovals {
        let item = match shape(type_of(code)) {
            Shape::List(t) => *t,
            _ => Type::I64,
        };
        let texts = folder.as_deref().map(request_fields).unwrap_or_default();
        let items = texts.into_iter().map(|t| new_str(t, item) as u64).collect();
        return alloc_slots(type_of(code), 0, items);
    }
    let fields: Vec<String> = (&crate::shapes::items_of(b))
        .into_iter()
        .map(|s| str_of(*s as i64))
        .collect();
    release(b);
    let field = |i: usize| fields.get(i).cloned().unwrap_or_default();
    let answer = Answer {
        approved: field(1) == "yes",
        by: field(2),
        reason: Some(field(3)).filter(|r| !r.is_empty()),
    };
    let done = match &folder {
        Some(f) => answer_request(f, &field(0), &answer),
        None => Err(AnswerError::NotWaiting),
    };
    let Err(e) = done else {
        return 0;
    };
    let Some(ty) = crate::shapes::find_union("ApprovalError", "NotWaiting") else {
        return 0;
    };
    let text = |s: &str| crate::shapes::new_text(s, crate::heap::first_of(&Shape::Str)) as u64;
    let error = match e {
        AnswerError::NotWaiting => {
            crate::rt_json::new_variant(ty, "NotWaiting", &[text(&field(0))])
        }
        AnswerError::Failed(why) => {
            crate::rt_json::new_variant(ty, "Failed", &[text(&field(0)), text(&why)])
        }
    };
    set_error(error);
    0
}
