//! Generated values (spec 11.3): edge cases first, then random ones.

use crate::interp::Interp;
use crate::map::MapValue;
use crate::ops::int_range;
use crate::value::{Field, Value};
use kavach_ast::{Span, TypeBody, TypeExpr, TypeKind};
use kavach_checker::Prim;
use kavach_resolver::{DefId, DefKind};
use kavach_runtime_core::Decimal;
use std::rc::Rc;

/// A type that values can be generated for.
#[derive(Clone, Debug)]
pub(crate) enum Gen {
    Int(Prim),
    Float,
    Decimal,
    Bool,
    Char,
    Str,
    /// Raw data (spec 2.13).
    Bytes,
    List(Box<Gen>),
    Map(Box<Gen>, Box<Gen>),
    Optional(Box<Gen>),
    Tuple(Vec<Gen>),
    /// A record or union declared in the file.
    Adt(DefId),
}

/// A small, fast random number generator (SplitMix64): the same seed
/// always gives the same values (spec 11.9).
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Rng(seed)
    }

    /// A generator for a spawned task: its own numbers, the same each run.
    pub(crate) fn fork(&self, salt: u64) -> Self {
        let mut r = Rng(self.0 ^ salt.wrapping_mul(0xA24B_AED4_963E_E407));
        r.next();
        r
    }

    /// `seed` mixed with `text`: a seed of its own for each named thing.
    pub(crate) fn hash(seed: u64, text: &str) -> u64 {
        text.bytes().fold(seed, |h, b| {
            h.wrapping_mul(0x100_0000_01B3).wrapping_add(u64::from(b))
        })
    }

    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n` (`n` > 0).
    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// A number in `lo..=hi`.
    pub(crate) fn range(&mut self, lo: i128, hi: i128) -> i128 {
        let span = (hi - lo + 1) as u128;
        lo + ((self.next() as u128 * 0x1_0000_0001 + self.next() as u128) % span) as i128
    }
}

/// How a value is chosen: the `n`th edge case, or at random.
#[derive(Clone, Copy)]
pub(crate) enum Pick {
    Edge(usize),
    Random,
}

/// Deeper than this, lists are empty and optionals `none`, so recursive
/// types stay small.
const MAX_DEPTH: usize = 3;

const TEXTS: [&str; 6] = ["", "a", " ", "नमस्ते", "Hello, World!", "0"];

impl<'a> Interp<'a> {
    /// The generator for a written type, or `None` if values of it can't
    /// be generated yet (functions, generics, capabilities).
    /// Whether type `def` is a tag's (spec 10.15), so only its literals
    /// make one.
    fn made_by_literals(&self, def: kavach_resolver::DefId) -> bool {
        self.c.file.items.iter().any(|item| match &item.kind {
            kavach_ast::ItemKind::Literal(l) => match &l.ty.kind {
                TypeKind::Named { path, .. } => {
                    self.c.resolution.uses.get(&path.segments[0].span.start) == Some(&def)
                }
                _ => false,
            },
            _ => false,
        })
    }

    pub(crate) fn gen_of(&self, t: &TypeExpr) -> Option<Gen> {
        Some(match &t.kind {
            TypeKind::Named { path, args } => {
                let first = &path.segments[0];
                let def = *self.c.resolution.uses.get(&first.span.start)?;
                match self.c.resolution.defs[def].kind {
                    // A tag's type comes only from its literals (10.15).
                    DefKind::Type if args.is_empty() && self.made_by_literals(def) => return None,
                    DefKind::Type if args.is_empty() => Gen::Adt(def),
                    DefKind::Builtin => match (first.name.as_str(), args.as_slice()) {
                        ("String", []) => Gen::Str,
                        ("Bytes", []) => Gen::Bytes,
                        ("Float", []) => Gen::Float,
                        ("Decimal", []) => Gen::Decimal,
                        ("Bool", []) => Gen::Bool,
                        ("Char", []) => Gen::Char,
                        ("List", [t]) => Gen::List(Box::new(self.gen_of(t)?)),
                        ("Map", [k, v]) => {
                            Gen::Map(Box::new(self.gen_of(k)?), Box::new(self.gen_of(v)?))
                        }
                        (name, []) => match Prim::from_name(name)? {
                            p if crate::ops::fits(0, p).is_some() && p != Prim::Decimal => {
                                Gen::Int(p)
                            }
                            _ => return None,
                        },
                        _ => return None,
                    },
                    _ => return None,
                }
            }
            TypeKind::List(t) => Gen::List(Box::new(self.gen_of(t)?)),
            TypeKind::Optional(t) => Gen::Optional(Box::new(self.gen_of(t)?)),
            TypeKind::Map(k, v) => Gen::Map(Box::new(self.gen_of(k)?), Box::new(self.gen_of(v)?)),
            TypeKind::Tuple(ts) => {
                Gen::Tuple(ts.iter().map(|t| self.gen_of(t)).collect::<Option<_>>()?)
            }
            _ => return None,
        })
    }

    /// A value of `g`, or `None` when no valid one was found (a record
    /// whose invariant every try broke).
    pub(crate) fn generate(
        &mut self,
        g: &Gen,
        pick: Pick,
        near: &[i128],
        rng: &mut Rng,
        depth: usize,
    ) -> Option<Value<'a>> {
        let edge = |n: usize| match pick {
            Pick::Edge(i) => Some(i % n.max(1)),
            Pick::Random => None,
        };
        Some(match g {
            Gen::Int(p) => {
                let (lo, hi) = int_range(*p);
                let mut edges = vec![0, 1, -1, lo, hi];
                edges.extend(near.iter().flat_map(|n| [n - 1, *n, n + 1]));
                edges.retain(|n| (lo..=hi).contains(n));
                let n = match edge(edges.len()) {
                    Some(i) => edges[i],
                    None => match rng.below(10) {
                        0..=4 => rng.range(lo.max(-100), hi.min(100)),
                        5..=7 => rng.range(lo.max(-1_000_000), hi.min(1_000_000)),
                        _ => rng.range(lo, hi),
                    },
                };
                Value::Int(n, *p)
            }
            Gen::Float => {
                const EDGES: [f64; 6] = [0.0, 1.0, -1.0, 0.5, 1e300, -1e-300];
                Value::Float(match edge(EDGES.len()) {
                    Some(i) => EDGES[i],
                    None => rng.range(-1_000_000, 1_000_000) as f64 / 100.0,
                })
            }
            Gen::Decimal => {
                let mut edges: Vec<String> =
                    ["0", "1", "-1", "0.01", "0.1", "123.456", "999999999999.99"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect();
                edges.extend(
                    near.iter()
                        .flat_map(|n| [n - 1, *n, n + 1])
                        .map(|n| n.to_string()),
                );
                let text = match edge(edges.len()) {
                    Some(i) => edges[i].clone(),
                    None => {
                        let units = rng.range(-1_000_000, 1_000_000);
                        format!("{}.{:02}", units, rng.below(100))
                    }
                };
                Value::Decimal(Decimal::parse(&text).unwrap_or(Decimal::from_int(0)))
            }
            Gen::Bool => Value::Bool(match edge(2) {
                Some(i) => i == 1,
                None => rng.below(2) == 1,
            }),
            Gen::Char => {
                const EDGES: [char; 4] = ['a', ' ', '0', 'न'];
                Value::Char(match edge(EDGES.len()) {
                    Some(i) => EDGES[i],
                    None => (b'a' + rng.below(26) as u8) as char,
                })
            }
            Gen::Str => {
                let text = match edge(TEXTS.len() + 1) {
                    Some(i) if i < TEXTS.len() => TEXTS[i].to_string(),
                    Some(_) => "x".repeat(1_000),
                    None => {
                        let len = rng.below(12);
                        (0..len)
                            .map(|_| match rng.below(10) {
                                0 => ' ',
                                1 => (b'0' + rng.below(10) as u8) as char,
                                2 => 'é',
                                _ => (b'a' + rng.below(26) as u8) as char,
                            })
                            .collect()
                    }
                };
                self.str_value(&text)
            }
            Gen::Bytes => {
                const EDGES: [&[u8]; 3] = [b"", &[13, 10, 13, 10], &[0]];
                let data = match edge(EDGES.len()) {
                    Some(i) => EDGES[i].to_vec(),
                    None => (0..rng.below(16)).map(|_| rng.below(256) as u8).collect(),
                };
                Value::Bytes(data.into())
            }
            Gen::Optional(inner) => {
                let none = match pick {
                    Pick::Edge(i) => i % 2 == 0,
                    Pick::Random => rng.below(3) == 0,
                };
                if none || depth >= MAX_DEPTH {
                    Value::None
                } else {
                    self.generate(inner, pick, near, rng, depth + 1)?
                }
            }
            Gen::List(item) => {
                let len = match pick {
                    _ if depth >= MAX_DEPTH => 0,
                    Pick::Edge(i) => [0, 1, 4, 2][i % 4],
                    Pick::Random => rng.below(8),
                };
                let mut items = Vec::new();
                for k in 0..len {
                    let p = match pick {
                        Pick::Edge(i) => Pick::Edge(i / 4 + k),
                        Pick::Random => Pick::Random,
                    };
                    items.push(self.generate(item, p, near, rng, depth + 1)?);
                }
                // The fourth edge case is a list with a duplicate.
                if matches!(pick, Pick::Edge(i) if i % 4 == 3) && !items.is_empty() {
                    items[1] = items[0].clone();
                }
                Value::List(Rc::new(items))
            }
            Gen::Map(k, v) => {
                let len = match pick {
                    _ if depth >= MAX_DEPTH => 0,
                    Pick::Edge(i) => [0, 1, 3][i % 3],
                    Pick::Random => rng.below(6),
                };
                let mut map = MapValue::default();
                for n in 0..len {
                    let p = match pick {
                        Pick::Edge(i) => Pick::Edge(i / 3 + n),
                        Pick::Random => Pick::Random,
                    };
                    let key = self.generate(k, p, near, rng, depth + 1)?;
                    map.insert(key, self.generate(v, p, near, rng, depth + 1)?);
                }
                Value::Map(Rc::new(map))
            }
            Gen::Tuple(gs) => {
                let mut items = Vec::new();
                for (n, g) in gs.iter().enumerate() {
                    let p = match pick {
                        Pick::Edge(i) => Pick::Edge(i + n),
                        Pick::Random => Pick::Random,
                    };
                    items.push(self.generate(g, p, near, rng, depth + 1)?);
                }
                Value::Tuple(items)
            }
            Gen::Adt(def) => return self.generate_adt(*def, pick, near, rng, depth),
        })
    }

    /// A record (never one that breaks an invariant) or a union variant.
    fn generate_adt(
        &mut self,
        def: DefId,
        pick: Pick,
        near: &[i128],
        rng: &mut Rng,
        depth: usize,
    ) -> Option<Value<'a>> {
        let decl = self
            .records
            .get(&def)
            .or_else(|| self.unions.get(&def))
            .copied()?;
        let name: Rc<str> = Rc::from(decl.name.name.as_str());
        if let Some(sample) = self.sampler(def) {
            return self.sampled(def, sample, rng);
        }
        match &decl.body {
            TypeBody::Record(r) => {
                for attempt in 0..20 {
                    let p = match (pick, attempt) {
                        (Pick::Edge(i), 0) => Pick::Edge(i),
                        _ => Pick::Random,
                    };
                    let fields = self.generate_fields(&r.fields, p, near, rng, depth)?;
                    let value = Value::Record {
                        def,
                        name: name.clone(),
                        fields,
                        life: None,
                    };
                    if self.check_invariants(&value, Span::new(0, 0)).is_ok() {
                        return Some(value);
                    }
                }
                None
            }
            TypeBody::Alias(_) => None,
            TypeBody::Union(vs) => {
                // Deep down, prefer a variant without fields.
                let i = match pick {
                    _ if depth >= MAX_DEPTH => {
                        vs.iter().position(|v| v.fields.is_empty()).unwrap_or(0)
                    }
                    Pick::Edge(i) => i % vs.len(),
                    Pick::Random => rng.below(vs.len()),
                };
                let v = &vs[i];
                let fields = self.generate_fields(&v.fields, pick, near, rng, depth)?;
                Some(Value::Variant {
                    union: def,
                    name: Rc::from(v.name.name.as_str()),
                    fields,
                })
            }
        }
    }

    /// The `sample` function of a type that implements `std.test`'s
    /// `Sample` (spec 11.3).
    fn sampler(&self, def: DefId) -> Option<&'a kavach_ast::FnDecl> {
        let res = &self.c.resolution;
        let decl = self.records.get(&def).or_else(|| self.unions.get(&def))?;
        let module = res.modules.iter().find(|m| m.name == "std.test")?;
        let is_sample = decl.implements.iter().any(|t| match &t.kind {
            TypeKind::Named { path, .. } => res
                .uses
                .get(&path.segments[0].span.start)
                .map(|d| &res.defs[*d])
                .is_some_and(|d| {
                    d.name == "Sample"
                        && d.kind == DefKind::Interface
                        && d.span
                            .is_some_and(|s| module.start <= s.start && s.start < module.end)
                }),
            _ => false,
        });
        if !is_sample {
            return None;
        }
        self.methods.get(&def)?.get("sample").copied()
    }

    /// A value from the type's own `sample`, called with a `Gen` seeded
    /// from the test run; one that breaks an invariant is not used.
    fn sampled(
        &mut self,
        def: DefId,
        sample: &'a kavach_ast::FnDecl,
        rng: &mut Rng,
    ) -> Option<Value<'a>> {
        let gen_def = self.c.resolution.builtin_type("std.test", "Gen")?;
        for _ in 0..20 {
            self.gen_rng = Rng::new(rng.next());
            let choices = Value::Record {
                def: gen_def,
                name: Rc::from("Gen"),
                fields: Vec::new(),
                life: None,
            };
            let made = self.call_fn(sample, None, vec![(None, choices)], sample.name.span);
            if let Ok(v) = made {
                let ours = matches!(&v, Value::Record { def: d, .. } | Value::Variant { union: d, .. } if *d == def);
                if ours && self.check_invariants(&v, Span::new(0, 0)).is_ok() {
                    return Some(v);
                }
            }
        }
        None
    }

    fn generate_fields(
        &mut self,
        fields: &'a [kavach_ast::Field],
        pick: Pick,
        near: &[i128],
        rng: &mut Rng,
        depth: usize,
    ) -> Option<Vec<Field<'a>>> {
        let mut out = Vec::new();
        for (n, f) in fields.iter().enumerate() {
            let g = self.gen_of(&f.ty)?;
            let p = match pick {
                Pick::Edge(i) => Pick::Edge(i + n),
                Pick::Random => Pick::Random,
            };
            out.push(Field {
                name: Rc::from(f.name.name.as_str()),
                value: self.generate(&g, p, near, rng, depth + 1)?,
                secret: f.is_secret,
            });
        }
        Some(out)
    }
}
