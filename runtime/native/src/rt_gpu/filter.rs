//! `list.filter(f)` on the GPU (spec 15.2): the runtime side of
//! `GpuFilter` and `GpuFilterStay`. Like `sum`, it runs only on a list
//! already in GPU memory (unless `KAVACH_GPU=on`): copying a list over
//! costs more than the CPU's pass. It gives the new list, or 0 for the
//! CPU to filter it; the kept items may stay in GPU memory only, as a
//! `map`'s do.

use super::{captures_of, fetch, keep, message, resident, with_kernel_bytes};
use crate::heap::{obj, release, shape, with_str};
use crate::rt::{panic_with, type_of};
use kavach_kir::{Shape, Type};
use kavach_runtime_gpu::{GpuError, Items};

pub(super) fn filter(list: i64, closure: i64, text: i64, code: i64, line: i32, stay: bool) -> i64 {
    let kernel = with_str(text, str::to_string);
    release(text);
    let n = crate::heap::list_len_items_pub(list);
    let mode = std::env::var("KAVACH_GPU").unwrap_or_default();
    let on_gpu = resident(list);
    let ty = obj(list).ty();
    let wanted = !kernel.is_empty()
        && n > 0
        && crate::flat::item_size(ty) == 4
        && !matches!(mode.as_str(), "off" | "check")
        && (on_gpu.is_some() || mode == "on");
    let captures = if wanted {
        captures_of(closure)
    } else {
        Vec::new()
    };
    release(closure);
    if !wanted {
        // The CPU reads it next.
        fetch(list);
        release(list);
        return 0;
    }
    let item = match shape(ty) {
        Shape::List(t) => *t,
        _ => Type::Unit,
    };
    let res = match on_gpu {
        Some(r) => kavach_runtime_gpu::filter(&kernel, Items::Resident(&r), n, &captures, stay),
        None => with_kernel_bytes(list, item, n, |b| {
            kavach_runtime_gpu::filter(&kernel, Items::Bytes(b), n, &captures, stay)
        }),
    };
    // It waited for the GPU: an earlier `map` that didn't is done too.
    super::finish_late();
    match res {
        Ok((len, kept, words)) => {
            if let Some(sent) = kept.items {
                keep(list, sent, false);
            }
            release(list);
            let list_ty = match shape(type_of(code)) {
                Shape::Optional(t) => *t,
                _ => type_of(code),
            };
            let mut slots = vec![0u64; len.div_ceil(2)];
            if let Some(words) = &words {
                super::as_bytes_mut(&mut slots)[..len * 4].copy_from_slice(words);
            }
            let made = crate::heap::new_list_raw(list_ty, 0, slots, len);
            if let Some(on_gpu) = kept.results {
                keep(made, on_gpu, words.is_none());
            }
            made
        }
        Err(GpuError::Unavailable) => {
            fetch(list);
            release(list);
            0
        }
        Err(GpuError::Stopped {
            code,
            a,
            b,
            line: at,
        }) => {
            let at = i32::try_from(at).ok().filter(|l| *l > 0).unwrap_or(line);
            panic_with(&message(code, a, b, &kernel), at)
        }
    }
}
