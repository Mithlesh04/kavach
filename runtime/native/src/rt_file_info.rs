//! A file's details and permissions in native code, renaming, copying
//! and removing a folder with its contents (spec 14.3), through
//! `kavach_runtime_core::file_info`, as the interpreter does. An entry of
//! its own, so only a program that uses them links them (decision 0024).

use crate::heap::{alloc_slots, release, str_of};
use crate::rt::type_of;
use kavach_kir::RtOp;
use kavach_runtime_core::file_info;

// `a` is the path (owned); `b` the second path (owned) of `rename` and
// `copy`, the read-only flag, or the permission bits.
crate::rt_data::entry!(kv_rt_file_info, |op, a, b, _c, code, _line| {
    crate::rt_tasks::turn();
    let path = str_of(a);
    release(a);
    let two_paths = matches!(
        op,
        RtOp::FileInfoRename | RtOp::FileInfoCopy | RtOp::FileInfoSymlink
    );
    let to = if two_paths {
        let to = str_of(b);
        release(b);
        to
    } else {
        String::new()
    };
    // Outside the folders of `uses Fs("…")` (spec 8.1): both paths count.
    if !crate::rt_files::allowed(&path) {
        return crate::rt_files::denied(&path);
    }
    if two_paths && !crate::rt_files::allowed(&to) {
        return crate::rt_files::denied(&to);
    }
    let done = match op {
        RtOp::FileInfoStat => {
            return match file_info::stat(&path) {
                Ok(n) => alloc_slots(type_of(code), 0, n.iter().map(|v| *v as u64).collect()),
                Err(e) => crate::rt_std::fail("read", &path, e),
            };
        }
        RtOp::FileInfoSetReadOnly => file_info::set_read_only(&path, b != 0),
        RtOp::FileInfoSetPermissions => file_info::set_permissions(&path, b),
        RtOp::FileInfoRename => file_info::rename(&path, &to),
        RtOp::FileInfoCopy => file_info::copy(&path, &to),
        RtOp::FileInfoSymlink => file_info::symlink(&path, &to),
        _ => file_info::remove_all(&path),
    };
    match done {
        Ok(()) => 0,
        Err(e) => crate::rt_std::fail("change", &path, e),
    }
});
