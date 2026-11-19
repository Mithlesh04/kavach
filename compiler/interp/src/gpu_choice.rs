// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Choosing a GPU in the interpreter (spec 15.6). The interpreter runs
//! every `map` on the CPU, which gives the same values, so a choice
//! changes nothing here but its checks: the device must be there, and
//! every device the code names by number must be there at the start.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{Arg, Block, Expr, GpuNumberAt, ItemKind, Span};

impl<'a> Interp<'a> {
    /// At the start: every GPU the code names by number (`gpu(n) fn`, `on
    /// gpu(n)`), and the one `[run] gpu` (`file`) or `KAVACH_GPU` names,
    /// must be there. `Err` says what is missing and what there is.
    pub(crate) fn gpu_start(&self, file: Option<&str>) -> Result<(), String> {
        let highest = kavach_ast::highest_gpu(&self.c.file);
        let device_setting = file.filter(|t| !matches!(*t, "cpu" | "auto"));
        if highest.is_none() && device_setting.is_none() {
            let has_gpu_fns = (self.c.file.items.iter())
                .any(|it| matches!(&it.kind, ItemKind::Fn(f) if f.is_gpu));
            return match has_gpu_fns {
                true => kavach_runtime_gpu::check_launch(),
                false => Ok(()),
            };
        }
        let (n, place) = match highest {
            Some((n, GpuNumberAt::Fn(name))) => (Some(n), format!("`gpu({n}) fn {name}`")),
            Some((n, GpuNumberAt::On(span))) => {
                let (_, line) = self.c.shown_location(span.start);
                (Some(n), format!("`on gpu({n})` on line {line}"))
            }
            None => (None, String::new()),
        };
        let n = n.and_then(|n| usize::try_from(n).ok());
        kavach_runtime_gpu::start(n, &place, device_setting)
    }

    /// Device `n`, chosen while running: a panic when the machine has no
    /// such GPU.
    pub(crate) fn check_device(&self, n: i128, span: Span) -> Result<(), Flow<'a>> {
        let n = n.clamp(i64::MIN as i128, i64::MAX as i128) as i64;
        match kavach_runtime_gpu::missing(n) {
            Some(why) => Err(self.panic(why, span)),
            None => Ok(()),
        }
    }

    /// `on gpu(n) x`: `x`, once GPU `n` is found there.
    pub(crate) fn on_gpu(&mut self, n: &'a Expr, value: &'a Expr) -> Eval<'a> {
        if let Value::Int(k, _) = self.eval(n)? {
            self.check_device(k, n.span)?;
        }
        self.eval(value)
    }

    /// `gpu.on(device) { … }`: the block, once the device is found there.
    pub(crate) fn gpu_on(&mut self, args: &'a [Arg], body: &'a Block) -> Eval<'a> {
        if let Some(a) = args.first() {
            let device = self.eval(&a.value)?;
            self.check_device(device_index(&device), a.value.span)?;
        }
        self.block(body)
    }

    /// The arguments of `xs.map(f, on: device)` without the device, once
    /// it is found there.
    pub(crate) fn map_on(
        &self,
        mut args: Vec<(Option<&'a str>, Value<'a>)>,
        span: Span,
    ) -> Result<Vec<(Option<&'a str>, Value<'a>)>, Flow<'a>> {
        if let Some((Some("on"), device)) = args.last() {
            self.check_device(device_index(device), span)?;
            args.pop();
        }
        Ok(args)
    }
}

/// The number of a `GpuDevice`: its `index` field.
fn device_index(device: &Value<'_>) -> i128 {
    let Value::Record { fields, .. } = device else {
        return -1;
    };
    match fields
        .iter()
        .find(|f| &*f.name == "index")
        .map(|f| &f.value)
    {
        Some(Value::Int(n, _)) => *n,
        _ => -1,
    }
}
