//! Values of a feature's own type, such as a socket or a regular
//! expression, inside a heap object (`Data`). Only the feature's code
//! knows the type: an [`Opaque`] is a pointer made by `Arc::into_raw`,
//! with the feature's functions that share and free it. Freeing an object
//! calls through them, so a program that never uses the feature doesn't
//! link its code (decision 0024). (With an `Arc<dyn Any>`, the compiler
//! put the drop code next to the network's, which brought TLS into every
//! program.)

/// How to share and free the values of one feature's type. Each feature
/// makes its own with [`opaque_kind!`].
pub struct Kind {
    /// Adds a reference to the value.
    pub retain: unsafe fn(*const ()),
    /// Lets go of a reference, freeing the value with the last one.
    pub release: unsafe fn(*const ()),
    /// What printing, comparing and hashing go by: a regex's pattern.
    pub describe: unsafe fn(*const ()) -> String,
}

/// One reference to a feature's value.
pub struct Opaque {
    ptr: *const (),
    kind: &'static Kind,
}

// SAFETY: `opaque_kind!` only accepts types that are `Send + Sync`, and an
// `Opaque` is only made from an `Arc` of the kind's type.
unsafe impl Send for Opaque {}
// SAFETY: as above.
unsafe impl Sync for Opaque {}

impl Opaque {
    /// Wraps `ptr`, from `Arc::into_raw` of a value of `kind`'s type,
    /// taking over that reference.
    ///
    /// # Safety
    /// `ptr` must come from `Arc::into_raw` of the type `kind` was made for.
    pub unsafe fn from_raw(ptr: *const (), kind: &'static Kind) -> Opaque {
        Opaque { ptr, kind }
    }

    /// The pointer, if the value is of `kind`: borrowed, still held here.
    pub fn get(&self, kind: &'static Kind) -> Option<*const ()> {
        std::ptr::eq(self.kind, kind).then_some(self.ptr)
    }

    /// What printing, comparing and hashing the value go by.
    pub fn describe(&self) -> String {
        // SAFETY: `ptr` is a live value of the kind's type (`from_raw`).
        unsafe { (self.kind.describe)(self.ptr) }
    }

    /// What tells shared copies of one value from other values.
    pub fn address(&self) -> usize {
        self.ptr as usize
    }
}

impl Clone for Opaque {
    fn clone(&self) -> Opaque {
        // SAFETY: `ptr` is a live `Arc` of the kind's type (`from_raw`).
        unsafe { (self.kind.retain)(self.ptr) };
        Opaque {
            ptr: self.ptr,
            kind: self.kind,
        }
    }
}

impl Drop for Opaque {
    fn drop(&mut self) {
        // SAFETY: this `Opaque` holds one reference, given up here.
        unsafe { (self.kind.release)(self.ptr) };
    }
}

/// Defines `static $kind: Kind` for values of type `$t` (`Send + Sync`),
/// described by `$describe(&$t) -> String`, with functions
/// `$wrap(Arc<$t>) -> Opaque` and `$unwrap(&Opaque) -> Option<Arc<$t>>`.
/// The functions are made in the feature's own module, so its code stays
/// apart from the rest.
macro_rules! opaque_kind {
    ($kind:ident, $t:ty, $wrap:ident, $unwrap:ident, $describe:expr) => {
        static $kind: $crate::opaque::Kind = {
            const fn send_sync<T: Send + Sync>() {}
            send_sync::<$t>();
            unsafe fn retain(p: *const ()) {
                // SAFETY: `p` is a live `Arc<$t>` (`Opaque::from_raw`).
                unsafe { std::sync::Arc::increment_strong_count(p as *const $t) }
            }
            unsafe fn release(p: *const ()) {
                // SAFETY: as above; this gives up one reference.
                unsafe { drop(std::sync::Arc::from_raw(p as *const $t)) }
            }
            unsafe fn describe(p: *const ()) -> String {
                // SAFETY: as above, borrowed.
                let value: &$t = unsafe { &*(p as *const $t) };
                $describe(value)
            }
            $crate::opaque::Kind {
                retain,
                release,
                describe,
            }
        };

        fn $wrap(value: std::sync::Arc<$t>) -> $crate::opaque::Opaque {
            let ptr = std::sync::Arc::into_raw(value) as *const ();
            // SAFETY: `ptr` comes from `Arc::into_raw` of an `Arc<$t>`.
            unsafe { $crate::opaque::Opaque::from_raw(ptr, &$kind) }
        }

        fn $unwrap(o: &$crate::opaque::Opaque) -> Option<std::sync::Arc<$t>> {
            let p = o.get(&$kind)? as *const $t;
            // SAFETY: `p` is a live `Arc<$t>`, held by `o`; this adds the
            // reference the new `Arc` gives up when dropped.
            unsafe {
                std::sync::Arc::increment_strong_count(p);
                Some(std::sync::Arc::from_raw(p))
            }
        }
    };
}
pub(crate) use opaque_kind;

/// What a regex (or other feature) object `p` prints, compares and hashes
/// by: through its kind, so this needs none of the feature's code.
pub fn describe(p: i64) -> String {
    match &*crate::heap::data(p).borrow() {
        crate::heap::Data::Regex(o) | crate::heap::Data::Socket(o) => o.describe(),
        _ => String::new(),
    }
}
