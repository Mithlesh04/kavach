// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `Int<bits: n>`, `UInt<bits: n>` and `Int<range: a..=b>` (spec 2.1):
//! what is written right passes, and each mistake gets its message.

use kavach_checker::{Options, check};

/// The errors from parsing, resolving and checking `src`, as "line: message".
fn errors(src: &str) -> Vec<String> {
    let parsed = kavach_parser::parse(src);
    let resolved = kavach_resolver::resolve(&parsed.file);
    let checked = check(&parsed.file, &resolved, Options::default());
    let line = |start: u32| src[..start as usize].matches('\n').count() + 1;
    let syntax = parsed.errors.iter().map(|e| (e.span.start, &e.message));
    let later = (resolved.errors.iter())
        .chain(&checked.errors)
        .map(|e| (e.span.start, &e.message));
    syntax
        .chain(later)
        .map(|(start, message)| format!("{}: {message}", line(start)))
        .collect()
}

#[test]
fn widths_and_ranges_are_checked_where_they_are_written() {
    let cases: &[(&str, &[&str])] = &[
        (
            "fn f(a: UInt<bits: 12>, p: Int<range: -5..=5>) -> Int {\n  Int(a) + Int(p)\n}",
            &[],
        ),
        (
            "fn f(p: Int<range: 10..=20>) -> Int<range: 10..=20> {\n  p - 1\n}",
            &[],
        ),
        (
            "fn f(n: Int) -> Int<range: 0..10> {\n  Int<range: 0..10>(n) + 0\n}",
            &[],
        ),
        (
            "fn f() -> UInt<bits: 12> {\n  4096\n}",
            &["2: `4096` doesn't fit in `UInt<bits: 12>`, which holds 0 to 4095: use `UInt16`"],
        ),
        (
            "fn f() -> Int<range: 1..=65535> {\n  0\n}",
            &["2: `0` is not in `Int<range: 1..=65535>`, which holds 1 to 65535 (spec 2.1)"],
        ),
        (
            "fn f() -> Int<bits: 0> {\n  0\n}",
            &["1: a width is from 1 to 65,535 bits, not 0 (spec 2.1)"],
        ),
        (
            "fn f() -> UInt<bits: 128> {\n  -1\n}",
            &[
                "2: `-1` doesn't fit in `UInt<bits: 128>`, which holds 0 to 340282366920938463463374607431768211455: use `Int8`",
            ],
        ),
        (
            "fn f() -> Int<range: 5..5> {\n  5\n}",
            &["1: the range `5..5` is empty: a value could never be in it (spec 2.1)"],
        ),
        (
            "fn f() -> UInt {\n  5\n}",
            &["1: `UInt` is written with its width: `UInt<bits: 32>`, or `UInt32` (spec 2.1)"],
        ),
        (
            "fn f() -> UInt<range: 1..=9> {\n  5\n}",
            &["1: a range is written on `Int`: `Int<range: 0..=4095>` (spec 2.1)"],
        ),
        (
            "fn f() -> Int<12> {\n  5\n}",
            &["1: a number in a type has a name: write `Int<bits: 12>` (spec 2.9)"],
        ),
        (
            "fn f() -> Int<width: 12> {\n  5\n}",
            &["1: `Int` has no number `width`: it takes `bits` or `range` (spec 2.1)"],
        ),
        (
            "fn f(p: Int<range: 1..=9>) -> Bool {\n  p & 1 == 1\n}",
            &[
                "2: `&` works on bits, and `Int<range: 1..=9>` is a range: convert it first, as `Int(x) & …`, or use an `Int<bits: n>` (spec 2.1)",
            ],
        ),
        (
            "fn f(p: Int<range: 1..=9>) -> Int<range: 1..=9> {\n  p.wrapping_add(1)\n}",
            &[
                "2: `Int<range: 1..=9>` has no method `wrapping_add`: wrapping has no meaning for a range: use `checked_add` and the others, which give `none` outside it",
            ],
        ),
        (
            "fn f(a: Int<bits: 12>, b: Int) -> Int<bits: 12> {\n  a + b\n}",
            &[
                "2: there are no implicit number conversions: expected `Int<bits: 12>`, found `Int`; convert with `Int<bits: 12>(…)`",
            ],
        ),
    ];
    let failures: Vec<String> = (cases.iter())
        .filter_map(|(src, want)| {
            let got = errors(src);
            (got != *want).then(|| format!("source: {src:?}\n  want: {want:?}\n   got: {got:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
