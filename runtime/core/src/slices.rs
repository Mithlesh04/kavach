// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Slices `xs[start:end:step]` and `#k` from the end (spec 2.19), worked
//! out the same way in the interpreter and in native code.

/// The panic of a step of 0.
pub const STEP_ZERO: &str = "the step of a slice can't be 0";

/// A slice as written: each part, when written, and whether an end was
/// written with `#` (counted from the end).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Part {
    pub start: Option<i64>,
    pub start_from_end: bool,
    pub end: Option<i64>,
    pub end_from_end: bool,
    pub step: Option<i64>,
}

/// The bits of the flags in `Part::words`.
const START: i64 = 1;
const START_FROM_END: i64 = 2;
const END: i64 = 4;
const END_FROM_END: i64 = 8;
const STEP: i64 = 16;

impl Part {
    /// The four numbers native code passes: start, end, step and flags.
    pub fn words(&self) -> [i64; 4] {
        let flags = Part::flags(
            self.start.is_some(),
            self.start_from_end,
            self.end.is_some(),
            self.end_from_end,
            self.step.is_some(),
        );
        [
            self.start.unwrap_or(0),
            self.end.unwrap_or(0),
            self.step.unwrap_or(1),
            flags,
        ]
    }

    /// The part `words` gave.
    pub fn from_words(w: [i64; 4]) -> Part {
        let has = |bit: i64| w[3] & bit != 0;
        Part {
            start: has(START).then_some(w[0]),
            start_from_end: has(START_FROM_END),
            end: has(END).then_some(w[1]),
            end_from_end: has(END_FROM_END),
            step: has(STEP).then_some(w[2]),
        }
    }

    /// The flags `words` gives for these ends, for code that builds the
    /// words itself.
    pub fn flags(
        start: bool,
        start_from_end: bool,
        end: bool,
        end_from_end: bool,
        step: bool,
    ) -> i64 {
        let mut flags = 0;
        for (on, bit) in [
            (start, START),
            (start_from_end, START_FROM_END),
            (end, END),
            (end_from_end, END_FROM_END),
            (step, STEP),
        ] {
            if on {
                flags |= bit;
            }
        }
        flags
    }

    /// `start`, `end` and `step` as plain numbers for `len` items, as
    /// Python's `slice.indices` gives them: `#k` is `len - k`, the
    /// missing parts are filled in, and both ends are clamped, to
    /// `0..=len`, or to `-1..=len - 1` for a negative step (spec 2.16).
    pub fn bounds(&self, len: usize) -> Result<(i64, i64, i64), &'static str> {
        let step = self.step.unwrap_or(1);
        if step == 0 {
            return Err(STEP_ZERO);
        }
        let n = len as i64;
        let at = |v: i64, from_end: bool| if from_end { n.saturating_sub(v) } else { v };
        let start = self.start.map(|v| at(v, self.start_from_end));
        let end = self.end.map(|v| at(v, self.end_from_end));
        Ok(if step > 0 {
            (
                start.unwrap_or(0).clamp(0, n),
                end.unwrap_or(n).clamp(0, n),
                step,
            )
        } else {
            (
                start.unwrap_or(n - 1).clamp(-1, n - 1),
                end.unwrap_or(-1).clamp(-1, n - 1),
                step,
            )
        })
    }

    /// The positions the slice holds, in order, for `len` items.
    pub fn positions(&self, len: usize) -> Result<Vec<usize>, &'static str> {
        let (start, end, step) = self.bounds(len)?;
        let mut out = Vec::new();
        let mut i = start;
        while (step > 0 && i < end) || (step < 0 && i > end) {
            out.push(i as usize);
            i = match i.checked_add(step) {
                Some(next) => next,
                None => break,
            };
        }
        Ok(out)
    }

    /// The positions `xs[part] = values` sets, when `given` values are
    /// given: a slice is set only with as many (spec 2.19).
    pub fn set_positions(&self, len: usize, given: usize) -> Result<Vec<usize>, String> {
        let at = self.positions(len)?;
        if at.len() != given {
            return Err(set_length_panic(at.len(), given));
        }
        Ok(at)
    }
}

/// The panic of a slice set with another number of items.
pub fn set_length_panic(slice: usize, given: usize) -> String {
    let items = if slice == 1 { "item" } else { "items" };
    let were = if given == 1 { "was" } else { "were" };
    format!("the slice has {slice} {items}, but {given} {were} given")
}

/// The position of `#k` in `len` items: `len - k`.
pub fn from_end(len: usize, k: i64) -> i128 {
    len as i128 - i128::from(k)
}

/// The panic of character `i` out of range of a text of `n` characters.
pub fn text_index_panic(i: i128, n: usize) -> String {
    let chars = if n == 1 { "character" } else { "characters" };
    format!("index {i} is out of range: the text has {n} {chars}")
}

