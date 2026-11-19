// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `arena { … }` (spec 6.3): it waits for the tasks spawned in it, which
//! may use its values, before it ends, as native code does
//! (`compiler/kir/src/lower_arena.rs`).

use crate::interp::Interp;
use crate::value::Eval;
use kavach_ast::{Block, Span};
use std::sync::Arc;

impl<'a> Interp<'a> {
    pub(crate) fn arena(&mut self, b: &'a Block, span: Span) -> Eval<'a> {
        let before: Vec<_> = (self.frames.last().into_iter())
            .flat_map(|f| f.spawned.iter().map(|t| t.id))
            .collect();
        let result = self.block(b);
        let Some(sched) = self.tasks.as_ref().map(|t| Arc::clone(&t.sched)) else {
            return result;
        };
        let ours: Vec<_> = (self.frames.last().into_iter())
            .flat_map(|f| f.spawned.iter().map(|t| t.id))
            .filter(|id| !before.contains(id))
            .collect();
        for id in ours {
            if sched.is_done(id) {
                continue;
            }
            self.flush();
            if let Err(stop) = sched.join(self.task, id) {
                return Err(self.stopped(stop, span));
            }
        }
        result
    }
}
