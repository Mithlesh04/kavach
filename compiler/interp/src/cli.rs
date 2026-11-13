//! `cli.parse<T>()` (spec 14.6): command-line options read from the
//! fields of record type `T`, or a command and its options read from the
//! variants of union type `T`.

use crate::interp::Interp;
use crate::value::{Eval, Field, Flow, Value};
use kavach_ast::{Field as FieldDecl, Span, TypeBody, TypeExpr, TypeKind, Variant};
use kavach_checker::Prim;
use kavach_runtime_core::Decimal;
use std::rc::Rc;

/// The option for field `name`: `dry_run` is `--dry-run`.
pub(crate) fn option(name: &str) -> String {
    name.replace('_', "-")
}

/// The command for variant `name`: `DryRun` is `dry-run`.
pub(crate) fn command(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('-');
        }
        out.extend(c.to_lowercase());
    }
    out
}

fn is_flag(t: &TypeExpr) -> bool {
    matches!(&t.kind, TypeKind::Named { path, .. } if path.segments[0].name == "Bool")
}

fn program(i: &Interp<'_>) -> String {
    std::path::Path::new(&i.c.path)
        .file_stem()
        .map_or("program".into(), |s| s.to_string_lossy().to_string())
}

/// One line per option, with its type and default, indented by `indent`.
fn option_lines(i: &Interp<'_>, fields: &[FieldDecl], indent: usize) -> Vec<String> {
    let width = 26 - indent;
    fields
        .iter()
        .map(|f| {
            let name = format!("--{}", option(&f.name.name));
            let shown = if is_flag(&f.ty) {
                name
            } else {
                format!("{name} <{}>", i.text(f.ty.span).trim_end_matches('?'))
            };
            let note = match (&f.default, &f.ty.kind) {
                (Some(d), _) if !is_flag(&f.ty) => format!("(default {})", i.text(d.span)),
                (None, TypeKind::Optional(_)) => "(optional)".into(),
                (None, _) if !is_flag(&f.ty) => "(required)".into(),
                _ => String::new(),
            };
            format!("{:indent$}{shown:<width$} {note}", "")
                .trim_end()
                .to_string()
        })
        .collect()
}

/// The usage text of a record of options, or of command `cmd`'s options.
pub(crate) fn usage(i: &Interp<'_>, cmd: Option<&str>, fields: &[FieldDecl]) -> String {
    let words = match cmd {
        Some(cmd) => format!("{} {cmd}", program(i)),
        None => program(i),
    };
    let mut lines = vec![format!("usage: {words} [options]")];
    lines.extend(option_lines(i, fields, 2));
    lines.join("\n")
}

/// The usage text of a union of commands: every command with its options.
pub(crate) fn commands_usage(i: &Interp<'_>, variants: &[Variant]) -> String {
    let mut lines = vec![format!("usage: {} <command> [options]", program(i))];
    for v in variants {
        lines.push(format!("  {}", command(&v.name.name)));
        lines.extend(option_lines(i, &v.fields, 4));
    }
    lines.join("\n")
}

/// The text given for each field, in field order (`"true"` for a flag),
/// or the problem with the arguments.
pub(crate) fn read_args(
    args: &[String],
    fields: &[FieldDecl],
) -> Result<Vec<Option<String>>, String> {
    let mut given: Vec<Option<String>> = vec![None; fields.len()];
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == "--help" || arg == "-h" {
            return Err("help:".into());
        }
        let Some(word) = arg.strip_prefix("--") else {
            return Err(format!(
                "unexpected argument `{arg}`: options start with `--`"
            ));
        };
        let (name, inline) = match word.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (word, None),
        };
        let Some(i) = fields.iter().position(|f| option(&f.name.name) == name) else {
            return Err(format!("unknown option `--{name}`"));
        };
        let value = match (inline, is_flag(&fields[i].ty)) {
            (Some(v), _) => v,
            (None, true) => "true".into(),
            (None, false) => rest
                .next()
                .cloned()
                .ok_or(format!("--{name} needs a value"))?,
        };
        given[i] = Some(value);
    }
    Ok(given)
}

