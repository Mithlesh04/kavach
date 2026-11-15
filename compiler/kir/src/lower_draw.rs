//! `gpu.pipeline(v, f)` and a frame's draws (spec 15.3, decision 0106).
//! A pipeline is its number: `DrawPipeline` takes the two gpu fns, as
//! closures, and the WGSL the backend writes for them (`attach` in
//! `compiler/backend/wgsl`). A draw takes the frame, the pipeline, and its
//! vertices, instances and shared value as one tuple.

use crate::ir::{self, ExprKind as K, Shape, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    pub(crate) fn draw_call(
        &mut self,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let mut values = Vec::new();
        for a in args {
            values.push(self.expr(&a.value)?);
        }
        if name == "pipeline" {
            // Until the backend writes the module: the two fns' definitions,
            // by which `kavach run` gives the interpreter the module.
            let def = |a: &ast::Arg| match &a.value.kind {
                ast::ExprKind::Name(id) => self.c.resolution.uses.get(&id.span.start).copied(),
                _ => None,
            };
            let key = match args {
                [v, f] => format!("{} {}", def(v).unwrap_or(0), def(f).unwrap_or(0)),
                _ => String::new(),
            };
            let text = self.str_type();
            values.push(self.at(K::Str(key), text, line));
            return Ok(self.at(K::Rt(RtOp::DrawPipeline, values), Type::I64, line));
        }
        if values.len() != 5 {
            return Err(self.unsupported("this draw", span));
        }
        let rest = values.split_off(2);
        let tuple = self.shapes_intern(Shape::Tuple(rest.iter().map(|v| v.ty).collect()));
        values.push(self.at(K::Tuple(rest), tuple, line));
        Ok(self.at(K::Rt(RtOp::DrawDraw, values), ty, line))
    }
}

impl<'a> Lowerer<'a> {
    /// `frame.draw(p, vertices, instances:, shared:)` (spec 15.3), lowered
    /// here rather than through `Frame.draw` in `std/gpu.kav`, whose
    /// defaults have the pipeline's types: an instance or shared value not
    /// given is an empty list or `none` of them. `None` for any other call.
    pub(crate) fn frame_draw(
        &mut self,
        base: &'a ast::Expr,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
    ) -> Lowered<Option<ir::Expr>> {
        use kavach_checker::Ty;
        let typed = |e: &ast::Expr| self.c.expr_types.get(&(e.span.start, e.span.end)).cloned();
        match typed(base) {
            Some(Ty::Adt(d, _)) if self.c.resolution.defs[d].name == "Frame" => {}
            _ => return Ok(None),
        }
        let positional: Vec<&ast::Arg> = args.iter().filter(|a| a.name.is_none()).collect();
        let named = |n: &str| {
            args.iter()
                .find(|a| a.name.as_ref().is_some_and(|x| x.name == n))
        };
        let ([p, vertices], Some(Ty::Builtin("Pipeline", inputs))) =
            (&positional[..], typed(&positional[0].value))
        else {
            return Ok(None);
        };
        let frame = self.expr(base)?;
        let frame = self.at(K::GetField(Box::new(frame), 0), Type::I64, line);
        let pipeline = self.expr(&p.value)?;
        let vertices = self.expr(&vertices.value)?;
        let span = p.value.span;
        let instances = match named("instances") {
            Some(a) => self.expr(&a.value)?,
            None => {
                let t = self.kir_type(&Ty::List(Box::new(inputs[1].clone())), span)?;
                self.at(K::List(Vec::new()), t, line)
            }
        };
        let shared_ty = self.kir_type(&Ty::Optional(Box::new(inputs[2].clone())), span)?;
        let shared = match named("shared") {
            Some(a) => {
                let v = self.expr(&a.value)?;
                self.coerce(v, shared_ty)?
            }
            None => self.at(K::None, shared_ty, line),
        };
        let rest = vec![vertices, instances, shared];
        let tuple = self.shapes_intern(Shape::Tuple(rest.iter().map(|v| v.ty).collect()));
        let parts = self.at(K::Tuple(rest), tuple, line);
        Ok(Some(self.at(
            K::Rt(RtOp::DrawDraw, vec![frame, pipeline, parts]),
            ty,
            line,
        )))
    }
}
