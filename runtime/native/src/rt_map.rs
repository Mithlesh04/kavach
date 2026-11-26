// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `fs.map(path)` (spec 14.3): a file mapped into memory, read only, as
//! the bytes of a `Bytes` value (`Data::Mapped`). The system loads each
//! page when it is first read; the mapping is let go with the last holder.

#![allow(unsafe_code)]

use std::sync::Arc;

/// A file's pages in the program's memory, read only. Empty files have
/// no mapping (`at` is null, `len` 0), as no system maps zero bytes.
pub struct Mapping {
    at: *const u8,
    len: usize,
}

// SAFETY: the pages are read only and nobody in the program writes them;
// unmapping happens once, in `drop`, when no holder is left.
unsafe impl Send for Mapping {}
// SAFETY: as above: shared reads of read-only memory.
unsafe impl Sync for Mapping {}

impl Mapping {
    /// The mapped bytes. Another program can change them between two
    /// reads, so a check of them holds only for a copy made first and then
    /// checked (`rt_bytes::text_of`); no code may check them in place and
    /// then trust a second read.
    pub fn bytes(&self) -> &[u8] {
        if self.len == 0 {
            return &[];
        }
        // SAFETY: `at` points to `len` mapped bytes that stay mapped until
        // `drop`. Another program can change them (spec 14.3 says so), but
        // they stay readable memory while the file is as long as it was.
        unsafe { std::slice::from_raw_parts(self.at, self.len) }
    }
}

/// Maps the file at `path`.
pub fn map(path: &str) -> std::io::Result<Arc<Mapping>> {
    let file = std::fs::File::open(path)?;
    let meta = file.metadata()?;
    if meta.is_dir() {
        return Err(std::io::ErrorKind::IsADirectory.into());
    }
    let len = usize::try_from(meta.len()).map_err(|_| std::io::Error::other("too big to map"))?;
    if len == 0 {
        return Ok(Arc::new(Mapping {
            at: std::ptr::null(),
            len: 0,
        }));
    }
    let at = sys::map(&file, len)?;
    Ok(Arc::new(Mapping { at, len }))
}

impl Drop for Mapping {
    fn drop(&mut self) {
        if self.len != 0 {
            sys::unmap(self.at, self.len);
        }
    }
}

#[cfg(unix)]
mod sys {
    use std::os::fd::AsRawFd;

    pub fn map(file: &std::fs::File, len: usize) -> std::io::Result<*const u8> {
        // SAFETY: a new read-only, shared mapping of an open file; the
        // file can be closed after, the mapping keeps its own reference.
        let at = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };
        if at == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }
        Ok(at as *const u8)
    }

    pub fn unmap(at: *const u8, len: usize) {
        // SAFETY: `at` and `len` are a mapping made by `map`, unmapped once.
        unsafe { libc::munmap(at as *mut libc::c_void, len) };
    }
}

#[cfg(windows)]
mod sys {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Memory::{
        CreateFileMappingW, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile,
        PAGE_READONLY, UnmapViewOfFile,
    };

    pub fn map(file: &std::fs::File, len: usize) -> std::io::Result<*const u8> {
        // SAFETY: a read-only mapping object of an open file, the size of
        // the whole file (0, 0), with no name and default security.
        let mapping = unsafe {
            CreateFileMappingW(
                file.as_raw_handle(),
                std::ptr::null(),
                PAGE_READONLY,
                0,
                0,
                std::ptr::null(),
            )
        };
        if mapping.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: a read-only view of all `len` bytes of that mapping. The
        // view keeps the mapping object alive, so its handle closes now.
        let view = unsafe { MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, len) };
        let failed = view.Value.is_null().then(std::io::Error::last_os_error);
        // SAFETY: `mapping` is a handle made above, closed once.
        unsafe { CloseHandle(mapping) };
        match failed {
            Some(e) => Err(e),
            None => Ok(view.Value as *const u8),
        }
    }

    pub fn unmap(at: *const u8, _len: usize) {
        let view = MEMORY_MAPPED_VIEW_ADDRESS {
            Value: at as *mut core::ffi::c_void,
        };
        // SAFETY: `at` is a view made by `map`, unmapped once.
        unsafe { UnmapViewOfFile(view) };
    }
}
