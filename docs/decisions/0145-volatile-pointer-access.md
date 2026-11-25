# 0145. Volatile reads and writes through `Ptr`, and register blocks from SVD files

- **Status:** Accepted (points marked "for a maintainer's review" are open to change)
- **Date:** 2026-11-23

## Context

The [platform plan](../vision/platform.md) promises "memory-mapped registers with volatile reads and writes through typed wrappers". `Ptr` reads and writes (spec 6.4) are ordinary memory operations: the compiler may merge two reads of one address, drop a write that a later write replaces, or move them. For a device register that is wrong. Reading a status register can clear it, and each write to a data register sends a byte.

A maintainer chose the shape on 2026-11-15 (to-do 15):

1. In `unsafe`, `p.read_volatile()`, `p.write_volatile(v)` and `Ptr.at(address)`, never merged, dropped or reordered.
2. Registers mapped on Linux through UIO or VFIO, behind a device capability.
3. Typed register blocks generated from a vendor's SVD file by a tool, as `svd2rust` does, where only making the block is `unsafe`.

How others do it:

- **C** marks the pointer's type `volatile`. Every access through it is volatile, which is easy to forget on one copy of the pointer.
- **Rust** has `ptr::read_volatile` and `ptr::write_volatile`: the operation is volatile, not the type. `svd2rust` generates a type for each peripheral, a reader and writer for each register, and a method for each field.
- **Zig** has `*volatile T` pointers, as C does.

## Decision

Spec [6.5](../spec/06-memory.md#65-hardware-registers-volatile-memory).

1. **The operation is volatile, not the pointer**, as in Rust: `p.read_volatile()` and `p.write_volatile(v)` on any `Ptr<T>`, allowed where `Ptr` is (inside `unsafe`, native code only). One `Ptr` type stays; an access that must be volatile says so where it happens.
2. **What volatile means** is C's and LLVM's: each one happens once each time it is reached, in program order with the other volatile accesses. Ordinary reads and writes may move around them. Volatile is not atomic.
3. **`Ptr.at(address)`** is `Ptr.from_int` under a name that reads well for a register. *For a maintainer's review:* the two do the same; one could go.
4. **`p.address()`**, the address as an `Int`, was added: without it a block holding a `Ptr<UInt8>` couldn't reach a `UInt32` register at an offset, as Kavach has no pointer casts. *For a maintainer's review:* it is the smallest way; a cast (`p.as<UInt32>()`) is another.
5. **A record** is read or written field by field, each field one volatile access, in field order. *For a maintainer's review.*
6. **The address must be a multiple of the value's size.** Registers always are, and point 8 needs it.
7. **LLVM** gets `load volatile` and `store volatile`.
8. **Cranelift has no volatile flag.** Its alias analysis merges a load with an earlier load or store of the same address. Its atomic loads and stores are never merged or dropped, and nothing moves across them, so they stand in for volatile ones. On x86-64 an atomic load is a plain `mov`. An atomic store is a `mov` and a fence: slower, but Cranelift builds are for development. A `Float` goes through an integer of its size. *For a maintainer's review.*
9. **The interpreter** stops at `unsafe` code, as for every other `Ptr` operation (6.4).
10. **`kavach svd chip.svd`** prints a Kavach module: a type for each peripheral, a method for each register and field, and a constant with each peripheral's address. It is written in Rust (`tools/svd`) with its own small XML reader for the part of XML SVD files use, so it needs no outside crate.
11. **Only making a block is `unsafe`.** `Gpioa.at(p)` takes a `Ptr<UInt8>`, which only `unsafe` code can make. The methods use volatile operations inside `unsafe` blocks of their own, so the package needs `unsafe = true`. A test passes memory from `alloc` instead of the chip's address.
12. **A field setter reads, changes and writes back** the register, with a `require` that the value fits the field's bits. A one-bit field is a `Bool`. *For a maintainer's review:* `svd2rust` instead gathers several field changes into one write (`modify(|r, w| …)`); that can come later as a method taking several fields.
13. **Mapping registers through UIO or VFIO waits**: it needs a device to test on. It is listed in the chapter's open questions.

## Consequences

- Drivers in user space, and later on bare metal, can read and write registers correctly in both backends.
- A register's name, offset and bits come from the vendor's file, not from code copied out of a manual.
- A Cranelift build pays a fence for each volatile write.
- Clusters and enumerated values in SVD files are skipped, with a comment in the module for each.

## Alternatives considered

- **A `volatile` pointer type, as in C and Zig.** Two pointer types for one address, and a copy of a pointer into the other type loses the guarantee.
- **Runtime functions for Cranelift** (`read_volatile` in Rust, called from the generated code). Correct, but a call for each access where an atomic is one instruction.
- **Fences around ordinary loads in Cranelift.** A fence on every read too, where atomic loads need none on x86-64.
- **Writing the SVD tool in Kavach.** It would need an XML reader in Kavach first; Rust's tools are already where `kavach` is.
