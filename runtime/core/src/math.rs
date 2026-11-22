// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.math`'s inexact functions (spec 14.5, decision 0122): the same
//! code on every system, from the pure-Rust `libm` crate built without its
//! machine-specific paths, so each gives the same bits everywhere. The
//! interpreter, native code and LLVM builds (`kv_math_*`) all use these.

pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

pub fn asin(x: f64) -> f64 {
    libm::asin(x)
}

pub fn acos(x: f64) -> f64 {
    libm::acos(x)
}

pub fn atan(x: f64) -> f64 {
    libm::atan(x)
}

pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

pub fn log10(x: f64) -> f64 {
    libm::log10(x)
}

pub fn cbrt(x: f64) -> f64 {
    libm::cbrt(x)
}

pub fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}

/// IEEE's remainder: `x - n * y`, `n` the whole number nearest `x / y`.
pub fn remainder(x: f64, y: f64) -> f64 {
    libm::remainder(x, y)
}

pub fn erfc(x: f64) -> f64 {
    libm::erfc(x)
}

/// The natural logarithm of the absolute value of the gamma function.
pub fn lgamma(x: f64) -> f64 {
    libm::lgamma(x)
}

/// The NaN watch's panic for an operator of `Float`s, or of `Float32`s
/// when `narrow` (spec 5.9): `NaN watch: 0.0 / 0.0 gives NaN`.
pub fn nan_watch_op(x: f64, op: &str, y: f64, narrow: bool) -> String {
    let show = |v: f64| {
        if narrow {
            crate::format_float32(v as f32)
        } else {
            crate::format_float(v)
        }
    };
    format!("NaN watch: {} {op} {} gives NaN", show(x), show(y))
}

/// The NaN watch's panic for `math.name(x)`, or `(x, y)` for the
/// functions of two (spec 5.9); `pow` shows as `x ** y`.
pub fn nan_watch_call(name: &str, x: f64, y: f64) -> String {
    let show = crate::format_float;
    match name {
        "pow" => nan_watch_op(x, "**", y, false),
        "atan2" | "hypot" | "remainder" => {
            format!("NaN watch: math.{name}({}, {}) gives NaN", show(x), show(y))
        }
        _ => format!("NaN watch: math.{name}({}) gives NaN", show(x)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nan_watch_names_the_operation_and_its_operands() {
        let cases = [
            (nan_watch_op(0.0, "/", 0.0, false), "0.0 / 0.0"),
            (
                nan_watch_op(f64::INFINITY, "-", f64::INFINITY, false),
                "inf - inf",
            ),
            (
                nan_watch_op(0.1f32 as f64, "*", f64::INFINITY, true),
                "0.1 * inf",
            ),
            (nan_watch_call("sqrt", -1.0, 0.0), "math.sqrt(-1.0)"),
            (nan_watch_call("pow", -8.0, 0.5), "-8.0 ** 0.5"),
            (
                nan_watch_call("remainder", 1.0, 0.0),
                "math.remainder(1.0, 0.0)",
            ),
        ];
        for (got, what) in cases {
            assert_eq!(got, format!("NaN watch: {what} gives NaN"));
        }
    }

    /// musl's results, which every system now gives: `sin(1e22)` needs
    /// the full argument reduction some C libraries cut short.
    #[test]
    fn each_function_gives_musls_bits() {
        let cases: [(f64, u64); 6] = [
            (sin(1e22), (-0.852_200_849_767_188_8f64).to_bits()),
            // One unit in the last place above `e`, as musl gives it.
            (exp(1.0), std::f64::consts::E.to_bits() + 1),
            (ln(10.0), std::f64::consts::LN_10.to_bits()),
            (pow(2.0, 0.5), std::f64::consts::SQRT_2.to_bits()),
            (atan2(1.0, 1.0), std::f64::consts::FRAC_PI_4.to_bits()),
            (cos(0.0), 1f64.to_bits()),
        ];
        for (i, (got, want)) in cases.iter().enumerate() {
            assert_eq!(got.to_bits(), *want, "case {i}: {got}");
        }
    }
}
