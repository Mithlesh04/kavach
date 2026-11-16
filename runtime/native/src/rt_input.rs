//! `std.input` in native code (spec 14.26, decision 0113), from
//! `kavach_runtime_input`, as the interpreter does. An entry of its own,
//! so only a program that reads gamepads links SDL3 (decision 0024).

use crate::heap::{alloc_slots, new_str, release, shape, slots};
use crate::rt::type_of;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_input as input;

/// The item type of the list type `code`.
fn item(code: i64) -> Type {
    match shape(type_of(code)) {
        Shape::List(t) => *t,
        _ => Type::I64,
    }
}

// `a` is a pad's number for `InputRumble`, `b` the list of its motors'
// strengths (owned) and `c` the milliseconds.
crate::rt_data::entry!(kv_rt_input, |op, a, b, c, code, _line| {
    match op {
        RtOp::InputStart => {
            input::start();
            0
        }
        RtOp::InputPads => {
            let numbers = input::pad_numbers();
            alloc_slots(
                type_of(code),
                0,
                numbers.iter().map(|f| f.to_bits()).collect(),
            )
        }
        RtOp::InputNames => {
            let t = item(code);
            let names = (input::names().into_iter())
                .map(|s| new_str(s, t) as u64)
                .collect();
            alloc_slots(type_of(code), 0, names)
        }
        _ => {
            let motors: Vec<f64> = slots(b).iter().map(|s| f64::from_bits(*s)).collect();
            release(b);
            let part = |k: usize| motors.get(k).copied().unwrap_or(0.0);
            input::rumble_pad(a, part(0), part(1), c);
            0
        }
    }
});
