//! Functions: the signature, its parameters and its clauses, in their
//! fixed order (spec 3.5).

use crate::parser::Parser;
use kavach_ast::*;
use kavach_lexer::{Keyword, TokenKind};

/// Signature clauses in the only allowed order (spec 3.5).
const CLAUSES: [&str; 6] = ["->", "!", "uses", "require", "ensure", "example"];

impl Parser<'_> {
    pub(crate) fn fn_decl(&mut self, body_required: bool) -> FnDecl {
        self.bump();
        let name = self.ident("a function name");
        self.fn_rest(name, body_required)
    }

    /// Everything after the name: generics, parameters, clauses and body.
    pub(crate) fn fn_rest(&mut self, name: Ident, body_required: bool) -> FnDecl {
        let mut f = FnDecl {
            generics: self.generic_params(),
            params: self.params(),
            name,
            ret: None,
            ret_secret: false,
            is_audited: false,
            errors: None,
            uses: Vec::new(),
            fs_scope: None,
            process_scope: None,
            requires: Vec::new(),
            ensures: Vec::new(),
            examples: Vec::new(),
            body: None,
            c_library: None,
            is_gpu: false,
            deprecated: None,
        };
        self.clauses(&mut f);
        if *self.peek_past_newlines() == TokenKind::LBrace {
            self.skip_newlines();
            f.body = Some(self.block());
        } else if body_required {
            self.error_here(format!(
                "expected `{{` to start the body of `{}`",
                f.name.name
            ));
        }
        f
    }

    fn params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        if !self.expect(&TokenKind::LParen, "`(` and the parameters") {
            return params;
        }
        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
            if self.at_kw(Keyword::SelfValue) {
                let span = self.span();
                self.bump();
                let name = Ident {
                    name: "self".to_string(),
                    span,
                };
                params.push(Param {
                    name,
                    ty: None,
                    default: None,
                    is_secret: false,
                    is_inout: false,
                });
            } else {
                // `inout` is a keyword only here, before a name (spec 3.9).
                let is_inout = self.at_word("inout") && *self.nth(1) == TokenKind::Ident;
                if is_inout {
                    self.bump();
                }
                let is_secret = self.eat_kw(Keyword::Secret);
                let name = self.ident("a parameter name");
                let ty = if self.eat(&TokenKind::Colon) {
                    Some(self.type_expr())
                } else {
                    self.error(
                        name.span,
                        format!(
                            "write the type of `{}`: parameters always have a type",
                            name.name
                        ),
                    );
                    None
                };
                let default = self.eat(&TokenKind::Eq).then(|| self.expr());
                params.push(Param {
                    name,
                    ty,
                    default,
                    is_secret,
                    is_inout,
                });
            }
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "`)` to close the parameters");
        params
    }

    fn clauses(&mut self, f: &mut FnDecl) {
        let mut stage = 0;
        loop {
            let rank = match self.peek_past_newlines() {
                TokenKind::Arrow => 0,
                TokenKind::Bang => 1,
                TokenKind::Keyword(Keyword::Uses) => 2,
                TokenKind::Keyword(Keyword::Require) => 3,
                TokenKind::Keyword(Keyword::Ensure) => 4,
                TokenKind::Keyword(Keyword::Example) => 5,
                _ => return,
            };
            self.skip_newlines();
            if rank < stage {
                self.error_here(format!(
                    "`{}` must come before `{}`",
                    CLAUSES[rank], CLAUSES[stage]
                ));
            } else if rank == stage
                && rank <= 2
                && (f.ret.is_some() || f.errors.is_some() || !f.uses.is_empty())
            {
                self.error_here(format!("a signature has only one `{}`", CLAUSES[rank]));
            }
            stage = stage.max(rank);
            self.bump();
            match rank {
                0 => {
                    f.ret_secret = self.eat_kw(Keyword::Secret);
                    f.ret = Some(self.type_expr());
                }
                1 => f.errors = Some(self.type_expr()),
                2 => self.uses_clause(f),
                3 => f.requires.push(self.expr()),
                4 => f.ensures.push(self.expr()),
                _ => {
                    let expr = self.expr();
                    let fails = self.at_word("fails").then(|| {
                        self.bump();
                        self.expr()
                    });
                    f.examples.push(Example { expr, fails });
                }
            }
        }
    }

    /// `uses Net, Fs.read("./data"), Process("git")` (spec 8.1, 14.23):
    /// capabilities, the folder file access is kept inside, and the one
    /// program that may be run.
    fn uses_clause(&mut self, f: &mut FnDecl) {
        loop {
            let path = self.path("a capability");
            let narrowed = match path.segments[0].name.as_str() {
                "Fs" => Some(("the folder", "`Fs(\"./data\")`")),
                "Process" => Some(("the program", "`Process(\"git\")`")),
                _ => None,
            };
            if let (true, Some((what, how))) = (self.at(&TokenKind::LParen), narrowed) {
                self.bump();
                let start = self.span();
                let text = match self.at(&TokenKind::StrStart).then(|| self.string()) {
                    Some(ExprKind::Str(parts)) => match parts.as_slice() {
                        [StrPart::Text(t)] => Some(t.clone()),
                        [] => Some(String::new()),
                        _ => None,
                    },
                    _ => None,
                };
                match (text, path.segments[0].name.as_str()) {
                    (Some(t), "Fs") => f.fs_scope = Some(t),
                    (Some(t), _) => f.process_scope = Some(t),
                    (None, _) => self.error(start, format!("write {what} as plain text: {how}")),
                }
                self.expect(&TokenKind::RParen, &format!("`)` after {what}"));
            }
            f.uses.push(path);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
    }

    /// `deprecated "message" fix expression` and its line break, before a
    /// function (spec 12.6).
    pub(crate) fn deprecation(&mut self) -> Option<Box<Deprecation>> {
        let start = self.span();
        if !self.eat_kw(Keyword::Deprecated) {
            return None;
        }
        let message = match self.at(&TokenKind::StrStart).then(|| self.string()) {
            Some(ExprKind::Str(parts)) => match parts.as_slice() {
                [StrPart::Text(t)] => t.clone(),
                _ => String::new(),
            },
            _ => {
                self.error(
                    start,
                    "say what to use instead, in quotes: `deprecated \"use refund_order instead\"`",
                );
                String::new()
            }
        };
        let fix = self.at_word("fix").then(|| {
            self.bump();
            self.expr()
        });
        let span = self.span_from(start);
        self.skip_newlines();
        Some(Box::new(Deprecation { message, fix, span }))
    }
}
