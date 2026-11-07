//! Found cases (spec 11.4): failing inputs saved in `tests/found/`, one
//! file per module, and replayed first on every run.
//!
//! Each line is one call, written the way Kavach code writes values:
//!
//! ```text
//! withdraw(Account(owner: "", balance: 0.1), 0.2)
//! Account.deposit(Account(owner: "a", balance: 0), -1)
//! ```
//!
//! Reading a line is guided by the parameter types, so the format needs
//! no type names beyond the ones the values already show.

use super::values::Gen;
use super::{Target, TestOptions};
use crate::interp::Interp;
use crate::map::MapValue;
use crate::value::{Field, Value};
use kavach_ast::TypeBody;
use kavach_runtime_core::Decimal;
use std::path::PathBuf;
use std::rc::Rc;

/// One call as a line of a found-cases file, its values written as code
/// (`i.code_of`).
pub(crate) fn write_case(i: &Interp<'_>, name: &str, args: &[Value<'_>]) -> String {
    let parts: Vec<String> = args.iter().map(|a| i.code_of(a)).collect();
    format!("{name}({})", parts.join(", "))
}

impl Interp<'_> {
    /// `v` as a program writes it: each variant with its union's name
    /// (decision 0070).
    pub(crate) fn code_of(&self, v: &Value<'_>) -> String {
        let union = |d| {
            self.unions
                .get(&d)
                .map_or(String::new(), |u| u.name.name.clone())
        };
        v.source_code(&union)
    }
}

/// Reads values from text written by [`write_case`].
pub(crate) struct Reader<'s> {
    s: &'s str,
}

impl<'s> Reader<'s> {
    pub(crate) fn new(s: &'s str) -> Self {
        Reader { s }
    }

    fn eat(&mut self, token: &str) -> bool {
        match self.s.strip_prefix(token) {
            Some(rest) => {
                self.s = rest;
                true
            }
            None => false,
        }
    }

    pub(crate) fn expect(&mut self, token: &str) -> Option<()> {
        self.eat(token).then_some(())
    }

    pub(crate) fn is_done(&self) -> bool {
        self.s.trim().is_empty()
    }

    /// Characters up to (not including) the first one not in `keep`.
    fn take(&mut self, keep: impl Fn(char) -> bool) -> &'s str {
        let end = self.s.find(|c| !keep(c)).unwrap_or(self.s.len());
        let (word, rest) = self.s.split_at(end);
        self.s = rest;
        word
    }

    pub(crate) fn name(&mut self) -> &'s str {
        self.take(|c| c.is_alphanumeric() || c == '_' || c == '.')
    }

    fn number(&mut self) -> &'s str {
        self.take(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
    }

    /// Text in Rust's escaped form, as values print (`{:?}`).
    fn quoted(&mut self, quote: char) -> Option<String> {
        self.expect(&quote.to_string())?;
        let mut out = String::new();
        let mut chars = self.s.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                c if c == quote => {
                    self.s = &self.s[i + 1..];
                    return Some(out);
                }
                '\\' => match chars.next()?.1 {
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    '0' => out.push('\0'),
                    'u' => {
                        let mut hex = String::new();
                        chars.next()?;
                        for (_, h) in chars.by_ref() {
                            if h == '}' {
                                break;
                            }
                            hex.push(h);
                        }
                        out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                    }
                    other => out.push(other),
                },
                c => out.push(c),
            }
        }
        None
    }

    /// A value of type `g`.
    pub(crate) fn value<'a>(&mut self, g: &Gen, i: &Interp<'a>) -> Option<Value<'a>> {
        if let Gen::Optional(inner) = g {
            return if self.eat("none") {
                Some(Value::None)
            } else {
                self.value(inner, i)
            };
        }
        Some(match g {
            Gen::Int(p) => Value::Int(self.number().parse().ok()?, *p),
            Gen::Float => Value::Float(self.number().parse().ok()?),
            Gen::Decimal => Value::Decimal(Decimal::parse(self.number())?),
            Gen::Bool => Value::Bool(self.name().parse().ok()?),
            Gen::Char => Value::Char(self.quoted('\'')?.chars().next()?),
            Gen::Str => Value::Str(Rc::from(self.quoted('"')?)),
            Gen::Bytes => {
                self.expect("Bytes(")?;
                let mut data = Vec::new();
                while !self.eat(")") {
                    let hex = self.number();
                    for pair in hex.as_bytes().chunks(2) {
                        let pair = std::str::from_utf8(pair).ok()?;
                        data.push(u8::from_str_radix(pair, 16).ok()?);
                    }
                    if hex.is_empty() {
                        return None;
                    }
                }
                Value::Bytes(data.into())
            }
            Gen::List(item) => {
                self.expect("[")?;
                let mut items = Vec::new();
                while !self.eat("]") {
                    if !items.is_empty() {
                        self.expect(", ")?;
                    }
                    items.push(self.value(item, i)?);
                }
                Value::List(Rc::new(items))
            }
            Gen::Map(k, v) => {
                let mut map = MapValue::default();
                if !self.eat("[:]") {
                    self.expect("[")?;
                    while !self.eat("]") {
                        if !map.is_empty() {
                            self.expect(", ")?;
                        }
                        let key = self.value(k, i)?;
                        self.expect(": ")?;
                        map.insert(key, self.value(v, i)?);
                    }
                }
                Value::Map(Rc::new(map))
            }
            Gen::Tuple(gs) => {
                self.expect("(")?;
                let mut items = Vec::new();
                for (n, g) in gs.iter().enumerate() {
                    if n > 0 {
                        self.expect(", ")?;
                    }
                    items.push(self.value(g, i)?);
                }
                self.expect(")")?;
                Value::Tuple(items)
            }
            Gen::Adt(def) => {
                let decl = i.records.get(def).or_else(|| i.unions.get(def))?;
                let mut written = self.name();
                // `Union.Name` (decision 0070); files written before it say `Name`.
                if written == decl.name.name && i.unions.contains_key(def) && self.eat(".") {
                    written = self.name();
                }
                let fields = match &decl.body {
                    TypeBody::Record(r) => &r.fields,
                    TypeBody::Union(vs) => &vs.iter().find(|v| v.name.name == written)?.fields,
                    TypeBody::Alias(_) => return None,
                };
                let mut values = Vec::new();
                if !fields.is_empty() {
                    self.expect("(")?;
                    for (n, f) in fields.iter().enumerate() {
                        if n > 0 {
                            self.expect(", ")?;
                        }
                        self.expect(&format!("{}: ", f.name.name))?;
                        values.push(Field {
                            name: Rc::from(f.name.name.as_str()),
                            value: self.value(&i.gen_of(&f.ty)?, i)?,
                            secret: f.is_secret,
                        });
                    }
                    self.expect(")")?;
                }
                match &decl.body {
                    TypeBody::Record(_) => Value::Record {
                        def: *def,
                        name: Rc::from(written),
                        fields: values,
                        life: None,
                    },
                    TypeBody::Alias(_) => return None,
                    TypeBody::Union(_) => Value::Variant {
                        union: *def,
                        name: Rc::from(written),
                        fields: values,
                    },
                }
            }
            Gen::Optional(_) => unreachable!("handled above"),
        })
    }
}

