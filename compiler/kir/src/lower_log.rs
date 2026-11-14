//! Logging in native code (spec 14.12): each `log_write` of `std.log`,
//! with its event's lowest level worked out when compiling, and the
//! `[log]` settings handed to the runtime at the start of `main`;
//! OpenTelemetry export when `[log.otlp]` is set.

use crate::ir::{self, BinOp, Block, ExprKind as K, FuncId, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;
use kavach_driver::Compiled;
use kavach_runtime_core::json::{self, Tree};

/// When the program logs, or makes spans or metrics (spec 14.12), `main`
/// first hands the runtime the `[log]` settings from `kavach.toml`, read
/// now (as for agents, 8.5), and the collector of `[log.otlp]`.
pub(crate) fn configure_log(
    c: &Compiled,
    functions: &mut [ir::Function],
    entry: FuncId,
    text: Type,
) {
    let logs = functions.iter().any(|f| {
        crate::walk::any_in_block(&f.body, &mut |e| {
            matches!(
                e.kind,
                K::Rt(RtOp::LogWrite | RtOp::ScopeSpan | RtOp::OtlpMetric, _)
            )
        })
    });
    let Some(main) = functions.get_mut(entry as usize).filter(|_| logs) else {
        return;
    };
    let s = kavach_driver::Config::for_file(std::path::Path::new(&c.path))
        .unwrap_or_default()
        .log_settings();
    let file = s
        .file
        .map_or(Tree::Null, |f| Tree::Text(f.to_string_lossy().into_owned()));
    let settings = Tree::Object(vec![
        ("level".into(), Tree::Int(s.level)),
        ("console".into(), Tree::Bool(s.console)),
        ("file".into(), file),
        ("max_bytes".into(), Tree::Int(s.max_bytes as i64)),
        ("keep".into(), Tree::Int(i64::from(s.keep))),
    ]);
    let at = |kind, ty| ir::Expr {
        kind,
        ty,
        line: main.line,
    };
    let description = at(K::Str(json::write(&settings, 0)), text);
    let configure = at(K::Rt(RtOp::LogConfigure, vec![description]), Type::Unit);
    main.body.stmts.insert(0, ir::Stmt::Expr(configure));
    if let Some((endpoint, service)) = s.otlp {
        let collector = at(K::Str(format!("{endpoint}\n{service}")), text);
        let configure = at(K::Rt(RtOp::OtlpConfigure, vec![collector]), Type::Unit);
        main.body.stmts.insert(0, ir::Stmt::Expr(configure));
    }
}

impl<'a> Lowerer<'a> {
    /// `log_write(level, line, event)` in `std.log`: the line, written when
    /// `level` is at least the event's lowest level, which is known now.
    pub(crate) fn log_write_call(
        &mut self,
        level: &'a ast::Arg,
        text: &'a ast::Arg,
        event: &'a ast::Arg,
        line: u32,
    ) -> Lowered<ir::Expr> {
        // The event's lowest level (spec 14.12), from its type and
        // module: known now, as `kavach.toml` is read now.
        let event_ty = self.expr(&event.value)?.ty;
        let lowest = self.log_level_for(event_ty);
        let lowest = ir::Expr {
            kind: K::Int(i128::from(lowest)),
            ty: Type::I64,
            line,
        };
        let level = self.expr(&level.value)?;
        let text = self.expr(&text.value)?;
        if let Some(export) =
            self.log_export(level.clone(), text.clone(), event_ty, lowest.clone())?
        {
            return Ok(export);
        }
        let operands = vec![level, text, lowest];
        Ok(ir::Expr {
            kind: K::Rt(RtOp::LogWrite, operands),
            ty: Type::Unit,
            line,
        })
    }

    /// The lowest level written for events of type `ty` (spec 14.12): its
    /// own in `[log.levels]`, else its module's, else `level`.
    pub(crate) fn log_level_for(&self, ty: Type) -> i64 {
        let settings = kavach_driver::Config::for_file(std::path::Path::new(&self.c.path))
            .unwrap_or_default()
            .log_settings();
        let Some(def) = self.shapes.def_of(ty) else {
            return settings.level;
        };
        let res = &self.c.resolution;
        let start = res.defs[def].span.map_or(0, |s| s.start);
        let module = &res.modules[res.module_at(start)].name;
        settings.level_for(module, &res.defs[def].name)
    }

    /// With `[log.otlp]` set: the line written, then sent to the collector
    /// when `level` is at least `lowest` (spec 14.12).
    pub(crate) fn log_export(
        &mut self,
        level: ir::Expr,
        text: ir::Expr,
        event_ty: Type,
        lowest: ir::Expr,
    ) -> Lowered<Option<ir::Expr>> {
        let settings = kavach_driver::Config::for_file(std::path::Path::new(&self.c.path))
            .unwrap_or_default()
            .log_settings();
        if settings.otlp.is_none() {
            return Ok(None);
        }
        let line = level.line;
        let name = self
            .shapes
            .def_of(event_ty)
            .map_or(String::new(), |d| self.c.resolution.defs[d].name.clone());
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let (l, t) = (self.new_local(Type::I64), self.new_local(text.ty));
        let str_ty = self.str_type();
        let write = vec![
            at(K::Local(l), Type::I64),
            at(K::Local(t), text.ty),
            lowest.clone(),
        ];
        let export = vec![
            at(K::Local(l), Type::I64),
            at(K::Local(t), text.ty),
            at(K::Str(name), str_ty),
        ];
        let wanted = at(
            K::Binary(
                BinOp::Ge,
                Box::new(at(K::Local(l), Type::I64)),
                Box::new(lowest),
            ),
            Type::Bool,
        );
        let send = at(K::Rt(RtOp::OtlpExport, export), Type::Unit);
        let stmts = vec![
            Stmt::Let(l, level),
            Stmt::Let(t, text),
            Stmt::Expr(at(K::Rt(RtOp::LogWrite, write), Type::Unit)),
            Stmt::Expr(at(
                K::If(
                    Box::new(wanted),
                    crate::lower_expr::wrap(send, false),
                    Block::default(),
                ),
                Type::Unit,
            )),
        ];
        Ok(Some(at(K::Block(Block { stmts, value: None }), Type::Unit)))
    }
}
