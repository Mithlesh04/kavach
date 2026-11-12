//! The fakes of `Fs` and `Clock` in generated tests (spec 11.5): a case
//! never touches the real world. Files live in memory, on a disk that is
//! full for some paths, and the clock
//! moves only when the case sleeps (the network is `fake_net.rs`). What a case finds is decided from its arguments, so the same
//! case finds the same world again (when shrinking, or replaying found
//! cases).

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_checker::Prim;
use std::collections::BTreeMap;
use std::rc::Rc;

/// 2026-01-01 00:00 UTC, when every case starts.
const START_MS: i128 = 1_767_225_600_000;

/// What a file the case didn't write holds, when it is there.
const CONTENTS: [&str; 5] = ["", "hello\nworld\n", "नमस्ते", "{\"a\": 1}", "1,2,3\n"];

/// A path as the case sees it.
#[derive(Clone, Debug)]
enum Entry {
    File(Vec<u8>),
    Folder,
    Missing,
}

#[derive(Debug)]
pub(crate) struct World {
    seed: u64,
    paths: BTreeMap<String, Entry>,
    now_ms: i128,
    /// The case's clipboard (spec 14.17): empty at the start.
    pub(crate) clipboard: Option<String>,
}

/// `./a/b/` and `a/b` are the same path.
fn clean(path: &str) -> String {
    let p = path.trim_start_matches("./").trim_end_matches('/');
    p.to_string()
}

fn parent(path: &str) -> &str {
    path.rfind('/').map_or("", |i| &path[..i])
}

impl World {
    /// What the case's world is decided from.
    pub(crate) fn seed(&self) -> u64 {
        self.seed
    }

    pub(crate) fn new(seed: u64) -> World {
        World {
            seed,
            paths: BTreeMap::new(),
            now_ms: START_MS,
            clipboard: None,
        }
    }

    /// Whether writing to `path` fails as on a full disk: decided from
    /// the path, so the case finds the same again (spec 11.5).
    fn full_at(&self, path: &str) -> bool {
        let h = path.bytes().fold(self.seed ^ 0xD15C_F011, |h, b| {
            h.wrapping_mul(0x100_0000_01B3).wrapping_add(u64::from(b))
        });
        (h >> 7) % 6 == 0
    }

    /// What `path` is: decided the first time the case reaches it.
    fn entry(&mut self, path: &str) -> Entry {
        if let Some(e) = self.paths.get(path) {
            return e.clone();
        }
        let has_children = self.paths.keys().any(|k| parent(k) == path);
        let h = path.bytes().fold(self.seed, |h, b| {
            h.wrapping_mul(0x100_0000_01B3).wrapping_add(u64::from(b))
        });
        let e = match h % 3 {
            _ if has_children => Entry::Folder,
            0 => Entry::Missing,
            n => Entry::File(CONTENTS[(h / 3 + n) as usize % CONTENTS.len()].into()),
        };
        self.paths.insert(path.to_string(), e.clone());
        e
    }
}

fn not_found() -> std::io::Error {
    std::io::Error::from(std::io::ErrorKind::NotFound)
}

