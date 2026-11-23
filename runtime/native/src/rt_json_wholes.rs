// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Whole numbers with their own limits read from JSON (spec 2.1, 2.12):
//! `UInt<bits: 12>`, `Int<range: 1..=65535>`. Their values are `Int`s,
//! so the type alone doesn't say their limits: `json.decode` is given a
//! *plan* of where they are in the type it decodes
//! (`compiler/kir/src/lower_json_wholes.rs`), and the decoders walk it
//! beside the type. Node 0 stands for "nothing of the kind below here";
//! node 1 is the decoded type's own.
//!
//! The plan is a list of numbers, one node after another:
//! `0` any; `1 lo hi bits signed` a whole number (`bits` 0 for a range);
//! `2 x` a list's, set's or optional's item; `3 k v` a dict's key and
//! value; `4 n x…` a tuple's items; `5 n x…` a record's fields, in the
//! order of its shape; `6 n` then, for each of `n` variants, `m x…`.

use std::cell::RefCell;
use std::rc::Rc;

/// One part of a type, as the plan describes it.
#[derive(Debug, PartialEq)]
enum Node {
    Any,
    Whole {
        lo: i64,
        hi: i64,
        bits: i64,
        signed: bool,
    },
    /// Items, keys and values, or fields, by their place.
    Parts(Vec<usize>),
    /// Each variant's fields.
    Variants(Vec<Vec<usize>>),
}

thread_local! {
    /// The plan of the `json.decode` running now, if it holds any.
    static PLAN: RefCell<Option<Rc<Vec<Node>>>> = const { RefCell::new(None) };
}

/// Where the plan starts: the decoded type itself.
pub(crate) const ROOT: usize = 1;

/// Reads plan `words`; `None` when it is empty or broken.
fn parse(words: &[i64]) -> Option<Vec<Node>> {
    let mut nodes = Vec::new();
    let mut at = 0;
    let mut next = || {
        let w = *words.get(at)?;
        at += 1;
        Some(w)
    };
    let index = |w: i64| usize::try_from(w).ok();
    while let Some(kind) = next() {
        nodes.push(match kind {
            0 => Node::Any,
            1 => Node::Whole {
                lo: next()?,
                hi: next()?,
                bits: next()?,
                signed: next()? != 0,
            },
            2 => Node::Parts(vec![index(next()?)?]),
            3 => Node::Parts(vec![index(next()?)?, index(next()?)?]),
            4 | 5 => {
                let n = next()?;
                let parts = (0..n).map(|_| index(next()?)).collect::<Option<_>>()?;
                Node::Parts(parts)
            }
            6 => {
                let n = next()?;
                let mut variants = Vec::new();
                for _ in 0..n {
                    let m = next()?;
                    variants.push((0..m).map(|_| index(next()?)).collect::<Option<_>>()?);
                }
                Node::Variants(variants)
            }
            _ => return None,
        });
    }
    (nodes.len() > ROOT).then_some(nodes)
}

/// Starts a decode with plan `words` (empty: none).
pub(crate) fn begin(words: &[i64]) {
    let plan = parse(words).map(Rc::new);
    PLAN.with(|p| *p.borrow_mut() = plan);
}

/// Ends what `begin` started.
pub(crate) fn end() {
    PLAN.with(|p| *p.borrow_mut() = None);
}

/// Where to start: `ROOT` while a plan is in force, else nothing.
pub(crate) fn root() -> usize {
    PLAN.with(|p| if p.borrow().is_some() { ROOT } else { 0 })
}

fn with_node<R>(n: usize, f: impl FnOnce(&Node) -> R) -> Option<R> {
    if n == 0 {
        return None;
    }
    PLAN.with(|p| p.borrow().as_ref().and_then(|nodes| nodes.get(n).map(f)))
}

/// Part `i` of node `n`: an item, key or value, or field.
pub(crate) fn part(n: usize, i: usize) -> usize {
    with_node(n, |node| match node {
        Node::Parts(parts) => parts.get(i).copied().unwrap_or(0),
        _ => 0,
    })
    .unwrap_or(0)
}

/// The fields of variant `v` of node `n`.
pub(crate) fn variant(n: usize, v: usize) -> Vec<usize> {
    with_node(n, |node| match node {
        Node::Variants(all) => all.get(v).cloned().unwrap_or_default(),
        _ => Vec::new(),
    })
    .unwrap_or_default()
}

/// Each field's node of record node `n`, `count` fields in all: none
/// (each 0) for node 0, so a plain decode makes no list.
pub(crate) fn fields(n: usize, count: usize) -> Vec<usize> {
    if n == 0 {
        return Vec::new();
    }
    (0..count).map(|i| part(n, i)).collect()
}

/// The name of node `n`'s whole number when it is one with its own
/// limits, and whether `value` is within them.
pub(crate) fn whole(n: usize, value: i64) -> Option<(String, bool)> {
    with_node(n, |node| match *node {
        Node::Whole {
            lo,
            hi,
            bits,
            signed,
        } => {
            let name = match (bits, signed) {
                (0, _) => format!("Int<range: {lo}..={hi}>"),
                (b, true) => format!("Int<bits: {b}>"),
                (b, false) => format!("UInt<bits: {b}>"),
            };
            Some((name, (lo..=hi).contains(&value)))
        }
        _ => None,
    })
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_read_back() {
        // `[UInt<bits: 12>]`: a list of node 2, a whole number.
        let plan = parse(&[0, 2, 2, 1, 0, 4095, 12, 0]).unwrap();
        assert_eq!(plan[1], Node::Parts(vec![2]));
        begin(&[0, 2, 2, 1, 0, 4095, 12, 0]);
        assert_eq!(part(ROOT, 0), 2);
        assert_eq!(whole(2, 4095), Some(("UInt<bits: 12>".into(), true)));
        assert_eq!(whole(2, 4096), Some(("UInt<bits: 12>".into(), false)));
        assert_eq!(whole(ROOT, 1), None);
        end();
        assert_eq!(part(ROOT, 0), 0);
        // Broken or empty plans are none.
        assert!(parse(&[0, 2]).is_none());
        assert!(parse(&[]).is_none());
        assert!(parse(&[0, 6, 1, 2, 5]).is_none());
    }
}
