//! The parser's loop (spec 2.12): it keeps its own stack instead of
//! recursing, with the limits and errors of the spec.

use super::read::Parser;
use super::{Build, Error, Limits, MAX_DEPTH, SCAN};
use std::borrow::Cow;
use std::collections::HashSet;

/// A container being filled: where its items, or its fields and their
/// names, start on the shared stacks; an object's key waiting for its
/// value, and its keys in a set once it is large.
enum Open<'a, K> {
    Array(usize),
    Object(usize, usize, Option<K>, Option<Box<Seen<'a>>>),
}

/// A large object's keys, in a set: by their numbers (`Build::key_id`) or
/// their text.
pub(super) enum Seen<'a> {
    Ids(HashSet<u64>),
    Names(HashSet<Cow<'a, str>>),
}

/// The values of every open container, one stack for all, so a container
/// costs no allocation of its own; and each field's key text, for
/// finding duplicates.
struct Stacks<'a, B: Build> {
    items: Vec<B::Value>,
    fields: Vec<(B::Key, B::Value)>,
    names: Vec<Cow<'a, str>>,
    open: Vec<Open<'a, B::Key>>,
}

/// The value written in `text`, made by `b`.
pub fn parse_with<B: Build>(text: &str, limits: &Limits, b: &mut B) -> Result<B::Value, Error> {
    if text.len() > limits.max_bytes {
        return Err(Error::TooLarge(limits.max_bytes));
    }
    let mut p = Parser {
        s: text.as_bytes(),
        text,
        at: 0,
        comments: limits.comments,
    };
    p.space();
    value_at(&mut p, limits, 0, true, b)
}

/// The value at `p`, inside `depth` open containers. With `whole`, it
/// must be all that is left of the text.
pub(super) fn value_at<'a, B: Build>(
    p: &mut Parser<'a>,
    limits: &Limits,
    depth: usize,
    whole: bool,
    b: &mut B,
) -> Result<B::Value, Error> {
    let mut st = Stacks::<B> {
        items: Vec::new(),
        fields: Vec::new(),
        names: Vec::new(),
        open: Vec::new(),
    };
    let result = run(p, limits, depth, whole, b, &mut st);
    if result.is_err() {
        st.items.drain(..).for_each(|v| b.discard(v));
        for (k, v) in st.fields.drain(..) {
            b.discard_key(k);
            b.discard(v);
        }
        for o in st.open.drain(..) {
            if let Open::Object(_, _, Some(k), _) = o {
                b.discard_key(k);
            }
        }
    }
    result
}

