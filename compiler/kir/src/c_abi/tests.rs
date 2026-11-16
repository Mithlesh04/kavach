//! Records classified as each system's C does, from the ABI documents'
//! own examples.

use super::*;

fn layout(fields: &[(Type, i128)], size: i128) -> Layout<'_> {
    Layout { size, fields }
}

#[test]
fn system_v_puts_eightbytes_in_registers_by_their_fields() {
    let point = [(Type::I32, 0), (Type::I32, 4)];
    let complex = [(Type::F64, 0), (Type::F64, 8)];
    let mixed = [(Type::I64, 0), (Type::F32, 8), (Type::F32, 12)];
    let float_int = [(Type::F32, 0), (Type::I32, 4)];
    let big = [(Type::I64, 0), (Type::I64, 8), (Type::I64, 16)];
    let (p, c, m, fi, b) = (
        layout(&point, 8),
        layout(&complex, 16),
        layout(&mixed, 16),
        layout(&float_int, 8),
        layout(&big, 24),
    );
    let got = passed(
        Rules::SysV64,
        &[Ok(&p), Ok(&c), Ok(&m), Ok(&fi), Ok(&b)],
        false,
    );
    use Part::{Float as F, Int as I};
    assert_eq!(
        got,
        vec![
            Some(Passed::Parts(vec![I])),
            Some(Passed::Parts(vec![F, F])),
            Some(Passed::Parts(vec![I, F])),
            Some(Passed::Parts(vec![I])),
            Some(Passed::Stack),
        ]
    );
    assert_eq!(returned(Rules::SysV64, &c), Returned::Parts(vec![F, F]));
    assert_eq!(returned(Rules::SysV64, &b), Returned::Hidden);
}

#[test]
fn system_v_puts_a_record_on_the_stack_when_registers_run_out() {
    let pair = [(Type::I64, 0), (Type::I64, 8)];
    let p = layout(&pair, 16);
    // Five whole numbers first, with a hidden pointer: one register left.
    let plain: Vec<Result<&Layout, Type>> = vec![Err(Type::I64); 4];
    let mut args = plain;
    args.push(Ok(&p));
    let got = passed(Rules::SysV64, &args, true);
    assert_eq!(got[4], Some(Passed::Stack));
}

#[test]
fn windows_passes_only_whole_number_sizes_in_a_register() {
    let small = [(Type::F32, 0)];
    let three = [(Type::U8, 0), (Type::U8, 1), (Type::U8, 2)];
    let complex = [(Type::F64, 0), (Type::F64, 8)];
    let (s, t, c) = (layout(&small, 4), layout(&three, 3), layout(&complex, 16));
    let got = passed(Rules::Win64, &[Ok(&s), Ok(&t), Ok(&c)], false);
    assert_eq!(
        got,
        vec![
            Some(Passed::Parts(vec![Part::Int])),
            Some(Passed::Copy),
            Some(Passed::Copy),
        ]
    );
    assert_eq!(returned(Rules::Win64, &s), Returned::Parts(vec![Part::Int]));
    assert_eq!(returned(Rules::Win64, &c), Returned::Hidden);
}
