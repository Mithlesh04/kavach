//! `std.math`'s functions of floats (spec 14.5), as native code works them
//! out: a `Float32` is widened to a `Float`, worked out, and rounded once.

/// `math.name(x)` (or `(x, y)` for `pow` and `atan2`) of `Float`s, or
/// `None` when `name` isn't one of these.
pub(crate) fn float_fn(name: &str, x: f64, y: f64) -> Option<f64> {
    Some(match name {
        "sqrt" => x.sqrt(),
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "round" => x.round(),
        "pow" => x.powf(y),
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "asin" => x.asin(),
        "acos" => x.acos(),
        "atan" => x.atan(),
        "atan2" => x.atan2(y),
        "exp" => x.exp(),
        "ln" => x.ln(),
        _ => return None,
    })
}