fn run<'a, B: Build>(
    p: &mut Parser<'a>,
    limits: &Limits,
    depth: usize,
    whole: bool,
    b: &mut B,
    st: &mut Stacks<'a, B>,
) -> Result<B::Value, Error> {
    let max_depth = limits.max_depth.min(MAX_DEPTH);
    loop {
        // A value: a scalar, or the start of a container.
        let value = match p.peek() {
            Some(b'[') | Some(b'{') => {
                if depth + st.open.len() >= max_depth {
                    return Err(Error::TooDeep(max_depth));
                }
                let array = p.s[p.at] == b'[';
                p.at += 1;
                p.space();
                let close = if array { b']' } else { b'}' };
                if p.peek() == Some(close) {
                    p.at += 1;
                    if array {
                        b.array(Vec::new().drain(..))
                    } else {
                        b.object(Vec::new().drain(..))
                    }
                } else if array {
                    st.open.push(Open::Array(st.items.len()));
                    continue;
                } else {
                    let name = p.key()?;
                    let key = b.key(&name);
                    let starts = (st.fields.len(), st.names.len());
                    if b.key_id(&key).is_none() {
                        st.names.push(name);
                    }
                    st.open
                        .push(Open::Object(starts.0, starts.1, Some(key), None));
                    continue;
                }
            }
            Some(b'"') => {
                let s = p.string()?;
                b.text(&s)
            }
            _ => p.scalar(b)?,
        };
        // Put it in its container; close every container that ends here.
        let mut value = value;
        loop {
            let Some(top) = st.open.last_mut() else {
                if !whole {
                    return Ok(value);
                }
                p.space();
                if p.at < p.s.len() {
                    b.discard(value);
                    return Err(p.syntax("unexpected text after the value"));
                }
                return Ok(value);
            };
            p.space();
            match top {
                Open::Array(_) => {
                    st.items.push(value);
                    match p.next_byte() {
                        Some(b',') => {
                            p.space();
                            // JSONC: `[1, 2,]`.
                            if !(p.comments && p.peek() == Some(b']')) {
                                break;
                            }
                            p.at += 1;
                        }
                        Some(b']') => {}
                        _ => return Err(p.syntax_before("expected `,` or `]` in an array")),
                    }
                }
                Open::Object(start, names, key, seen) => {
                    let k = key
                        .take()
                        .expect("an object waiting for a value has its key");
                    st.fields.push((k, value));
                    let (start, names) = (*start, *names);
                    if st.fields.len() - start > limits.max_fields {
                        return Err(Error::TooLarge(limits.max_fields));
                    }
                    match p.next_byte() {
                        Some(b',') if p.comments && p.after_space() == Some(b'}') => {
                            // JSONC: `{"a": 1,}`.
                            p.space();
                            p.at += 1;
                        }
                        Some(b',') => {
                            p.space();
                            let at = p.at;
                            let name = p.key()?;
                            let k = b.key(&name);
                            let id = b.key_id(&k);
                            let twice = match id {
                                Some(id) => duplicate_id(b, &st.fields[start..], seen, id),
                                None => duplicate(&st.names[names..], seen, &name),
                            };
                            if twice {
                                b.discard_key(k);
                                let (line, column) = p.place(at);
                                return Err(Error::DuplicateKey {
                                    key: name.into_owned(),
                                    line,
                                    column,
                                });
                            }
                            *key = Some(k);
                            if id.is_none() {
                                st.names.push(name);
                            }
                            break;
                        }
                        Some(b'}') => {}
                        _ => return Err(p.syntax_before("expected `,` or `}` in an object")),
                    }
                }
            }
            value = match st.open.pop() {
                Some(Open::Array(start)) => b.array(st.items.drain(start..)),
                Some(Open::Object(start, names, ..)) => {
                    st.names.truncate(names);
                    b.object(st.fields.drain(start..))
                }
                None => unreachable!("a container was open"),
            };
        }
    }
}

/// Whether `key` is already one of `names` (an object's keys so far): a
/// scan while the object is small, then a set of its keys.
pub(super) fn duplicate<'a>(
    names: &[Cow<'a, str>],
    seen: &mut Option<Box<Seen<'a>>>,
    key: &str,
) -> bool {
    if names.len() < SCAN {
        return names.iter().any(|k| k == key);
    }
    let set = seen.get_or_insert_with(|| Box::new(Seen::Names(names.iter().cloned().collect())));
    match &mut **set {
        Seen::Names(set) => !set.insert(Cow::Owned(key.to_string())),
        Seen::Ids(_) => false,
    }
}

/// `duplicate`, for a builder that gives keys numbers: `fields` are the
/// object's fields so far.
fn duplicate_id<B: Build>(
    b: &B,
    fields: &[(B::Key, B::Value)],
    seen: &mut Option<Box<Seen<'_>>>,
    id: u64,
) -> bool {
    let of = |k: &B::Key| b.key_id(k).unwrap_or(0);
    if fields.len() < SCAN {
        return fields.iter().any(|(k, _)| of(k) == id);
    }
    let set = seen
        .get_or_insert_with(|| Box::new(Seen::Ids(fields.iter().map(|(k, _)| of(k)).collect())));
    match &mut **set {
        Seen::Ids(set) => !set.insert(id),
        Seen::Names(_) => false,
    }
}
