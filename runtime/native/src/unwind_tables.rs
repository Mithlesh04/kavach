//! Registers the unwind tables of code the JIT made (`kavach run
//! --native`; decision 0065, step 3), so a panic can unwind through it to
//! the start of its task. Built programs carry their tables in the
//! executable; the JIT's code is made at run time, so the system is told
//! of its tables here. The tables themselves are written by the Cranelift
//! backend (`compiler/backend/cranelift/src/jit_unwind.rs`).

/// A function's entry in a Windows function table: its start, its end and
/// its unwind codes, each relative to the table's base address.
#[repr(C)]
pub struct RuntimeFunction {
    pub begin: u32,
    pub end: u32,
    pub unwind: u32,
}

#[cfg(target_os = "linux")]
unsafe extern "C" {
    /// libgcc's: registers an `.eh_frame` section, ended by a zero word.
    fn __register_frame(begin: *const u8);
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn RtlAddFunctionTable(table: *const RuntimeFunction, count: u32, base: u64) -> u8;
}

/// Registers `eh_frame`, a whole `.eh_frame` section ended by a zero word,
/// for as long as the program runs (Linux; elsewhere it does nothing).
#[allow(unsafe_code)]
pub fn register_eh_frame(eh_frame: Vec<u8>) {
    #[cfg(target_os = "linux")]
    {
        let section: &'static [u8] = eh_frame.leak();
        // SAFETY: `section` is a complete `.eh_frame` (a CIE, its FDEs and
        // the zero word that ends it) describing code that lives as long
        // as the program, and it is kept alive just as long.
        unsafe { __register_frame(section.as_ptr()) };
    }
    #[cfg(not(target_os = "linux"))]
    drop(eh_frame);
}

/// Registers a function table whose addresses are relative to `base`, for
/// as long as the program runs (Windows; elsewhere it does nothing).
/// `false` when the system refused it.
#[allow(unsafe_code)]
pub fn add_function_table(table: Vec<RuntimeFunction>, base: u64) -> bool {
    #[cfg(windows)]
    {
        let table: &'static [RuntimeFunction] = table.leak();
        let count = u32::try_from(table.len()).unwrap_or(0);
        // SAFETY: `table` lists functions of code that lives as long as the
        // program, sorted by start, each with unwind codes `base` reaches
        // in 32 bits; the table is kept alive as long as the code.
        unsafe { RtlAddFunctionTable(table.as_ptr(), count, base) != 0 }
    }
    #[cfg(not(windows))]
    {
        let _ = (table, base);
        true
    }
}
