//! C functions (`extern c`, spec 10.7) in native code: a function with no
//! body and its library recorded. The backends declare it as the C symbol
//! of its name, and the linker finds it in the library.

use crate::ir::{self, Block, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    pub(crate) fn c_function(
        &mut self,
        f: &'a ast::FnDecl,
        library: &str,
    ) -> Lowered<ir::Function> {
        let ret = match &f.ret {
            Some(t) => self.written_type(t)?,
            None => Type::Unit,
        };
        self.start_function(ret);
        let mut params = Vec::new();
        for p in &f.params {
            let Some(t) = &p.ty else {
                return Err(self.unsupported("`self` in a C function", p.name.span));
            };
            // Text goes to C as the address of a copy (`lower_ptr.rs`),
            // and a callback as the address of the function.
            let ty = match self.written_type(t)? {
                _ if matches!(t.kind, ast::TypeKind::Fn { .. }) => Type::I64,
                ty if ty == self.str_type() => Type::I64,
                ty => ty,
            };
            params.push(self.new_local(ty));
        }
        Ok(ir::Function {
            name: f.name.name.clone(),
            params,
            ret,
            locals: self.locals.clone(),
            body: Block::default(),
            line: self.line(f.name.span),
            fails: false,
            external: Some(library.to_string()),
        })
    }
}
