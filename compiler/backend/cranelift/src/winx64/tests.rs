//! The codes `unwind_info` writes, undone as Windows' unwinder does
//! (`RtlVirtualUnwind`) on a model stack: every saved register must come
//! back from its own slot, and the stack must end above the return
//! address.

use super::*;
use cranelift_codegen::ir::{AbiParam, InstBuilder, UserFuncName, types};
use cranelift_codegen::isa::unwind::UnwindInst;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;

/// A function that keeps eight values alive across a call, so it saves
/// callee-saved registers: its prologue as Cranelift describes it.
fn prologue_of_a_busy_function() -> Vec<(u32, UnwindInst)> {
    let isa = crate::new_isa(false).unwrap();
    let builder = ObjectBuilder::new(isa, "t", default_libcall_names()).unwrap();
    let mut module = ObjectModule::new(builder);
    let mut sig = module.make_signature();
    for _ in 0..4 {
        sig.params.push(AbiParam::new(types::I64));
    }
    sig.returns.push(AbiParam::new(types::I64));
    let callee = module.declare_function("g", Linkage::Import, &sig).unwrap();
    let f = module.declare_function("f", Linkage::Export, &sig).unwrap();
    let mut ctx = module.make_context();
    ctx.func.signature = sig;
    ctx.func.name = UserFuncName::user(0, f.as_u32());
    let mut fctx = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
    let block = b.create_block();
    b.append_block_params_for_function_params(block);
    b.switch_to_block(block);
    let p = b.block_params(block).to_vec();
    let mut live: Vec<_> = p.clone();
    for (i, v) in p.iter().enumerate() {
        live.push(b.ins().imul_imm(*v, i as i64 + 3));
    }
    let g = module.declare_func_in_func(callee, b.func);
    let call = b.ins().call(g, &p);
    let mut sum = b.inst_results(call)[0];
    for v in live {
        sum = b.ins().iadd(sum, v);
    }
    b.ins().return_(&[sum]);
    b.seal_all_blocks();
    b.finalize();
    module.define_function(f, &mut ctx).unwrap();
    let code = ctx.compiled_code().unwrap();
    code.buffer.unwind_info.to_vec()
}

/// Undoes `info` from the function's body: the registers the stack gives
/// back, and where the stack pointer ends. `rbp` is the frame pointer;
/// `memory` the stack's words.
fn undo(info: &[u8], rbp: u64, memory: &HashMap<u64, u64>) -> (HashMap<u8, u64>, u64) {
    let read = |at: u64| *memory.get(&at).unwrap_or_else(|| panic!("read {at:#x}"));
    let (count, frame) = (info[2] as usize, info[3]);
    assert_eq!(frame & 15, RBP, "the frame register");
    let base = rbp - 16 * u64::from(frame >> 4);
    let slot = |i: usize| u16::from_le_bytes([info[4 + 2 * i], info[5 + 2 * i]]);
    let (mut regs, mut rsp, mut i) = (HashMap::new(), rbp, 0);
    while i < count {
        let (op, reg) = (info[5 + 2 * i] & 15, info[5 + 2 * i] >> 4);
        match op {
            SAVE_NONVOL => {
                regs.insert(reg, read(base + 8 * u64::from(slot(i + 1))));
                i += 2;
            }
            SET_FPREG => {
                rsp = base;
                i += 1;
            }
            ALLOC_SMALL => {
                rsp += 8 * u64::from(reg) + 8;
                i += 1;
            }
            ALLOC_LARGE if reg == 0 => {
                rsp += 8 * u64::from(slot(i + 1));
                i += 2;
            }
            PUSH_NONVOL => {
                regs.insert(reg, read(rsp));
                rsp += 8;
                i += 1;
            }
            _ => panic!("an unexpected code {op}"),
        }
    }
    (regs, rsp)
}

#[test]
fn every_saved_register_comes_back_from_its_own_slot() {
    let insts = prologue_of_a_busy_function();
    let info = unwind_info(&insts).expect("codes for a usual prologue");
    // Where Cranelift saved each register: below the frame pointer.
    let rbp = 0x10_0000u64;
    let mut memory = HashMap::from([(rbp, 0xbb)]);
    let mut saved = HashMap::new();
    let mut down = 0;
    for (_, inst) in &insts {
        match inst {
            UnwindInst::DefineNewFrame {
                offset_downward_to_clobbers,
                ..
            } => down = u64::from(*offset_downward_to_clobbers),
            UnwindInst::SaveReg {
                clobber_offset,
                reg,
            } => {
                let marker = 0x1000 + u64::from(reg.hw_enc());
                memory.insert(rbp - down + u64::from(*clobber_offset), marker);
                saved.insert(reg.hw_enc(), marker);
            }
            _ => {}
        }
    }
    assert!(
        saved.len() >= 2,
        "a busy function saves registers: {insts:?}"
    );
    let (regs, rsp) = undo(&info, rbp, &memory);
    for (reg, marker) in &saved {
        assert_eq!(regs.get(reg), Some(marker), "register {reg}");
    }
    assert_eq!(regs.get(&RBP), Some(&0xbb), "the caller's frame pointer");
    assert_eq!(rsp, rbp + 8, "the return address is next");
}

#[test]
fn a_frame_with_nothing_saved_has_no_allocation_to_undo() {
    let insts = [
        (
            1,
            UnwindInst::PushFrameRegs {
                offset_upward_to_caller_sp: 16,
            },
        ),
        (
            4,
            UnwindInst::DefineNewFrame {
                offset_upward_to_caller_sp: 16,
                offset_downward_to_clobbers: 0,
            },
        ),
    ];
    let info = unwind_info(&insts).unwrap();
    assert_eq!(
        info,
        [1, 4, 2, RBP, 4, SET_FPREG, 1, (RBP << 4) | PUSH_NONVOL]
    );
}
