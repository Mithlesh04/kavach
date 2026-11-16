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

#[cfg(test)]
mod tests {
    use super::*;

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