/// Whether a value can be written to a found-cases file: secrets print
/// as `<secret>`, so a value holding one can't be read back.
pub(crate) fn can_save(v: &Value<'_>) -> bool {
    !v.display_code().contains("<secret>")
}

/// The files the found cases are read from and saved to.
impl<'a> Interp<'a> {
    /// The file of found cases for `t`'s module (spec 11.4): one file per
    /// module, `tests/found/orders.kavf` for `orders.kav`.
    pub(super) fn found_file(&self, opts: &TestOptions, t: &Target<'a>) -> Option<PathBuf> {
        let base = opts.found_file.as_ref()?;
        let res = &self.c.resolution;
        let module = &res.modules.get(res.module_at(t.f.name.span.start))?.name;
        let entry = res.modules.first().map(|m| m.name.as_str());
        match module.is_empty() || Some(module.as_str()) == entry {
            true => Some(base.clone()),
            false => Some(base.with_file_name(format!("{module}.kavf"))),
        }
    }

    /// The found cases saved in `file`, one call per line.
    pub(super) fn saved_cases(file: &std::path::Path) -> Vec<String> {
        std::fs::read_to_string(file)
            .map(|s| {
                s.lines()
                    .filter(|l| !l.starts_with("//") && !l.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Adds each new case to its module's file, and says where.
    pub(super) fn save_found(&mut self, found: &[(PathBuf, String)]) {
        let mut files: Vec<&PathBuf> = found.iter().map(|(f, _)| f).collect();
        files.dedup();
        for path in files {
            let lines = found.iter().filter(|(f, _)| f == path).map(|(_, l)| l);
            self.save_lines(path, lines);
        }
    }

    fn save_lines<'l>(&mut self, path: &PathBuf, lines: impl Iterator<Item = &'l String>) {
        let mut text = std::fs::read_to_string(path).unwrap_or_else(|_| {
            "// Found by `kavach test` (spec 11.4). Generated: commit it, never edit it.\n"
                .to_string()
        });
        for l in lines {
            text.push_str(l);
            text.push('\n');
        }
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::write(path, text));
        match written {
            Ok(()) => self.out.push_str(&format!(
                "  saved to {}  (replayed on every run)\n",
                kavach_driver::relative_path(path)
            )),
            Err(e) => self
                .out
                .push_str(&format!("  can't save found cases: {e}\n")),
        }
    }
}
