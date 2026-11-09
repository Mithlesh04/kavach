//! Heap values for native code: text, lists, tuples, records, unions and
//! boxed optionals (spec 6.1). Each object is reference-counted; changing a
//! shared object copies it first (copy-on-write), so values never alias.
//!
//! Every value travels in a 64-bit slot: integers, floats (as bits),
//! booleans, characters, or a pointer to an object (0 means `none`).
//!
//! Every object starts with the same 24-byte header (`#[repr(C)]`), so
//! generated code reads the reference count, fields and list items itself,
//! without calling the runtime. There are three kinds of object:
//! - a tuple, record, union value, closure or `Shared` keeps its slots right
//!   after the header (offset `FIELDS`), in the same allocation: a record
//!   with two fields takes 40 bytes;
//! - a list keeps a growable buffer (a `Vec<u64>` taken apart) after the
//!   header (offsets `PTR`, `LEN`, `CAP`);
//! - text, decimals, maps and the rest keep their contents in a `Data`
//!   after the header.

pub use crate::lists::{push_item, with_slots, with_words};
use crate::live::count_live;
pub use crate::live::{flush_live, kv_live, live_objects};
pub use crate::rc::{kv_free, release, release_slot, retain, unique};
pub(crate) use crate::shapes::slot_type;
pub use crate::shapes::{
    first_of, innermost, items_of, len_of, new_str, new_text, set_shapes, shape, slot_at,
    slot_types, str_of, with_str,
};
use kavach_kir::{Shape, Type};
use kavach_runtime_core::Decimal;
use std::alloc::Layout;
use std::cell::RefCell;

/// Byte offsets of the fields generated code uses.
pub const RC: i32 = 0;
pub const TAG: i32 = 8;
/// The kind of object, a `u16` (`FIXED`, …).
pub const KIND: i32 = 12;
/// Where a fixed object's slots start.
pub const FIELDS: i32 = 24;
/// A list's buffer, length and capacity.
pub const PTR: i32 = 24;
pub const LEN: i32 = 32;
pub const CAP: i32 = 40;

/// The kinds of object (`Obj::kind`).
pub(crate) const LIST: u16 = 0;
pub const FIXED: u16 = 1;
pub(crate) const DATA: u16 = 2;

/// Whether `p` holds contents other than slots (text, maps, …).
pub(crate) fn has_data(p: i64) -> bool {
    obj(p).kind == DATA
}

/// The header every object starts with.
#[repr(C)]
pub struct Obj {
    pub rc: usize,
    /// The variant of a union value (0 for everything else).
    pub tag: u32,
    pub(crate) kind: u16,
    /// The number of slots of a fixed object.
    pub(crate) fixed_len: u16,
    /// The object's type: its index in the shape table (see `ty`).
    pub type_id: u32,
    _pad: u32,
}

impl Obj {
    /// The object's type. (A `u32` id is kept rather than a `Type`, whose
    /// layout Rust doesn't fix, so generated code can write headers.)
    pub fn ty(&self) -> Type {
        match self.type_id {
            NOT_REF => Type::I64,
            id => Type::Ref(id),
        }
    }
}

/// The `type_id` of an object made with a type that is not a heap type
/// (a runtime fallback; such objects are text).
const NOT_REF: u32 = u32::MAX;

/// Where the type id is in the header.
pub const TYPE_ID: i32 = 16;

// Generated code relies on these offsets.
const _: () = assert!(size_of::<Obj>() == FIELDS as usize);
const _: () = assert!(std::mem::offset_of!(Obj, tag) == TAG as usize);
const _: () = assert!(std::mem::offset_of!(Obj, kind) == KIND as usize);
const _: () = assert!(std::mem::offset_of!(Obj, type_id) == TYPE_ID as usize);
const _: () = assert!(std::mem::offset_of!(ListObj, ptr) == PTR as usize);
const _: () = assert!(std::mem::offset_of!(ListObj, len) == LEN as usize);
const _: () = assert!(std::mem::offset_of!(ListObj, cap) == CAP as usize);

