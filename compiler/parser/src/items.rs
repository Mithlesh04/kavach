//! Declarations: the file, types, interfaces, `extend`, capabilities,
//! agents, `use` and `test`. Functions are in `fns.rs`.

use crate::parser::Parser;
use kavach_ast::*;
use kavach_lexer::{Keyword, TokenKind};

impl Parser<'_> {
    pub(crate) fn file(&mut self) -> File {
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&TokenKind::Eof) {
                break;
            }
            if self.at(&TokenKind::RBrace) {
                // The lexer has already reported this unmatched `}`.
                self.bump();
                continue;
            }
            let before = self.position();
            if self.at_kw(Keyword::Extern) || (self.at_kw(Keyword::Pub) && self.peek_is_extern()) {
                items.extend(self.extern_block());
                self.end_of_stmt();
                continue;
            }
            items.push(self.item());
            self.end_of_stmt();
            if self.position() == before {
                self.bump();
            }
        }
        File { items }
    }

    pub(crate) fn docs(&mut self) -> Vec<String> {
        let mut docs = Vec::new();
        while let TokenKind::DocComment(text) = self.kind().clone() {
            docs.push(text);
            self.bump();
            self.skip_newlines();
        }
        docs
    }

    pub(crate) fn item(&mut self) -> Item {
        let start = self.span();
        let docs = self.docs();
        let deprecated = self.deprecation();
        let is_pub = self.eat_kw(Keyword::Pub);
        let mut kind = match self.kind() {
            TokenKind::Keyword(Keyword::Fn) => ItemKind::Fn(self.fn_decl(true)),
            // `gpu fn` (spec 15.1): a word only before `fn`.
            TokenKind::Ident
                if self.at_word("gpu") && *self.nth(1) == TokenKind::Keyword(Keyword::Fn) =>
            {
                self.bump();
                let mut f = self.fn_decl(true);
                f.is_gpu = true;
                ItemKind::Fn(f)
            }
            TokenKind::Keyword(Keyword::Type) => ItemKind::Type(self.type_decl()),
            TokenKind::Keyword(Keyword::Interface) => {
                self.bump();
                let name = self.ident("an interface name");
                let generics = self.generic_params();
                let methods = self.methods(false);
                ItemKind::Interface(InterfaceDecl {
                    name,
                    generics,
                    methods,
                })
            }
            TokenKind::Keyword(Keyword::Extend) => {
                self.bump();
                let target = self.type_expr();
                let implements = self.implements();
                let methods = self.methods(true);
                ItemKind::Extend(ExtendDecl {
                    target,
                    implements,
                    methods,
                })
            }
            TokenKind::Keyword(Keyword::Capability) => {
                self.bump();
                let name = self.ident("a capability name");
                let methods = self.methods(false);
                ItemKind::Capability(CapabilityDecl { name, methods })
            }
            TokenKind::Keyword(Keyword::Agent) => ItemKind::Agent(self.agent()),
            TokenKind::Keyword(Keyword::Use) => ItemKind::Use(self.use_decl()),
            TokenKind::Keyword(Keyword::Test) => ItemKind::Test(self.test_decl()),
            _ => {
                if is_pub {
                    self.error_here("`pub` only goes before a declaration such as `fn` or `type`");
                }
                ItemKind::Stmt(self.stmt())
            }
        };
        if let Some(d) = deprecated {
            match &mut kind {
                ItemKind::Fn(f) => f.deprecated = Some(d),
                _ => self.error(
                    d.span,
                    "`deprecated` only goes before a function (spec 12.6)",
                ),
            }
        }
        Item {
            kind,
            docs,
            is_pub,
            span: self.span_from(start),
        }
    }

    /// `{ fn … }` bodies of interfaces, capabilities and `extend`.
    fn methods(&mut self, body_required: bool) -> Vec<Item> {
        let mut methods = Vec::new();
        self.members(|p| {
            let start = p.span();
            let docs = p.docs();
            let is_pub = p.eat_kw(Keyword::Pub);
            if p.at_kw(Keyword::Fn) {
                let kind = ItemKind::Fn(p.fn_decl(body_required));
                methods.push(Item {
                    kind,
                    docs,
                    is_pub,
                    span: p.span_from(start),
                });
            } else {
                p.error_here("expected `fn`");
                p.recover_line();
            }
        });
        methods
    }

    /// Reads `{ member ⏎ member … }`, one member per line.
    pub(crate) fn members(&mut self, mut member: impl FnMut(&mut Self)) {
        if !self.expect(&TokenKind::LBrace, "`{`") {
            return;
        }
        loop {
            self.skip_newlines();
            if self.at(&TokenKind::RBrace) || self.at(&TokenKind::Eof) {
                break;
            }
            let before = self.position();
            member(self);
            self.end_of_stmt();
            if self.position() == before {
                self.bump();
            }
        }
        self.expect(&TokenKind::RBrace, "`}`");
    }

    fn agent(&mut self) -> AgentDecl {
        self.bump();
        let name = self.ident("an agent name");
        let mut members = Vec::new();
        self.members(|p| {
            p.docs();
            let word = if p.at(&TokenKind::Ident) {
                p.text()
            } else {
                ""
            };
            match word {
                "can" => {
                    p.bump();
                    members.push(AgentMember::Can(p.paths("a capability")));
                }
                "limit" => {
                    p.bump();
                    let target = p.path("a capability or method");
                    let mut rules = Vec::new();
                    p.members(|p| {
                        let expr = p.expr();
                        let per = p.at_word("per").then(|| {
                            p.bump();
                            p.ident("a time window such as `day`")
                        });
                        rules.push(LimitRule { expr, per });
                    });
                    members.push(AgentMember::Limit { target, rules });
                }
                "approve" => {
                    p.bump();
                    let target = p.path("a capability method");
                    p.expect_word("when");
                    let when = p.expr();
                    p.expect_word("by");
                    let by = p.expr();
                    let within = (p.at_word("within")).then(|| {
                        p.bump();
                        p.expr()
                    });
                    members.push(AgentMember::Approve {
                        target,
                        when,
                        by,
                        within,
                    });
                }
                "on" => {
                    p.bump();
                    let name = p.ident("an event name");
                    members.push(AgentMember::On(p.fn_rest(name, true)));
                }
                "model" => {
                    p.bump();
                    members.push(AgentMember::Model(p.expr()));
                }
                "settings" => {
                    p.bump();
                    members.push(AgentMember::Settings(p.expr()));
                }
                _ => {
                    p.error_here("expected `model`, `settings`, `can`, `limit`, `approve` or `on`");
                    p.recover_line();
                }
            }
        });
        AgentDecl { name, members }
    }

    fn use_decl(&mut self) -> UseDecl {
        self.bump();
        let start = self.span();
        let mut segments = vec![self.ident("a module name")];
        let mut names = Vec::new();
        while self.at(&TokenKind::Dot) {
            self.bump();
            if self.eat(&TokenKind::LBrace) {
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    names.push(self.ident("a name to bring in"));
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RBrace, "`}`");
                break;
            }
            segments.push(self.ident("a module name"));
        }
        let path = Path {
            segments,
            span: self.span_from(start),
        };
        let alias = self.eat_kw(Keyword::As).then(|| self.ident("a new name"));
        UseDecl { path, names, alias }
    }

    /// `extern c "name" { fn … }` (spec 10.7): a C library's functions,
    /// each an item of its own, marked with the library.
    fn extern_block(&mut self) -> Vec<Item> {
        let start = self.span();
        let docs = self.docs();
        let is_pub = self.eat_kw(Keyword::Pub);
        self.bump(); // `extern`
        let lang = self.ident("a language: `c`");
        let library = match self.at(&TokenKind::StrStart).then(|| self.string()) {
            Some(ExprKind::Str(parts)) => match parts.as_slice() {
                [StrPart::Text(t)] => t.clone(),
                _ => String::new(),
            },
            _ => {
                self.error(
                    start,
                    "write the library's name in quotes: `extern c \"m\"`",
                );
                String::new()
            }
        };
        if lang.name != "c" {
            self.error(
                lang.span,
                format!(
                    "`extern {}` comes in Phase 3; `extern c` works now (spec 10.7)",
                    lang.name
                ),
            );
        }
        let mut functions = self.methods(false);
        for item in &mut functions {
            item.is_pub = item.is_pub || is_pub;
            if item.docs.is_empty() {
                item.docs = docs.clone();
            }
            if let ItemKind::Fn(f) = &mut item.kind {
                if f.body.is_some() {
                    self.error(
                        f.name.span,
                        "a C function has no body here: its code is in the library",
                    );
                }
                f.c_library = Some(library.clone());
            }
        }
        functions
    }

    /// Whether the token after this `pub` is `extern`.
    fn peek_is_extern(&self) -> bool {
        matches!(self.nth(1), TokenKind::Keyword(Keyword::Extern))
    }

    fn test_decl(&mut self) -> TestDecl {
        self.bump();
        let name_start = self.span();
        let name = if self.at(&TokenKind::StrStart) {
            let kind = self.string();
            Expr {
                kind,
                span: self.span_from(name_start),
            }
        } else {
            self.error_here("expected the test's name in quotes");
            Expr {
                kind: ExprKind::Error,
                span: name_start,
            }
        };
        let uses = if self.eat_kw(Keyword::Uses) {
            self.paths("a capability")
        } else {
            Vec::new()
        };
        let mut with = Vec::new();
        if self.eat_kw(Keyword::With) {
            loop {
                let capability = self.ident("a capability name");
                self.expect(&TokenKind::Eq, "`=` and an implementation");
                with.push((capability, self.expr()));
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
        }
        let body = self.block();
        TestDecl {
            name,
            uses,
            with,
            body,
        }
    }
}