impl<'a> Interp<'a> {
    /// A `std.fs` call in a generated test, on the case's files in memory.
    pub(crate) fn fake_fs(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        let text = |i: usize| match args.get(i) {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let shown = text(0);
        let path = clean(&shown);
        let Some(world) = self.fake.clone() else {
            return Ok(Value::Unit);
        };
        let mut w = world.borrow_mut();
        let fail = |l: &Self, e: std::io::Error| Err(l.fs_error(&shown, e));
        let is_dir = || std::io::Error::other("is a folder");
        Ok(match name {
            "read_text" | "read_bytes" | "lines" | "chunks" => {
                let data = match w.entry(&path) {
                    Entry::File(d) => d,
                    Entry::Folder => return fail(self, is_dir()),
                    Entry::Missing => return fail(self, not_found()),
                };
                drop(w);
                match name {
                    "read_bytes" => Value::Bytes(data.into()),
                    "read_text" => match String::from_utf8(data) {
                        Ok(s) => self.str_value(&s),
                        Err(_) => return fail(self, std::io::Error::other("not UTF-8")),
                    },
                    _ => {
                        let chunk = match args.get(1) {
                            Some(Value::Int(n, _)) if name == "chunks" => {
                                Some((*n).max(1) as usize)
                            }
                            _ => None,
                        };
                        return Ok(self.reader_of(data, &shown, chunk));
                    }
                }
            }
            "write_text" | "write_bytes" | "write_durable" | "append_text" => {
                if w.full_at(&path) {
                    return fail(self, std::io::Error::other("no space left on device"));
                }
                let mut data = match args.get(1) {
                    Some(Value::Bytes(b)) => b.to_vec(),
                    _ => text(1).into_bytes(),
                };
                if name == "append_text" {
                    match w.entry(&path) {
                        Entry::File(mut before) => {
                            before.extend(data);
                            data = before;
                        }
                        Entry::Folder => return fail(self, is_dir()),
                        Entry::Missing => {}
                    }
                } else if let Entry::Folder = w.entry(&path) {
                    return fail(self, is_dir());
                }
                w.paths.insert(path, Entry::File(data));
                Value::Unit
            }
            "exists" => Value::Bool(!matches!(w.entry(&path), Entry::Missing)),
            "list" => {
                match w.entry(&path) {
                    Entry::Folder => {}
                    Entry::File(_) => return fail(self, std::io::Error::other("not a folder")),
                    Entry::Missing => return fail(self, not_found()),
                }
                let names: Vec<String> = w
                    .paths
                    .iter()
                    .filter(|(k, e)| parent(k) == path && !matches!(e, Entry::Missing))
                    .map(|(k, _)| k.rsplit('/').next().unwrap_or(k).to_string())
                    .collect();
                drop(w);
                Value::List(Rc::new(names.iter().map(|n| self.str_value(n)).collect()))
            }
            "remove" => {
                if let Entry::Missing = w.entry(&path) {
                    return fail(self, not_found());
                }
                if w.paths.keys().any(|k| parent(k) == path) {
                    return fail(self, std::io::Error::other("folder not empty"));
                }
                w.paths.insert(path, Entry::Missing);
                Value::Unit
            }
            "create_dir" => {
                let mut at = path.as_str();
                while !at.is_empty() {
                    if let Entry::File(_) = w.entry(at) {
                        return fail(self, std::io::Error::other("a file is in the way"));
                    }
                    w.paths.insert(at.to_string(), Entry::Folder);
                    at = parent(at);
                }
                Value::Unit
            }
            // A file's details (`fs.info`): the case's files have no
            // permissions of their own, and never change by the clock.
            "stat" => {
                let (kind, size, mode) = match w.entry(&path) {
                    Entry::File(d) => (0, d.len() as i128, 0o644),
                    Entry::Folder => (1, 0, 0o755),
                    Entry::Missing => return fail(self, not_found()),
                };
                let n = [kind, size, 0, 0, mode];
                Value::List(Rc::new(
                    n.iter().map(|v| Value::Int(*v, Prim::Int)).collect(),
                ))
            }
            "set_read_only" | "set_permissions" => match w.entry(&path) {
                Entry::Missing => return fail(self, not_found()),
                _ => Value::Unit,
            },
            "rename" | "copy" | "remove_all" => {
                if let Entry::Missing = w.entry(&path) {
                    return fail(self, not_found());
                }
                let to = clean(&text(1));
                // The path and everything under it.
                let under: Vec<(String, Entry)> = (w.paths.iter())
                    .filter(|(k, _)| *k == &path || k.starts_with(&format!("{path}/")))
                    .map(|(k, e)| (k.clone(), e.clone()))
                    .collect();
                if name == "copy" && !matches!(w.entry(&path), Entry::File(_)) {
                    return fail(
                        self,
                        std::io::Error::other("a folder: copy its files one by one"),
                    );
                }
                for (k, e) in under {
                    if name != "copy" {
                        w.paths.insert(k.clone(), Entry::Missing);
                    }
                    if name != "remove_all" {
                        w.paths.insert(format!("{to}{}", &k[path.len()..]), e);
                    }
                }
                Value::Unit
            }
            _ => {
                return Err(self.panic(
                    format!("std.fs has no function `{name}`"),
                    kavach_ast::Span::new(0, 0),
                ));
            }
        })
    }

    /// The case's clock, in milliseconds since 1970, moved on by `ms`.
    pub(crate) fn fake_clock(&mut self, ms: i128) -> Option<Value<'a>> {
        let world = self.fake.clone()?;
        let mut w = world.borrow_mut();
        w.now_ms += ms.max(0);
        Some(Value::Int(w.now_ms, Prim::Int))
    }

    /// The case's monotonic clock: milliseconds since the case began.
    pub(crate) fn fake_monotonic(&self) -> Option<Value<'a>> {
        let w = self.fake.as_ref()?.borrow();
        Some(Value::Int(w.now_ms - START_MS, Prim::Int))
    }
}
