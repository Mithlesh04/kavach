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
