//! Prints syntax trees as short S-expressions, e.g. `(+ 1 (* 2 3))`, so
//! tests can compare a whole tree in one line.
// Each test file uses a different subset of these helpers.
#![allow(dead_code)]

use kavach_ast::*;
use kavach_parser::parse;

mod items;
pub use items::item;

/// Parses `src` and prints its items, one per line. Fails on syntax errors.
pub fn tree(src: &str) -> String {
    let parsed = parse(src);
    assert!(
        parsed.errors.is_empty(),
        "unexpected errors in {src:?}: {:?}",
        parsed.errors
    );
    parsed
        .file
        .items
        .iter()
        .map(item)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The syntax error messages for `src`, each with its line number.
pub fn errors(src: &str) -> Vec<String> {
    parse(src)
        .errors
        .iter()
        .map(|e| format!("{}: {}", line_of(src, e.span.start as usize), e.message))
        .collect()
}

pub fn line_of(src: &str, offset: usize) -> usize {
    src[..offset].matches('\n').count() + 1
}

/// Checks every `(source, expected tree)` pair and reports all mismatches.
pub fn check_all(cases: &[(&str, &str)]) {
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(src, want)| {
            let got = tree(src);
            (got != *want).then(|| format!("source: {src:?}\n  want: {want}\n   got: {got}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

fn list<T>(items: &[T], f: impl Fn(&T) -> String) -> String {
    items.iter().map(f).collect::<Vec<_>>().join(" ")
}

pub fn path(p: &Path) -> String {
    list(&p.segments, |s| s.name.clone()).replace(' ', ".")
}

pub fn ty(t: &TypeExpr) -> String {
    match &t.kind {
        TypeKind::Named { path: p, args } if args.is_empty() => path(p),
        TypeKind::Named { path: p, args } => format!("{}<{}>", path(p), list(args, ty)),
        TypeKind::List(i) => format!("[{}]", ty(i)),
        TypeKind::Map(k, v) => format!("[{}: {}]", ty(k), ty(v)),
        TypeKind::Optional(i) => format!("{}?", ty(i)),
        TypeKind::Tuple(ts) => format!("({})", list(ts, ty)),
        TypeKind::AnonRecord(fs) => format!(
            "(anon {})",
            list(fs, |(n, t)| format!("{}: {}", n.name, ty(t)))
        ),
        TypeKind::Fn {
            params,
            ret,
            errors,
            uses,
        } => {
            let r = ret
                .as_ref()
                .map(|r| format!(" -> {}", ty(r)))
                .unwrap_or_default();
            let e = errors
                .as_ref()
                .map(|e| format!(" ! {}", ty(e)))
                .unwrap_or_default();
            let u = uses
                .as_ref()
                .map(|u| format!(" uses ({})", list(u, path)))
                .unwrap_or_default();
            format!("fn({}){r}{e}{u}", list(params, ty))
        }
        TypeKind::Error => "ERR".to_string(),
    }
}

pub fn block(b: &Block) -> String {
    format!("{{{}}}", list(&b.stmts, stmt))
}

pub fn stmt(s: &Stmt) -> String {
    match &s.kind {
        StmtKind::Let {
            mutable,
            pattern: p,
            ty: t,
            value,
        } => {
            let t = t
                .as_ref()
                .map(|t| format!(":{}", ty(t)))
                .unwrap_or_default();
            format!(
                "({} {}{t} {})",
                if *mutable { "let" } else { "const" },
                pattern(p),
                expr(value)
            )
        }
        StmtKind::Assign { target, op, value } => {
            let op = op
                .binary()
                .map_or("=".to_string(), |b| format!("{}=", b.as_str()));
            format!("({op} {} {})", expr(target), expr(value))
        }
        StmtKind::For {
            label,
            pattern: p,
            iter,
            body,
        } => format!(
            "{}(for {} {} {})",
            labeled(label),
            pattern(p),
            expr(iter),
            block(body)
        ),
        StmtKind::While { label, cond, body } => {
            format!("{}(while {} {})", labeled(label), expr(cond), block(body))
        }
        StmtKind::Assert(e) => format!("(assert {})", expr(e)),
        StmtKind::Expr(e) => expr(e),
    }
}

pub fn pattern(p: &Pattern) -> String {
    match &p.kind {
        PatternKind::Wildcard => "_".to_string(),
        PatternKind::Binding(i) => i.name.clone(),
        PatternKind::Literal(e) => expr(e),
        PatternKind::Range { lo, hi, inclusive } => format!(
            "({} {} {})",
            if *inclusive { "..=" } else { ".." },
            expr(lo),
            expr(hi)
        ),
        PatternKind::Tuple(ps) => format!("(tuple {})", list(ps, pattern)),
        PatternKind::List { items, rest } => {
            let mut parts: Vec<String> = items
                .iter()
                .map(|it| {
                    let key = match &it.key {
                        Some(MapKey::Name(n)) => format!("{}:", n.name),
                        Some(MapKey::Literal(e)) => format!("{}:", expr(e)),
                        None => String::new(),
                    };
                    let default = it.default.as_ref().map(|d| format!("={}", expr(d)));
                    format!(
                        "{key}{}{}",
                        pattern(&it.pattern),
                        default.unwrap_or_default()
                    )
                })
                .collect();
            parts.extend(rest.iter().map(|r| format!("...{}", r.name)));
            format!("(list {})", parts.join(" "))
        }
        PatternKind::Record(fields) => {
            let parts: Vec<String> = fields
                .iter()
                .map(|f| {
                    let default = f.default.as_ref().map(|d| format!("={}", expr(d)));
                    format!(
                        "{}:{}{}",
                        f.field.name,
                        pattern(&f.pattern),
                        default.unwrap_or_default()
                    )
                })
                .collect();
            format!("(record {})", parts.join(" "))
        }
        PatternKind::None => "none".to_string(),
        PatternKind::Variant {
            path: v,
            fields: None,
        } => path(v),
        PatternKind::Variant {
            path: v,
            fields: Some(fs),
        } => format!(
            "({} {})",
            path(v),
            list(fs, |f| match &f.name {
                Some(n) => format!("{}:{}", n.name, pattern(&f.pattern)),
                None => pattern(&f.pattern),
            })
        ),
        PatternKind::Ok(p) => format!("(ok {})", pattern(p)),
        PatternKind::Err(p) => format!("(err {})", pattern(p)),
    }
}

fn else_part(e: &Option<Box<Expr>>) -> String {
    e.as_ref()
        .map(|e| format!(" {}", expr(e)))
        .unwrap_or_default()
}

pub fn expr(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Int(v) => v.to_string(),
        ExprKind::Float(t) | ExprKind::BigInt(t) => t.clone(),
        ExprKind::Char(c) => format!("{c:?}"),
        ExprKind::Str(parts) => format!(
            "(str {})",
            list(parts, |p| match p {
                StrPart::Text(t) => format!("{t:?}"),
                StrPart::Expr(e) => expr(e),
            })
        ),
        ExprKind::Bool(b) => b.to_string(),
        ExprKind::Tagged { tag, args } => {
            let path: Vec<&str> = tag.segments.iter().map(|s| s.name.as_str()).collect();
            format!(
                "(tagged {} {})",
                path.join("."),
                list(args, |a| expr(&a.value))
            )
        }
        ExprKind::None => "none".to_string(),
        ExprKind::Name(i) => i.name.clone(),
        ExprKind::SelfValue => "self".to_string(),
        ExprKind::It => "it".to_string(),
        ExprKind::List(items) => format!("(list {})", list(items, expr)),
        ExprKind::AnonRecord(fs) => format!(
            "(anon {})",
            list(fs, |(n, x)| format!("{}: {}", n.name, expr(x)))
        ),
        ExprKind::Map(entries) => format!(
            "(map {})",
            list(entries, |(k, v)| format!("({} {})", expr(k), expr(v)))
        ),
        ExprKind::Tuple(items) => format!("(tuple {})", list(items, expr)),
        ExprKind::Unary(op, e) => {
            let op = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "not",
                UnaryOp::BitNot => "~",
                UnaryOp::Deref => "*",
            };
            format!("({op} {})", expr(e))
        }
        ExprKind::Binary(op, l, r) => format!("({} {} {})", op.as_str(), expr(l), expr(r)),
        ExprKind::Compare(first, links) => {
            let rest: Vec<String> = links
                .iter()
                .map(|(op, x)| format!("{} {}", op.as_str(), expr(x)))
                .collect();
            format!("(chain {} {})", expr(first), rest.join(" "))
        }
        ExprKind::Call {
            callee,
            generics,
            args,
        } => {
            let g = if generics.is_empty() {
                String::new()
            } else {
                format!("<{}>", list(generics, ty))
            };
            let a = list(args, |a| match &a.name {
                Some(n) => format!("{}:{}", n.name, expr(&a.value)),
                None => expr(&a.value),
            });
            if a.is_empty() {
                format!("(call {}{g})", expr(callee))
            } else {
                format!("(call {}{g} {a})", expr(callee))
            }
        }
        ExprKind::Field {
            base,
            name,
            optional,
        } => format!(
            "({} {} {})",
            if *optional { "?." } else { "." },
            expr(base),
            name.name
        ),
        ExprKind::TupleField { base, index } => format!("(. {} {index})", expr(base)),
        ExprKind::Index { base, index } => format!("(index {} {})", expr(base), expr(index)),
        ExprKind::With { base, fields } => format!(
            "(with {} {})",
            expr(base),
            list(fields, |(n, v)| format!("{}:{}", n.name, expr(v)))
        ),
        ExprKind::If {
            cond,
            then,
            otherwise,
        } => format!(
            "(if {} {}{})",
            expr(cond),
            block(then),
            else_part(otherwise)
        ),
        ExprKind::IfLet {
            name,
            value,
            then,
            otherwise,
        } => {
            format!(
                "(if-let {} {} {}{})",
                name.name,
                expr(value),
                block(then),
                else_part(otherwise)
            )
        }
        ExprKind::Match { scrutinee, arms } => format!(
            "(match {} {})",
            expr(scrutinee),
            list(arms, |a| match &a.guard {
                Some(g) => format!(
                    "({} if {} => {})",
                    pattern(&a.pattern),
                    expr(g),
                    expr(&a.body)
                ),
                None => format!("({} => {})", pattern(&a.pattern), expr(&a.body)),
            })
        ),
        ExprKind::Block(b) => block(b),
        ExprKind::Lambda { params, body } => format!(
            "(fn ({}) {})",
            list(params, |p| p.name.name.clone()),
            expr(body)
        ),
        ExprKind::DotLambda(body) => format!("(dot-fn {})", expr(body)),
        ExprKind::Try {
            expr: e,
            context: None,
        } => format!("(try {})", expr(e)),
        ExprKind::Try {
            expr: e,
            context: Some(c),
        } => format!("(try {} context {})", expr(e), expr(c)),
        ExprKind::Catch {
            expr: e,
            binding: Some(b),
            handler,
        } => format!("(catch {} {} => {})", expr(e), b.name, expr(handler)),
        ExprKind::Catch {
            expr: e,
            binding: None,
            handler,
        } => format!("(catch {} {})", expr(e), expr(handler)),
        ExprKind::Fail(e) => format!("(fail {})", expr(e)),
        ExprKind::Return(None) => "(return)".to_string(),
        ExprKind::Return(Some(e)) => format!("(return {})", expr(e)),
        ExprKind::Break(None) => "break".to_string(),
        ExprKind::Continue(None) => "continue".to_string(),
        ExprKind::Break(Some(l)) => format!("(break {})", l.name),
        ExprKind::Continue(Some(l)) => format!("(continue {})", l.name),
        ExprKind::Spawn(kind, e, within) => {
            let within = within
                .as_ref()
                .map_or(String::new(), |d| format!(" within {}", expr(d)));
            match kind.word() {
                Some(w) => format!("(spawn {w} {}{within})", expr(e)),
                None => format!("(spawn {}{within})", expr(e)),
            }
        }
        ExprKind::Arena(b) => format!("(arena {})", block(b)),
        ExprKind::Unsafe(b) => format!("(unsafe {})", block(b)),
        ExprKind::Error => "ERR".to_string(),
    }
}

/// `name:` before a labeled loop (spec 4.6).
fn labeled(label: &Option<kavach_ast::Ident>) -> String {
    label
        .as_ref()
        .map_or(String::new(), |l| format!("{}:", l.name))
}
