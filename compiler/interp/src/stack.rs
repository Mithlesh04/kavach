//! How deep interpreted calls may go (spec 3.8): the interpreter recurses
//! on its thread's stack, so it measures how much of the stack is in use
//! instead of counting calls.

/// How much of the thread's stack calls may use before a call is a stack
/// overflow, a panic (spec 3.8): all of it but a margin for the call that
/// finds it so. Measured, not counted, so a call takes the room it needs
/// in any build: about 50,000 small calls deep in a release build, 5,000
/// in a debug one.
const STACK_USE: usize = crate::STACK_SIZE - 32 * 1024 * 1024;

thread_local! {
    /// Where this thread's first call was: the top of its stack, near
    /// enough.
    static STACK_TOP: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How much stack this thread's calls use now.
fn stack_used() -> usize {
    let here = 0u8;
    let at = std::ptr::addr_of!(here) as usize;
    STACK_TOP.with(|top| {
        if top.get() == 0 {
            top.set(at);
        }
        top.get().abs_diff(at)
    })
}

/// Whether a call now would use more of the stack than calls may: then it
/// is a stack overflow, a panic.
pub(crate) fn too_deep() -> bool {
    stack_used() > STACK_USE
}
