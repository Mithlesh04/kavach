//! `std.fs`'s `FileWriter` in native code (spec 14.3), from
//! `kavach_runtime_core::writers`, by the writer's number, as the
//! interpreter does (`compiler/interp/src/std_writers.rs`). A path is
//! checked against `uses Fs("…")` when its writer opens. An entry of its
//! own, so only a program that writes files this way links it.

use crate::heap::release;
use crate::rt_bytes::with_bytes;
use crate::shapes::str_of;
use kavach_kir::RtOp;
use kavach_runtime_core::writers;

// `a` is the path (owned) for `FsWriterOpen`, else the writer's number;
// `b` the `Bytes` (owned) to write, or `finish`'s sync flag.
crate::rt_data::entry!(kv_rt_writers, |op, a, b, _c, _code, _line| {
    match op {
        RtOp::FsWriterOpen => {
            let path = str_of(a);
            release(a);
            if !crate::rt_scopes::allowed(&path) {
                return crate::rt_files::denied(&path);
            }
            writers::open(&path).unwrap_or_else(|e| crate::rt_std::fail("open", &path, e))
        }
        RtOp::FsWriterWrite => {
            let wrote = with_bytes(b, |data| writers::write(a, data));
            release(b);
            match wrote {
                Ok(()) => 0,
                Err(e) => crate::rt_std::fail("write", &writers::path(a), e),
            }
        }
        RtOp::FsWriterFinish => match writers::finish(a, b != 0) {
            Ok(n) => n,
            Err(e) => crate::rt_std::fail("finish", &writers::path(a), e),
        },
        _ => {
            writers::close(a);
            0
        }
    }
});