/// The variant the first argument names, or the problem.
pub(crate) fn read_command(args: &[String], variants: &[Variant]) -> Result<usize, String> {
    match args.first().map(String::as_str) {
        None => Err("missing command".into()),
        Some("--help" | "-h") => Err("help:".into()),
        Some(word) => variants
            .iter()
            .position(|v| command(&v.name.name) == word)
            .ok_or(format!("unknown command `{word}`")),
    }
}

/// `text` as a value of written type `t`, or `None` if it isn't one.
pub(crate) fn convert<'a>(i: &Interp<'a>, text: &str, t: &TypeExpr) -> Option<Value<'a>> {
    let t = match &t.kind {
        TypeKind::Optional(inner) => inner,
        _ => t,
    };
    let TypeKind::Named { path, .. } = &t.kind else {
        return None;
    };
    Some(match Prim::from_name(&path.segments[0].name)? {
        Prim::String => i.str_value(text),
        Prim::Bool => Value::Bool(text.parse().ok()?),
        Prim::Float32 => Value::Float32(text.parse().ok()?),
        Prim::Float => Value::Float(text.parse().ok()?),
        Prim::Decimal => Value::Decimal(Decimal::parse(text)?),
        p => Value::Int(crate::ops::fits(text.parse().ok()?, p)?, p),
    })
}

/// A failed parse: the problem, then the usage. `--help` gives the usage
/// alone.
fn fail<'a>(message: String, usage: &str) -> Flow<'a> {
    let text = if message == "help:" {
        usage.to_string()
    } else {
        format!("{message}\n{usage}")
    };
    Flow::Fail(Value::BuiltinError(text.into()))
}

impl<'a> Interp<'a> {
    /// `cli.parse<T>()`: the program's arguments as `T` (spec 14.6).
    pub(crate) fn cli_parse(&mut self, generics: &'a [TypeExpr], span: Span) -> Eval<'a> {
        let def = generics.first().and_then(|t| match &t.kind {
            TypeKind::Named { path, .. } => {
                self.c.resolution.uses.get(&path.segments[0].span.start)
            }
            _ => None,
        });
        let decl = def.and_then(|d| self.records.get(d).or(self.unions.get(d)));
        let (Some(&def), Some(decl)) = (def, decl.copied()) else {
            return Err(self.panic("`cli.parse` needs a record or union type", span));
        };
        let args = self.args.clone();
        match &decl.body {
            TypeBody::Record(r) => {
                let usage = usage(self, None, &r.fields);
                let fields = self.cli_fields(&args, &r.fields, &usage)?;
                let value = Value::Record {
                    def,
                    name: Rc::from(decl.name.name.as_str()),
                    fields,
                    life: None,
                };
                self.check_invariants(&value, span)?;
                Ok(value)
            }
            TypeBody::Alias(_) => Err(self.panic("`cli.parse` needs a record or union type", span)),
            TypeBody::Union(variants) => {
                let v = &variants[read_command(&args, variants)
                    .map_err(|m| fail(m, &commands_usage(self, variants)))?];
                let usage = usage(self, Some(&command(&v.name.name)), &v.fields);
                let fields = self.cli_fields(&args[1..], &v.fields, &usage)?;
                Ok(Value::Variant {
                    union: def,
                    name: Rc::from(v.name.name.as_str()),
                    fields,
                    life: None,
                })
            }
        }
    }

    /// The fields read from `args`, with defaults filled in.
    fn cli_fields(
        &mut self,
        args: &[String],
        decls: &'a [FieldDecl],
        usage: &str,
    ) -> Result<Vec<Field<'a>>, Flow<'a>> {
        let given = read_args(args, decls).map_err(|m| fail(m, usage))?;
        let mut fields = Vec::new();
        for (f, value) in decls.iter().zip(given) {
            let name = option(&f.name.name);
            let value = match (value, &f.default, &f.ty.kind) {
                (Some(text), _, _) => convert(self, &text, &f.ty).ok_or_else(|| {
                    let written = self.text(f.ty.span);
                    fail(format!("--{name} needs {written}, not `{text}`"), usage)
                })?,
                (None, Some(d), _) => self.eval(d)?,
                (None, None, TypeKind::Optional(_)) => Value::None,
                (None, None, _) => return Err(fail(format!("missing --{name}"), usage)),
            };
            fields.push(Field {
                name: Rc::from(f.name.name.as_str()),
                value,
                secret: f.is_secret,
            });
        }
        Ok(fields)
    }
}
