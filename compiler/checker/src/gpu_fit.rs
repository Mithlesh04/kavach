//! Whether a plain `fn` could be a `gpu fn` (spec 15.1), for the message
//! when a `gpu fn` calls one: mark it `gpu fn`, or what stops it.

use crate::checker::Checker;
use crate::gpu::gpu_item;
use crate::gpu_walk::Walk;
use crate::ty::Ty;
use kavach_ast::{File, FnDecl, ItemKind};
use kavach_resolver::DefId;
use std::collections::HashMap;

/// For each plain function: `None` when it fits the GPU's rules, or what
/// doesn't (`it takes a `Float``).
pub(crate) type Fits = HashMap<DefId, Option<String>>;

impl Checker<'_> {
    /// Whether each plain function of `file` fits the GPU's rules.
    pub(crate) fn gpu_fits(&self, file: &File, types: &HashMap<(u32, u32), Ty>) -> Fits {
        let none = Fits::new();
        let mut fits = Fits::new();
        for item in &file.items {
            let ItemKind::Fn(f) = &item.kind else {
                continue;
            };
            if let (false, Some(&def)) = (f.is_gpu, self.decl.get(&f.name.span.start)) {
                fits.insert(def, self.gpu_misfit(f, def, types, &none));
            }
        }
        fits
    }

    /// What keeps plain function `f` off the GPU, or `None` when nothing
    /// does. Its calls to other plain functions count against it.
    fn gpu_misfit(
        &self,
        f: &FnDecl,
        def: DefId,
        types: &HashMap<(u32, u32), Ty>,
        none: &Fits,
    ) -> Option<String> {
        if !f.generics.is_empty() {
            return Some("it is generic".into());
        }
        if f.errors.is_some() || !f.uses.is_empty() {
            return Some("it can fail or uses capabilities".into());
        }
        if let Some(p) = f.params.iter().find(|p| p.is_inout) {
            return Some(format!("its parameter `{}` is `inout`", p.name.name));
        }
        let sig = self.fns.get(&def)?;
        let typed = f.params.iter().filter(|p| p.name.name != "self");
        for (p, info) in typed.zip(&sig.params) {
            let t = self.inf.resolve(&info.ty);
            if !gpu_item(self, &t) {
                return Some(format!("it takes `{}`, a `{}`", p.name.name, self.show(&t)));
            }
        }
        let ret = self.inf.resolve(&sig.ret);
        if !gpu_item(self, &ret) {
            return Some(format!("it gives a `{}`", self.show(&ret)));
        }
        let mut walk = Walk::new(types, none);
        if let Some(body) = &f.body {
            walk.block(self, body);
        }
        let (_, first) = walk.errors.first()?;
        let what = first
            .trim_start_matches("a `gpu fn` can't use ")
            .trim_end_matches(" (spec 15.1)");
        Some(format!("it uses {what}"))
    }
}
