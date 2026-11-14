//! `std.gpu`'s drawing in native code (spec 14.24, decision 0106), from
//! `kavach_runtime_gpu::draw`, by number, as the interpreter does
//! (`compiler/interp/src/std_gpu.rs`). The window comes from
//! `kavach_runtime_window`. An entry of its own, so only a program that
//! draws links it (decision 0024).

use crate::heap::{release, slots};
use crate::rt_errors::set_error;
use kavach_kir::{RtOp, Shape};
use kavach_runtime_gpu::draw;
use kavach_runtime_window as win;

/// Leaves `why` as a `GpuError.Unavailable` for `kv_take_error`.
fn fail(why: &str) -> i64 {
    if let Some(ty) = crate::shapes::find_union("GpuError", "Unavailable") {
        let text = crate::shapes::new_text(why, crate::heap::first_of(&Shape::Str));
        set_error(crate::rt_json::new_variant(
            ty,
            "Unavailable",
            &[text as u64],
        ));
    }
    0
}

/// Window `n`'s size now, in pixels.
fn size(n: i64) -> (u32, u32) {
    let (w, h) = win::size(n);
    let fit = |v: i64| u32::try_from(v).unwrap_or(0);
    (fit(w), fit(h))
}

// `a` is a window, screen or frame number; `b` the screen's window, or
// the list of colors (owned) for `DrawClear`.
crate::rt_data::entry!(kv_rt_draw, |op, a, b, _c, _code, _line| {
    match op {
        RtOp::DrawScreen => draw::screen_on(a, size(a)).unwrap_or_else(|why| fail(&why)),
        RtOp::DrawFrame => {
            let (w, h) = size(b);
            draw::frame(a, w, h).unwrap_or_else(|why| fail(&why))
        }
        RtOp::DrawClear => {
            let rgb: Vec<f64> = slots(b).iter().map(|s| f64::from_bits(*s)).collect();
            release(b);
            let part = |i: usize| rgb.get(i).copied().unwrap_or(0.0);
            draw::clear(a, part(0), part(1), part(2));
            0
        }
        RtOp::DrawShow => {
            draw::show(a);
            0
        }
        RtOp::DrawDropFrame => {
            draw::drop_frame(a);
            0
        }
        _ => {
            draw::close(a);
            0
        }
    }
});