/// A list: the header, then its items as a `Vec<u64>` taken apart.
#[repr(C)]
pub(crate) struct ListObj {
    head: Obj,
    pub(crate) ptr: *mut u64,
    pub(crate) len: usize,
    pub(crate) cap: usize,
}

/// Data objects (text, maps, …) come from the pool, in the size class of
/// fixed objects of the same size.
pub(crate) const DATA_CLASS: usize = (size_of::<WithData>() - FIELDS as usize) / 8;
const _: () = assert!(size_of::<WithData>() == FIELDS as usize + 8 * DATA_CLASS);
const _: () = assert!(DATA_CLASS <= crate::pool::MAX_SLOTS);
const _: () = assert!(align_of::<WithData>() <= 8, "the pool aligns to 8 bytes");
// One cache line: an object that straddles two lines costs two misses.
const _: () = assert!(
    size_of::<WithData>() == 64,
    "a data object fills one cache line"
);

/// An object with contents other than slots.
#[repr(C)]
pub(crate) struct WithData {
    head: Obj,
    data: RefCell<Data>,
}

#[derive(Clone)]
pub enum Data {
    /// Only slots.
    Slots,
    Str(crate::text::Text),
    /// Raw data (spec 2.13, `rt_bytes.rs`).
    Bytes(Vec<u8>),
    /// Raw data in whole words, its length in bytes: a big piece of a file
    /// (`rt_files.rs`), whose buffer can become a list's (`rt_bytes.rs`).
    /// Boxed, so `Data` fits in 32 bytes.
    Words(Box<(Vec<u64>, usize)>),
    /// A file read as a `for` loop goes (spec 14.3, `rt_files.rs`).
    Reader(crate::rt_files::Shared),
    /// A TCP listener or connection (spec 14.10, `rt_net.rs`): a
    /// `kavach_runtime_net::net::Shared`. Like `Regex`, an `Opaque`, so
    /// freeing an object doesn't link the network's code into every
    /// program (decision 0024).
    Socket(crate::opaque::Opaque),
    /// A present optional of a scalar type.
    Boxed(u64),
    /// Boxed: its 128-bit mantissa would make every object 16-byte
    /// aligned, which the pool doesn't give.
    Decimal(Box<Decimal>),
    /// A whole number of any size (spec 2.1), boxed as `Decimal` is.
    BigInt(Box<kavach_runtime_core::BigInt>),
    /// A `Decimal` too big for the form above (decision 0042).
    BigDecimal(Box<kavach_runtime_core::BigDecimal>),
    /// An error with text added in front (`context`, spec 5.7), holding
    /// the error it wraps. Boxed, so `Data` fits in 32 bytes.
    Context(Box<(String, u64)>),
    /// A map or set (see `table.rs`).
    Map(Box<crate::table::Table>),
    /// A compiled regular expression (spec 14.8): a
    /// `kavach_runtime_core::regex::Pattern`.
    Regex(crate::opaque::Opaque),
    /// A spawned task (spec 9.1, `rt_tasks.rs`).
    Task(std::sync::Arc<crate::rt_tasks::TaskObj>),
    /// A channel (spec 9.4, `rt_channels.rs`).
    Channel(std::sync::Arc<crate::rt_channels::ChanObj>),
    /// A `Shared` value's cell (spec 6.2): every copy holds the same.
    Shared(std::sync::Arc<crate::send::SharedCell>),
    /// A `Weak` (spec 6.2): the cell of a `Shared`, not kept alive.
    Weak(std::sync::Weak<crate::send::SharedCell>),
}

fn head(ty: Type, tag: u32, kind: u16, fixed_len: u16) -> Obj {
    Obj {
        rc: 1,
        tag,
        kind,
        fixed_len,
        type_id: match ty {
            Type::Ref(id) => id,
            _ => NOT_REF,
        },
        _pad: 0,
    }
}

pub(crate) fn fixed_layout(n: usize) -> Layout {
    Layout::from_size_align(size_of::<Obj>() + n * 8, align_of::<Obj>()).expect("a small object")
}

