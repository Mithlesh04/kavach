//! Operators and checks as WGSL text: what `expr.rs` writes for an
//! operator, and whether written code can stop an item.

use kavach_kir::{BinOp, Type};

/// Whether WGSL `code` calls anything that can stop an item: `halt`, a
/// checked helper (`add_i`, `f_to_u`…), or a function of the program
/// (`f12`), which may.
pub(crate) fn can_stop(code: &str) -> bool {
    let helper = ["halt(", "_i(", "_u(", "f_to_", "i_to_", "u_to_"]
        .iter()
        .any(|h| code.contains(h));
    let call = code.match_indices('f').any(|(i, _)| {
        let before = code[..i]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');
        let digits: String = code[i + 1..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        before && !digits.is_empty() && code[i + 1 + digits.len()..].starts_with('(')
    });
    helper || call
}

/// `x op y` on operands of type `t`: checked for whole numbers.
pub(crate) fn binary(op: BinOp, t: Type, x: &str, y: &str, line: u32) -> Option<String> {
    let s = match t {
        Type::I32 => "i",
        Type::U32 => "u",
        _ => "",
    };
    // A check that stops an item says the line, for the panic (spec 15.2).
    let checked = |name: &str| format!("{name}_{s}({x}, {y}, {line}u)");
    Some(match op {
        BinOp::Eq => format!("({x} == {y})"),
        BinOp::Ne => format!("({x} != {y})"),
        BinOp::Lt => format!("({x} < {y})"),
        BinOp::Le => format!("({x} <= {y})"),
        BinOp::Gt => format!("({x} > {y})"),
        BinOp::Ge => format!("({x} >= {y})"),
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div if t == Type::F32 => {
            format!("({x} {} {y})", op.symbol())
        }
        BinOp::Rem if t == Type::F32 => format!("({x} % {y})"),
        BinOp::Add if !s.is_empty() => checked("add"),
        BinOp::Sub if !s.is_empty() => checked("sub"),
        BinOp::Mul if !s.is_empty() => checked("mul"),
        BinOp::Div if !s.is_empty() => checked("div"),
        BinOp::Rem if !s.is_empty() => checked("rem"),
        BinOp::Shl if !s.is_empty() => checked("shl"),
        BinOp::Shr if !s.is_empty() => checked("shr"),
        BinOp::BitAnd if !s.is_empty() => format!("({x} & {y})"),
        BinOp::BitOr if !s.is_empty() => format!("({x} | {y})"),
        BinOp::BitXor if !s.is_empty() => format!("({x} ^ {y})"),
        _ => return None,
    })
}