/// The characters of `s` at the slice's positions.
pub fn text(s: &str, part: &Part) -> Result<String, &'static str> {
    if part.step.unwrap_or(1) == 1 {
        // The common case, without a list of positions.
        let n = s.chars().count();
        let (start, end, _) = part.bounds(n)?;
        let take = (end - start).max(0) as usize;
        return Ok(s.chars().skip(start as usize).take(take).collect());
    }
    let chars: Vec<char> = s.chars().collect();
    Ok(part
        .positions(chars.len())?
        .into_iter()
        .map(|i| chars[i])
        .collect())
}

/// `s[start:end]` with no step and no `#`, borrowed from `s`: both ends
/// clamped to the text, as `text` clamps them. ASCII text is cut by
/// bytes, with no walk over its characters.
pub fn text_range(s: &str, start: i64, end: i64) -> &str {
    if s.is_ascii() {
        let n = s.len() as i64;
        let (a, b) = (start.clamp(0, n), end.clamp(0, n));
        return &s[a as usize..b.max(a) as usize];
    }
    let n = s.chars().count() as i64;
    let a = start.clamp(0, n);
    let b = end.clamp(0, n).max(a);
    let at = |k: i64| s.char_indices().nth(k as usize).map_or(s.len(), |(i, _)| i);
    &s[at(a)..at(b)]
}

/// Character `i` of `s`, or the panic when it is out of range.
pub fn char_at(s: &str, i: i128) -> Result<char, String> {
    let found = usize::try_from(i).ok().and_then(|i| s.chars().nth(i));
    found.ok_or_else(|| text_index_panic(i, s.chars().count()))
}

/// The items of `items` at the slice's positions.
pub fn pick<T: Clone>(items: &[T], part: &Part) -> Result<Vec<T>, &'static str> {
    if part.step.unwrap_or(1) == 1 {
        let (start, end, _) = part.bounds(items.len())?;
        let end = end.max(start);
        return Ok(items[start as usize..end as usize].to_vec());
    }
    Ok(part
        .positions(items.len())?
        .into_iter()
        .map(|i| items[i].clone())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `text_range` cuts as `text` does, ASCII or not, past the ends too.
    #[test]
    fn a_range_cuts_as_a_part_does() {
        for s in ["", "abc", "héllo wörld", "日本語テキスト"] {
            for (a, b) in [
                (0, 0),
                (1, 2),
                (0, i64::MAX),
                (2, 1),
                (3, 99),
                (-5, 2),
                (0, -1),
            ] {
                let p = part(Some(a), Some(b), None);
                assert_eq!(text_range(s, a, b), text(s, &p).unwrap(), "{s} {a} {b}");
            }
        }
    }

    fn part(start: Option<i64>, end: Option<i64>, step: Option<i64>) -> Part {
        Part {
            start,
            end,
            step,
            ..Part::default()
        }
    }

    #[test]
    fn slices_as_python_with_counted_ends() {
        let xs = [10, 20, 30, 40, 50];
        let from_end = |k| Part {
            start: Some(k),
            start_from_end: true,
            ..Part::default()
        };
        let cases: &[(Part, &[i32])] = &[
            (part(Some(1), Some(3), None), &[20, 30]),
            (part(None, None, Some(2)), &[10, 30, 50]),
            (part(None, None, Some(-1)), &[50, 40, 30, 20, 10]),
            (part(Some(3), Some(0), Some(-1)), &[40, 30, 20]),
            (part(None, None, Some(-2)), &[50, 30, 10]),
            (part(Some(1), Some(99), None), &[20, 30, 40, 50]),
            (part(Some(9), None, None), &[]),
            (part(Some(2), Some(1), None), &[]),
            (part(Some(-2), None, None), &[10, 20, 30, 40, 50]),
            (part(None, Some(-1), None), &[]),
            (from_end(2), &[40, 50]),
            (from_end(0), &[]),
        ];
        for (p, want) in cases {
            assert_eq!(pick(&xs, p).unwrap(), *want, "{p:?}");
            assert_eq!(Part::from_words(p.words()), *p);
        }
        assert_eq!(pick(&xs, &part(None, None, Some(0))), Err(STEP_ZERO));
    }

    #[test]
    fn text_by_characters() {
        assert_eq!(text("héllo", &part(Some(1), Some(3), None)).unwrap(), "él");
        assert_eq!(text("héllo", &part(None, None, Some(-1))).unwrap(), "olléh");
        assert_eq!(char_at("héllo", 1), Ok('é'));
        assert_eq!(
            char_at("héllo", 9).unwrap_err(),
            "index 9 is out of range: the text has 5 characters"
        );
    }

    #[test]
    fn setting_needs_as_many() {
        let p = part(Some(0), Some(2), None);
        assert_eq!(p.set_positions(5, 2).unwrap(), vec![0, 1]);
        assert_eq!(
            p.set_positions(5, 1).unwrap_err(),
            "the slice has 2 items, but 1 was given"
        );
    }
}
