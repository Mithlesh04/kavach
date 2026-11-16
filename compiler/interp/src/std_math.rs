//! `std.math`'s functions of floats (spec 14.5), as native code works them
//! out: a `Float32` is widened to a `Float`, worked out, and rounded once.
//! The inexact ones are `kavach_runtime_core::math`'s, the same bits on
//! every system (decision 0122).

use kavach_runtime_core::math;

/// `math.name(x)` (or `(x, y)` for `pow` and `atan2`) of `Float`s, or
/// `None` when `name` isn't one of these.
pub(crate) fn float_fn(name: &str, x: f64, y: f64) -> Option<f64> {
    Some(match name {
        "sqrt" => x.sqrt(),
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "round" => x.round(),
        "pow" => math::pow(x, y),
        "sin" => math::sin(x),
        "cos" => math::cos(x),
        "tan" => math::tan(x),
        "asin" => math::asin(x),
        "acos" => math::acos(x),
        "atan" => math::atan(x),
        "atan2" => math::atan2(x, y),
        "exp" => math::exp(x),
        "ln" => math::ln(x),
        _ => return None,
    })
}