/// A tuple, record or union value with `n` zeroed slots after its header.
#[allow(unsafe_code)]
pub(crate) fn alloc_fixed(ty: Type, tag: u32, n: usize) -> i64 {
    count_live(1);
    let n16 = u16::try_from(n).expect("at most 65535 fields");
    let layout = fixed_layout(n);
    // SAFETY: small objects come from the pool, others from the system
    // allocator (the layout has a non-zero size: the header). The header
    // is written and the slots zeroed before the pointer is used.
    unsafe {
        let p = if n <= crate::pool::MAX_SLOTS {
            crate::pool::take(n, layout.size())
        } else {
            std::alloc::alloc(layout)
        } as *mut Obj;
        if p.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        p.write(head(ty, tag, FIXED, n16));
        std::ptr::write_bytes((p as *mut u8).add(FIELDS as usize), 0, n * 8);
        p as i64
    }
}

pub(crate) fn new_obj(ty: Type, tag: u32, slots: Vec<u64>, data: Data) -> i64 {
    if !matches!(data, Data::Slots) {
        count_live(1);
        // SAFETY: the pool gives `size_of::<WithData>()` bytes aligned to
        // 8, which are filled (header, then contents) before the pointer
        // is used. Written in place: building the object first and copying
        // it would read back half-finished stores.
        unsafe {
            let p = crate::pool::take(DATA_CLASS, size_of::<WithData>()) as *mut WithData;
            std::ptr::addr_of_mut!((*p).head).write(head(ty, tag, DATA, 0));
            std::ptr::addr_of_mut!((*p).data).write(RefCell::new(data));
            return p as i64;
        }
    }
    if matches!(shape(ty), Shape::List(_)) {
        // Packed items (decisions 0030 and 0045): `slots` came in one
        // item per word, as every list literal does.
        if let size @ 1.. = crate::flat::item_size(ty) {
            let n = slots.len();
            return new_list_raw(ty, tag, crate::flat::pack(&slots, size), n);
        }
        // A flat list's words, `width` per item.
        count_live(1);
        let width = crate::flat::width(ty).max(1);
        let mut slots = std::mem::ManuallyDrop::new(crate::flat::fit(slots, width));
        let o = Box::new(ListObj {
            head: head(ty, tag, LIST, 0),
            ptr: slots.as_mut_ptr(),
            len: slots.len() / width,
            cap: slots.capacity() / width,
        });
        return Box::into_raw(o) as i64;
    }
    let p = alloc_fixed(ty, tag, slots.len());
    slots_mut(p).copy_from_slice(&slots);
    p
}

/// An object whose contents are `data` (text, decimals, maps, …).
pub fn alloc(ty: Type, data: Data) -> i64 {
    new_obj(ty, 0, Vec::new(), data)
}

/// A list, tuple, record (`tag` 0) or union variant, with these slots.
/// The items of a flat list are objects, whose fields move into it.
pub fn alloc_slots(ty: Type, tag: u32, slots: Vec<u64>) -> i64 {
    match crate::flat::width(ty) {
        0 => new_obj(ty, tag, slots, Data::Slots),
        w => new_obj(ty, tag, crate::flat::unbox_all(slots, w), Data::Slots),
    }
}

/// A list made from its buffer as it is: its items, or a flat list's
/// words.
pub fn alloc_words(ty: Type, words: Vec<u64>) -> i64 {
    new_obj(ty, 0, words, Data::Slots)
}

/// A list (of `n` items) or another object with `n` slots, all 0.
pub fn alloc_zeroed(ty: Type, tag: u32, n: usize) -> i64 {
    if matches!(shape(ty), Shape::List(_)) {
        let width = crate::flat::width(ty).max(1);
        return new_obj(ty, tag, vec![0; n * width], Data::Slots);
    }
    alloc_fixed(ty, tag, n)
}

/// The object behind a non-null slot.
#[allow(unsafe_code)]
pub fn obj<'a>(p: i64) -> &'a Obj {
    // SAFETY: native code only passes pointers made by `new_obj` that are
    // still alive: generated code keeps a reference for every pointer it
    // holds (retain on copy, release when done), so the count is above 0.
    unsafe { &*(p as *const Obj) }
}

