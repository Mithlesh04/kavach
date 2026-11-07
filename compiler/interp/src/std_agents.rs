//! Approval requests answered in code (spec 14.16, decision 0072): found
//! and answered in the project's agents folder, as `kavach approvals`,
//! `approve` and `deny` do; `std/agents.kav` gives them their types.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_runtime_core::agents::{Answer, AnswerError, answer_request, request_fields};
use std::path::PathBuf;

impl<'a> Interp<'a> {
    pub(crate) fn approvals_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        // Under `kavach test` and in a dry run agents keep nothing.
        let folder = (self.c)
            .agents_folder(self.testing || self.dry_run)
            .map(PathBuf::from);
        if name == "request_fields" {
            let texts = folder.as_deref().map(request_fields).unwrap_or_default();
            let items = texts.iter().map(|t| self.str_value(t)).collect();
            return Ok(Value::List(std::rc::Rc::new(items)));
        }
        // `[id, "yes" or "no", by, reason]`, from `agents.answer`.
        let fields: Vec<String> = match args.first() {
            Some(Value::List(items)) => (items.iter())
                .map(|v| match v {
                    Value::Str(s) => s.to_string(),
                    _ => String::new(),
                })
                .collect(),
            _ => Vec::new(),
        };
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
        done.map(|()| Value::Unit).map_err(|e| {
            let id = ("id", self.str_value(&field(0)));
            let error = match e {
                AnswerError::NotWaiting => {
                    self.builtin_variant("ApprovalError", "NotWaiting", vec![id])
                }
                AnswerError::Failed(why) => {
                    let reason = ("reason", self.str_value(&why));
                    self.builtin_variant("ApprovalError", "Failed", vec![id, reason])
                }
            };
            Flow::Fail(error)
        })
    }
}
