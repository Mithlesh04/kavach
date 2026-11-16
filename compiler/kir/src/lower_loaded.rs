//! The functions of `extern c library Name { … }` (spec 10.7), which the
//! parser made methods of the type `Name` (`kavach_parser::loaded`):
//! `open` loads the library and finds each function in it, keeping
//! their addresses in the record; each other method calls C through the
//! address it keeps.

use crate::ir::{self, Block, ExprKind as K, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, CLoaded, ItemKind, TypeBody};

fn at(kind: K, ty: Type, line: u32) -> ir::Expr {
    ir::Expr { kind, ty, line }
}

impl<'a> Lowerer<'a> {
    /// The body of `f`, a function of a library loaded while running, of
    /// type `owner`; `params` are its locals, `self` first for a C
    /// function.
    pub(crate) fn loaded_body(
        &mut self,
        f: &'a ast::FnDecl,
        loaded: CLoaded,
        owner: kavach_resolver::DefId,
        params: &[ir::Local],
        ret: Type,
    ) -> Lowered<Block> {
        let line = self.line(f.name.span);
        let CLoaded::Function(number) = loaded else {
            return self.open_body(owner, params[0], ret, line);
        };
        let addresses = at(
            K::GetField(Box::new(self.local_expr(params[0], line)), 0),
            self.list_type(Type::I64),
            line,
        );
        let index = at(K::Int(number as i128), Type::I64, line);
        let address = at(
            K::Index(Box::new(addresses), Box::new(index)),
            Type::I64,
            line,
        );
        let mut args = Vec::new();
        for (p, local) in f.params.iter().skip(1).zip(&params[1..]) {
            if p.ty
                .as_ref()
                .is_some_and(|t| matches!(t.kind, ast::TypeKind::Fn { .. }))
            {
                return Err(
                    self.unsupported("a callback to a library loaded while running", p.name.span)
                );
            }
            args.push(self.local_expr(*local, line));
        }
        let call = match self.c_record_call(address.clone(), args, ret, line, f.name.span)? {
            Ok(call) => call,
            Err(args) => {
                let values = std::iter::once(address).chain(args).collect();
                self.c_copied_call(values, ret, line, |args| K::Rt(RtOp::CallAddress, args))
            }
        };
        Ok(if ret == Type::Unit {
            Block {
                stmts: vec![Stmt::Expr(call)],
                value: None,
            }
        } else {
            Block {
                stmts: Vec::new(),
                value: Some(Box::new(call)),
            }
        })
    }

    /// `Name.open(path)`: loads the library, then finds each function of
    /// the block in it, failing with a `LibraryError` at the first that
    /// isn't there.
    fn open_body(
        &mut self,
        owner: kavach_resolver::DefId,
        path: ir::Local,
        ret: Type,
        line: u32,
    ) -> Lowered<Block> {
        let names = self.loaded_names(owner);
        let mut stmts = Vec::new();
        let error_ty = self.error_type();
        let tried = |l: &mut Self, call: ir::Expr, stmts: &mut Vec<Stmt>| {
            let (value, error) = (l.new_local(Type::I64), l.new_local(error_ty));
            stmts.push(Stmt::LetResult { value, error, call });
            let fail = at(
                K::Fail(Box::new(l.local_expr(error, line))),
                Type::Unit,
                line,
            );
            let check = K::If(
                Box::new(l.failed(error, line)),
                Block {
                    stmts: vec![Stmt::Expr(fail)],
                    value: None,
                },
                Block::default(),
            );
            stmts.push(Stmt::Expr(at(check, Type::Unit, line)));
            value
        };
        let text = self.str_type();
        let open = K::Rt(RtOp::CLibraryOpen, vec![self.local_expr(path, line)]);
        let handle = tried(self, at(open, Type::I64, line), &mut stmts);
        let mut found = Vec::new();
        for name in names {
            let name = at(K::Str(name), text, line);
            let find = K::Rt(
                RtOp::CLibraryFind,
                vec![self.local_expr(handle, line), name],
            );
            let address = tried(self, at(find, Type::I64, line), &mut stmts);
            found.push(self.local_expr(address, line));
        }
        let list = at(K::List(found), self.list_type(Type::I64), line);
        let record = K::Record {
            variant: 0,
            fields: vec![list],
        };
        Ok(Block {
            stmts,
            value: Some(Box::new(at(record, ret, line))),
        })
    }

    /// The C functions of library type `owner`, in the block's order.
    fn loaded_names(&self, owner: kavach_resolver::DefId) -> Vec<String> {
        let Some(span) = self.c.resolution.defs[owner].span else {
            return Vec::new();
        };
        let found = self.c.file.items.iter().find_map(|item| match &item.kind {
            ItemKind::Type(t) if t.name.span == span => match &t.body {
                TypeBody::Record(r) => Some(&r.methods),
                _ => None,
            },
            _ => None,
        });
        let mut names: Vec<(usize, String)> = found
            .into_iter()
            .flatten()
            .filter_map(|m| match &m.kind {
                ItemKind::Fn(f) => match f.c_loaded {
                    Some(CLoaded::Function(n)) => Some((n, f.name.name.clone())),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        names.sort();
        names.into_iter().map(|(_, n)| n).collect()
    }
}