#[allow(unsafe_code)]
pub(crate) fn obj_mut<'a>(p: i64) -> &'a mut Obj {
    // SAFETY: as for `obj`; native code runs on one thread, and the
    // runtime never keeps another reference to the object while it changes.
    unsafe { &mut *(p as *mut Obj) }
}

#[allow(unsafe_code)]
pub(crate) fn list_mut<'a>(p: i64) -> &'a mut ListObj {
    assert_eq!(obj(p).kind, LIST, "a list");
    // SAFETY: objects of kind LIST are made as a `ListObj` (`new_obj`).
    unsafe { &mut *(p as *mut ListObj) }
}

thread_local! {
    /// What `data` gives for an object that has only slots.
    static NO_DATA: &'static RefCell<Data> = Box::leak(Box::new(RefCell::new(Data::Slots)));
}

/// The contents of an object (`Data::Slots` when it has only slots).
#[allow(unsafe_code)]
pub fn data<'a>(p: i64) -> &'a RefCell<Data> {
    if obj(p).kind != DATA {
        return NO_DATA.with(|d| *d);
    }
    // SAFETY: objects of kind DATA are made as a `WithData` (`new_obj`).
    unsafe { &(*(p as *const WithData)).data }
}

/// Where an object's slots are, and how many (for a flat list, the words
/// of all its items).
#[allow(unsafe_code)]
pub(crate) fn slot_area(p: i64) -> (*mut u64, usize) {
    let o = obj(p);
    match o.kind {
        LIST if crate::flat::packed(o.ty()) => {
            let l = list_mut(p);
            (
                l.ptr,
                crate::flat::words_for(l.len, crate::flat::item_size(o.ty())),
            )
        }
        LIST => {
            let l = list_mut(p);
            (l.ptr, l.len * crate::flat::width(o.ty()).max(1))
        }
        // SAFETY: a fixed object's slots follow its header (`alloc_fixed`).
        FIXED => (
            unsafe { (p as *mut u8).add(FIELDS as usize) } as *mut u64,
            o.fixed_len as usize,
        ),
        _ => (std::ptr::NonNull::dangling().as_ptr(), 0),
    }
}

/// The slots of an object (borrowed).
#[allow(unsafe_code)]
pub fn slots<'a>(p: i64) -> &'a [u64] {
    let (ptr, len) = slot_area(p);
    // SAFETY: `slot_area` gives the object's live slots.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

/// The slots of an object, to change in place.
#[allow(unsafe_code)]
pub fn slots_mut<'a>(p: i64) -> &'a mut [u64] {
    let (ptr, len) = slot_area(p);
    // SAFETY: as for `slots`; nothing else reads them while they change.
    unsafe { std::slice::from_raw_parts_mut(ptr, len) }
}

pub fn tag_of(p: i64) -> u32 {
    obj(p).tag
}

#[allow(unsafe_code)]
/// A packed list (decision 0030) straight from its words: `len` counts
/// items (bytes), the words hold them eight to one.
pub(crate) fn new_list_raw(ty: Type, tag: u32, mut words: Vec<u64>, len: usize) -> i64 {
    count_live(1);
    // Zero-filled to capacity, the invariant `with_packed` keeps.
    words.resize(words.capacity(), 0);
    let mut words = std::mem::ManuallyDrop::new(words);
    let o = Box::new(ListObj {
        head: head(ty, tag, LIST, 0),
        ptr: words.as_mut_ptr(),
        len,
        cap: words.len() * 8 / crate::flat::item_size(ty),
    });
    Box::into_raw(o) as i64
}

/// How many items a list holds (its `len` field counts items).
pub(crate) fn list_len_items(p: i64) -> usize {
    list_mut(p).len
}

/// `list_len_items`, for the other runtime modules.
pub fn list_len_items_pub(p: i64) -> usize {
    list_len_items(p)
}
