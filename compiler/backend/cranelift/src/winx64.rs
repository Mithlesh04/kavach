//! A function's unwind codes for Windows x64, written from Cranelift's
//! description of its prologue (`buffer.unwind_info`) instead of through
//! Cranelift's own encoder. That encoder (0.125, and still in 0.137) has
//! two mistakes: it divides an integer register's save offset by 16 where
//! Windows counts in 8 bytes, and it shifts the frame offset in bytes
//! where Windows counts in 16, so it overflows to 0. Windows then reads
//! saved registers from the wrong slots when a panic unwinds through
//! Kavach code, and the function that catches it goes on with them.
//!
//! Cranelift's prologue is `push rbp; mov rbp, rsp; sub rsp, n`, then the
//! callee-saved registers stored upward from `rbp - d` (the save area of
//! `d` bytes). Windows reads saved registers upward from `rbp - 16 *
//! FrameOffset`, so FrameOffset is `d / 16`; undoing the frame register
//! leaves the stack pointer there, so an allocation of `d` comes next,
//! which brings it back to `rbp`, where `push rbp` is undone.

use cranelift_codegen::isa::unwind::UnwindInst;

const RBP: u8 = 5;
const PUSH_NONVOL: u8 = 0;
const ALLOC_LARGE: u8 = 1;
const ALLOC_SMALL: u8 = 2;
const SET_FPREG: u8 = 3;
const SAVE_NONVOL: u8 = 4;
const SAVE_NONVOL_FAR: u8 = 5;
const SAVE_XMM128: u8 = 8;
const SAVE_XMM128_FAR: u8 = 9;

/// The `UNWIND_INFO` bytes (version, prologue size, codes) for a function
/// whose prologue Cranelift describes as `insts`; `None` when the save
/// area is too large for a frame offset (more than 240 bytes, which the
/// registers Windows saves never need).
pub(crate) fn unwind_info(insts: &[(u32, UnwindInst)]) -> Option<Vec<u8>> {
    // Each code in prologue order: where it ends, and its 2-byte slots.
    let mut codes: Vec<(u8, Vec<[u8; 2]>)> = Vec::new();
    let mut frame_offset = 0u8;
    let mut prologue = 0u8;
    for (at, inst) in insts {
        let at = u8::try_from(*at).ok()?;
        prologue = prologue.max(at);
        let op = |code: u8, info: u8| [at, (info << 4) | code];
        match inst {
            UnwindInst::PushFrameRegs { .. } => codes.push((at, vec![op(PUSH_NONVOL, RBP)])),
            UnwindInst::DefineNewFrame {
                offset_downward_to_clobbers: d,
                ..
            } => {
                if d % 16 != 0 || *d > 240 {
                    return None;
                }
                frame_offset = (*d / 16) as u8;
                // In prologue order: the allocation, then the frame register.
                if *d > 0 {
                    codes.push((at, alloc(at, *d)));
                }
                codes.push((at, vec![op(SET_FPREG, 0)]));
            }
            UnwindInst::StackAlloc { size } => codes.push((at, alloc(at, *size))),
            UnwindInst::SaveReg {
                clobber_offset,
                reg,
            } => {
                // Cranelift doesn't export the class's type, only its name.
                let (near, far, scale) = match format!("{:?}", reg.class()).as_str() {
                    "Int" => (SAVE_NONVOL, SAVE_NONVOL_FAR, 8),
                    _ => (SAVE_XMM128, SAVE_XMM128_FAR, 16),
                };
                let reg = reg.hw_enc();
                let scaled = clobber_offset / scale;
                codes.push(match u16::try_from(scaled) {
                    Ok(s) if clobber_offset % scale == 0 => {
                        (at, vec![op(near, reg), s.to_le_bytes()])
                    }
                    _ => {
                        let b = clobber_offset.to_le_bytes();
                        (at, vec![op(far, reg), [b[0], b[1]], [b[2], b[3]]])
                    }
                });
            }
            _ => return None,
        }
    }
    // Windows reads the codes from the end of the prologue back.
    let slots: Vec<[u8; 2]> = codes.into_iter().rev().flat_map(|(_, s)| s).collect();
    let mut out = vec![
        1,
        prologue,
        u8::try_from(slots.len()).ok()?,
        (frame_offset << 4) | RBP,
    ];
    for s in &slots {
        out.extend_from_slice(s);
    }
    // The codes take an even number of slots.
    if slots.len() % 2 == 1 {
        out.extend_from_slice(&[0, 0]);
    }
    Some(out)
}

#[cfg(test)]
mod tests;

/// The slots of an allocation of `size` bytes, ending at `at`.
fn alloc(at: u8, size: u32) -> Vec<[u8; 2]> {
    match size {
        8..=128 => vec![[at, ((((size - 8) / 8) as u8) << 4) | ALLOC_SMALL]],
        _ => match u16::try_from(size / 8) {
            Ok(s) => vec![[at, ALLOC_LARGE], s.to_le_bytes()],
            Err(_) => {
                let b = size.to_le_bytes();
                vec![[at, (1 << 4) | ALLOC_LARGE], [b[0], b[1]], [b[2], b[3]]]
            }
        },
    }
}
