// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The kernel frame every kernel starts with: its buffers, checked
//! arithmetic, conversions and the stop flag (WGSL text); and scalar
//! types as WGSL types and as the `u32` words of a buffer.

use kavach_kir::Type;

/// Checked arithmetic and the stop flag, shared by every kernel.
pub(crate) const PRELUDE: &str = "
@group(0) @binding(0) var<storage, read> items: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@group(0) @binding(2) var<storage, read> info: array<u32>;
@group(0) @binding(3) var<storage, read_write> stop: array<atomic<u32>, 5>;
var<private> halted: bool;

fn halt(code: u32, a: u32, b: u32, line: u32) {
    halted = true;
    if (atomicExchange(&stop[0], 1u) == 0u) {
        atomicStore(&stop[1], code);
        atomicStore(&stop[2], a);
        atomicStore(&stop[3], b);
        atomicStore(&stop[4], line);
    }
}
fn add_i(a: i32, b: i32, line: u32) -> i32 { let r = a + b; if (((a ^ r) & (b ^ r)) < 0) { halt(11u, bitcast<u32>(a), bitcast<u32>(b), line); } return r; }
fn sub_i(a: i32, b: i32, line: u32) -> i32 { let r = a - b; if (((a ^ b) & (a ^ r)) < 0) { halt(21u, bitcast<u32>(a), bitcast<u32>(b), line); } return r; }
fn mul_i(a: i32, b: i32, line: u32) -> i32 {
    let r = a * b;
    // Two numbers of 16 bits can't overflow: no division for those.
    if (((bitcast<u32>(a) + 0x8000u) | (bitcast<u32>(b) + 0x8000u)) < 0x10000u) { return r; }
    let m = i32(-2147483647) - 1;
    if ((a == -1 && b == m) || (b == -1 && a == m) || (a != 0 && a != -1 && r / a != b)) { halt(31u, bitcast<u32>(a), bitcast<u32>(b), line); }
    return r;
}
fn div_i(a: i32, b: i32, line: u32) -> i32 {
    if (b == 0) { halt(20u, 0u, 0u, line); return 0; }
    if (b == -1 && a == i32(-2147483647) - 1) { halt(41u, bitcast<u32>(a), bitcast<u32>(b), line); return a; }
    return div_k(a, b);
}
fn rem_i(a: i32, b: i32, line: u32) -> i32 {
    if (b == 0) { halt(20u, 0u, 0u, line); return 0; }
    return rem_k(a, b);
}
// Through unsigned numbers: a signed `/` and `%` take the GPU about a third
// longer, and the smallest number's size is right as unsigned.
fn div_k(a: i32, b: i32) -> i32 {
    let q = bitcast<i32>(bitcast<u32>(abs(a)) / bitcast<u32>(abs(b)));
    return select(q, -q, (a < 0) != (b < 0));
}
fn rem_k(a: i32, b: i32) -> i32 {
    let r = bitcast<i32>(bitcast<u32>(abs(a)) % bitcast<u32>(abs(b)));
    return select(r, -r, a < 0);
}
fn neg_i(a: i32, line: u32) -> i32 { if (a == i32(-2147483647) - 1) { halt(91u, bitcast<u32>(a), 0u, line); } return -a; }
fn shl_i(a: i32, s: i32, line: u32) -> i32 {
    if (s < 0 || s > 31) { halt(71u, bitcast<u32>(a), bitcast<u32>(s), line); return 0; }
    let r = a << u32(s);
    if ((r >> u32(s)) != a) { halt(71u, bitcast<u32>(a), bitcast<u32>(s), line); }
    return r;
}
fn shr_i(a: i32, s: i32, line: u32) -> i32 { if (s < 0 || s > 31) { halt(81u, bitcast<u32>(a), bitcast<u32>(s), line); return 0; } return a >> u32(s); }
fn add_u(a: u32, b: u32, line: u32) -> u32 { let r = a + b; if (r < a) { halt(12u, a, b, line); } return r; }
fn sub_u(a: u32, b: u32, line: u32) -> u32 { if (b > a) { halt(22u, a, b, line); } return a - b; }
fn mul_u(a: u32, b: u32, line: u32) -> u32 { let r = a * b; if ((a | b) >= 0x10000u && a != 0u && r / a != b) { halt(32u, a, b, line); } return r; }
fn div_u(a: u32, b: u32, line: u32) -> u32 { if (b == 0u) { halt(20u, 0u, 0u, line); return 0u; } return a / b; }
fn rem_u(a: u32, b: u32, line: u32) -> u32 { if (b == 0u) { halt(20u, 0u, 0u, line); return 0u; } return a % b; }
fn shl_u(a: u32, s: u32, line: u32) -> u32 {
    if (s > 31u) { halt(72u, a, s, line); return 0u; }
    let r = a << s;
    if ((r >> s) != a) { halt(72u, a, s, line); }
    return r;
}
fn shr_u(a: u32, s: u32, line: u32) -> u32 { if (s > 31u) { halt(82u, a, s, line); return 0u; } return a >> s; }
fn f_to_i(f: f32, line: u32) -> i32 {
    if (!(f >= -2147483648.0 && f < 2147483648.0)) { halt(331u, bitcast<u32>(f), 0u, line); return 0; }
    return i32(f);
}
fn f_to_u(f: f32, line: u32) -> u32 {
    if (!(f > -1.0 && f < 4294967296.0)) { halt(332u, bitcast<u32>(f), 0u, line); return 0u; }
    return u32(f);
}
fn i_to_u(a: i32, line: u32) -> u32 { if (a < 0) { halt(312u, bitcast<u32>(a), 0u, line); return 0u; } return u32(a); }
fn u_to_i(a: u32, line: u32) -> i32 { if (a > 2147483647u) { halt(321u, a, 0u, line); return 0; } return i32(a); }
fn kround(x: f32) -> f32 {
    let t = trunc(x);
    if (abs(x - t) >= 0.5) { return t + sign(x); }
    return t;
}
fn kpow(a: f32, b: f32) -> f32 {
    if (b == 0.0) { return 1.0; }
    if (a == 0.0) { if (b > 0.0) { return 0.0; } return bitcast<f32>(0x7f800000u); }
    let m = pow(abs(a), b);
    if (a < 0.0) {
        if (b != trunc(b)) { return bitcast<f32>(0x7fc00000u); }
        if (abs(b % 2.0) == 1.0) { return -m; }
    }
    return m;
}
";

/// WGSL's name for a scalar type, or `None` when a kernel can't hold it.
pub(crate) fn wgsl_type(t: Type) -> Option<&'static str> {
    Some(match t {
        Type::I32 => "i32",
        Type::U32 => "u32",
        Type::F32 => "f32",
        Type::Bool => "bool",
        _ => return None,
    })
}

/// `word` (a `u32` from a buffer) as a value of type `t`.
pub(crate) fn load(word: &str, t: Type) -> String {
    match t {
        Type::U32 => word.to_string(),
        Type::Bool => format!("({word} != 0u)"),
        _ => format!("bitcast<{}>({word})", wgsl_type(t).unwrap_or("u32")),
    }
}

/// A value of type `t` as a `u32` for a buffer.
pub(crate) fn store(v: &str, t: Type) -> String {
    match t {
        Type::U32 => v.to_string(),
        Type::Bool => format!("select(0u, 1u, {v})"),
        _ => format!("bitcast<u32>({v})"),
    }
}
