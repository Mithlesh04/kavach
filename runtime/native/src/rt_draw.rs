//! `std.gpu`'s drawing in native code (spec 14.24, decision 0106), from
//! `kavach_runtime_gpu::draw`, by number, as the interpreter does
//! (`compiler/interp/src/std_gpu.rs`). The window comes from
//! `kavach_runtime_window`. An entry of its own, so only a program that
//! draws links it (decision 0024).

use crate::heap::{release, slots};
use crate::rt_errors::set_error;
use kavach_kir::{RtOp, Shape};
use kavach_runtime_gpu::{draw, render};
use kavach_runtime_window as win;

/// Leaves `why` as a `GpuError.Unavailable` for `kv_take_error`.
fn fail(why: &str) -> i64 {
    fail_as("Unavailable", why)
}

/// Leaves `why` as a `GpuError` of `variant` for `kv_take_error`.
fn fail_as(variant: &str, why: &str) -> i64 {
    if let Some(ty) = crate::shapes::find_union("GpuError", "Unavailable") {
        let text = crate::shapes::new_text(why, crate::heap::first_of(&Shape::Str));
        set_error(crate::rt_json::new_variant(ty, variant, &[text as u64]));
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
crate::rt_data::entry!(kv_rt_draw, |op, a, b, c, _code, line| {
    match op {
        RtOp::DrawScreen => draw::screen_on(a, size(a)).unwrap_or_else(|why| fail(&why)),
        RtOp::DrawCompressed => {
            let made = crate::rt_bytes::with_bytes(a, kavach_runtime_gpu::texture::compressed);
            release(a);
            match made {
                Ok((n, w, h)) => crate::heap::alloc_slots(
                    crate::rt::type_of(_code),
                    0,
                    vec![n as u64, u64::from(w), u64::from(h)],
                ),
                Err((variant, why)) => fail_as(variant, &why),
            }
        }
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
        // `a` and `b` are the gpu fns, as closures; `c` the module's text.
        RtOp::DrawPipeline => {
            release(a);
            release(b);
            let text = crate::shapes::with_str(c, str::to_string);
            release(c);
            match kavach_runtime_gpu::render::pipeline(&text) {
                Ok(n) => n,
                Err(why) => crate::rt::panic_with(&why, line),
            }
        }
        // `a` wide, `b` high, the pixels in list `c` (owned).
        RtOp::DrawTexture => {
            let pixels = crate::rt_bytes::list_bytes(c);
            release(c);
            let fit = |v: i64| u32::try_from(v).unwrap_or(0);
            kavach_runtime_gpu::texture::texture(fit(a), fit(b), &pixels)
                .unwrap_or_else(|why| fail(&why))
        }
        RtOp::DrawDropTexture => {
            kavach_runtime_gpu::texture::drop_texture(a);
            0
        }
        // `a` is the frame, `b` the pipeline, `c` the tuple of vertices,
        // instances, shared value, textures and indices.
        RtOp::DrawDraw => {
            let parts = slots(c).to_vec();
            // A list a GPU `map` left in GPU memory is drawn from there,
            // with nothing copied (decision 0106); any other, as words.
            let kept = [
                crate::rt_gpu::resident(parts[0] as i64),
                crate::rt_gpu::resident(parts[1] as i64),
            ];
            let words = |k: usize| match kept[k] {
                Some(_) => Vec::new(),
                None => list_words(parts[k] as i64),
            };
            let (vw, iw, shared) = (words(0), words(1), record_words(parts[2] as i64));
            let vertices = match &kept[0] {
                Some(r) => render::Items::Kept(r),
                None => render::Items::Words(&vw),
            };
            let instances = match &kept[1] {
                Some(r) => render::Items::Kept(r),
                None => render::Items::Words(&iw),
            };
            // Each texture's number, its first field.
            let textures: Vec<i64> = (crate::heap::items_of(parts[3] as i64).iter())
                .map(|t| slots(*t as i64)[0] as i64)
                .collect();
            // The indices, when given: `none` is 0.
            let indices = (parts[4] != 0).then(|| list_words(parts[4] as i64));
            let drawn = draw::draw(
                a,
                render::Draw {
                    pipeline: b,
                    parts: (vertices, instances, &shared),
                    textures: &textures,
                    indices: indices.as_deref(),
                },
            );
            release(c);
            if let Err(why) = drawn {
                crate::rt::panic_with(&why, line);
            }
            0
        }
        _ => {
            draw::close(a);
            0
        }
    }
});

/// A list of records as the GPU reads it: each item's fields, one word
/// each, item after item.
fn list_words(list: i64) -> Vec<u32> {
    let ty = crate::heap::obj(list).ty();
    let item = crate::flat::item_type(ty);
    let n = crate::heap::list_len_items_pub(list);
    crate::rt_gpu::with_kernel_bytes(list, item, n, |bytes| {
        let (words, _) = bytes.as_chunks::<4>();
        words.iter().map(|b| u32::from_le_bytes(*b)).collect()
    })
}

/// The shared value: a record's fields, one word each; none for `none`.
fn record_words(record: i64) -> Vec<u32> {
    if record == 0 {
        return Vec::new();
    }
    (slots(record).iter().enumerate())
        .map(|(i, s)| crate::rt_gpu::word(*s, crate::shapes::slot_type(record, i)))
        .collect()
}
