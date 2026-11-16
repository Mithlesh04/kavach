//! Calls into C that Cranelift's plain calls don't cover (spec 10.7):
//! through an address, for a library loaded while running, and of
//! functions that end in `...`, which Cranelift has no notion of.
//!
//! A call of a function ending in `...` gets a signature of its own, its own
//! arguments then the extra ones, and goes through the function's address,
//! as C's rules for each system ask:
//!
//! - **x86-64 System V** (Linux, macOS on Intel): the callee reads in `al`
//!   how many vector registers hold arguments. A thunk of our own sets
//!   `al` to 8, the most there can be, and jumps to the C function, so
//!   every register and the stack stay as the call left them.
//! - **Windows x64**: a callee reads its extra arguments from the integer
//!   registers' home slots, so a `double` among them goes in an integer
//!   register, as its bits.
//! - **AArch64 Linux**: extra arguments go as any others.

use crate::codegen::FnGen;
use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::ir::{AbiParam, InstBuilder, Value, types};
use cranelift_codegen::isa::CallConv;
use cranelift_module::{FuncId, Linkage, Module, ModuleReloc, ModuleRelocTarget};
use kavach_kir::Type;

/// Whether `...` calls can be made here; or why not.
pub(crate) fn supported(module: &dyn Module) -> Result<(), String> {
    let isa = module.isa();
    match (isa.name(), isa.default_call_conv()) {
        ("x64", CallConv::SystemV | CallConv::WindowsFastcall) => Ok(()),
        ("aarch64", CallConv::AppleAarch64) => {
            Err("a C function with `...` can't be called on macOS on ARM yet (spec 10.7)".into())
        }
        ("aarch64", _) => Ok(()),
        _ => Err("a C function with `...` can't be called on this machine yet (spec 10.7)".into()),
    }
}

/// What a program calls for C function `c`, which ends in `...`: on x86-64
/// System V a thunk that sets `al` and jumps to it; elsewhere `c` itself.
pub(crate) fn callee(module: &mut dyn Module, c: FuncId, name: &str) -> Result<FuncId, String> {
    let isa = module.isa();
    let system_v = isa.default_call_conv() == CallConv::SystemV && isa.name() == "x64";
    if !system_v {
        return Ok(c);
    }
    let pic = isa.flags().is_pic();
    let sig = module.declarations().get_function_decl(c).signature.clone();
    let thunk = module
        .declare_function(&format!("kv_va_{name}"), Linkage::Local, &sig)
        .map_err(|e| e.to_string())?;
    let target = ModuleRelocTarget::from(c);
    // `mov al, 8`, then a jump to `c`: through its PLT entry in an object
    // file, which a position-independent executable needs, or through
    // its address, which the JIT knows wherever `c` is.
    let (bytes, reloc): (Vec<u8>, _) = if pic {
        let bytes = vec![0xB0, 8, 0xE9, 0, 0, 0, 0];
        (bytes, (3, Reloc::X86CallPLTRel4, -4))
    } else {
        let mut bytes = vec![0xB0, 8, 0xFF, 0x25, 0, 0, 0, 0];
        bytes.extend([0; 8]);
        (bytes, (8, Reloc::Abs8, 0))
    };
    let relocs = [ModuleReloc {
        offset: reloc.0,
        kind: reloc.1,
        name: target,
        addend: reloc.2,
    }];
    module
        .define_function_bytes(thunk, 16, &bytes, &relocs)
        .map_err(|e| e.to_string())?;
    Ok(thunk)
}

impl FnGen<'_, '_> {
    /// Calls program function `f` with `values`, made from `args`: a C
    /// function given more than its own parameters by its own signature
    /// for this call (see the top of this file), any other as declared.
    pub(crate) fn call_fn(
        &mut self,
        f: u32,
        args: &[kavach_kir::Expr],
        values: &[Value],
    ) -> Vec<Value> {
        let id = self.funcs[f as usize];
        let Some(own) = self.variadic[f as usize] else {
            return self.call(id, values);
        };
        let windows = self.module.isa().default_call_conv() == CallConv::WindowsFastcall;
        let types: Vec<Type> = args.iter().map(|a| a.ty).collect();
        let decl = self
            .module
            .declarations()
            .get_function_decl(id)
            .signature
            .clone();
        let mut sig = crate::abi::c_signature(&*self.module, &types, Type::Unit);
        sig.returns = decl.returns;
        let mut values = values.to_vec();
        if windows {
            for (i, v) in values.iter_mut().enumerate().skip(own) {
                if types[i] == Type::F64 {
                    *v = self.b.ins().bitcast(
                        types::I64,
                        cranelift_codegen::ir::MemFlags::new(),
                        *v,
                    );
                    sig.params[i] = AbiParam::new(types::I64);
                }
            }
        }
        let fref = self.module.declare_func_in_func(id, self.b.func);
        let address = self.b.ins().func_addr(types::I64, fref);
        let sig = self.b.import_signature(sig);
        let inst = self.b.ins().call_indirect(sig, address, &values);
        self.b.inst_results(inst).to_vec()
    }

    /// A call of the C function at address `args[0]` with the other
    /// operands, giving `ty` (a library loaded while running).
    pub(crate) fn call_address(&mut self, ty: Type, args: &[kavach_kir::Expr]) -> Option<Value> {
        let address = self.expr(&args[0])?;
        let values: Vec<Value> = args[1..].iter().filter_map(|a| self.expr(a)).collect();
        let types: Vec<Type> = args[1..].iter().map(|a| a.ty).collect();
        let sig = crate::abi::c_signature(&*self.module, &types, ty);
        let sig = self.b.import_signature(sig);
        let inst = self.b.ins().call_indirect(sig, address, &values);
        self.b.inst_results(inst).first().copied()
    }

    /// A call into C with records by value (`RtOp::CallC`): `args[0]`
    /// says how each operand goes (`kavach_kir::c_abi::descriptor`),
    /// `args[1]` is the function's address. Result parts are stored where
    /// the `m` operand points.
    pub(crate) fn call_c(&mut self, ty: Type, args: &[kavach_kir::Expr]) -> Option<Value> {
        use cranelift_codegen::ir::{ArgumentPurpose, MemFlags};
        let kavach_kir::ExprKind::Str(text) = &args[0].kind else {
            unreachable!("a C call's descriptor is a text constant")
        };
        let (result, words) = text.split_once('|').unwrap_or(("v", ""));
        let target = self.expr(&args[1])?;
        let mut sig = self.module.make_signature();
        let (mut values, mut out) = (Vec::new(), None);
        for (word, a) in words.split(',').filter(|w| !w.is_empty()).zip(&args[2..]) {
            let v = self.expr(a)?;
            let size = word[1..].parse().unwrap_or(0);
            let param = match &word[..1] {
                "m" => {
                    out = Some(v);
                    continue;
                }
                "i" => AbiParam::new(types::I64),
                "f" => AbiParam::new(types::F64),
                "s" => AbiParam::special(types::I64, ArgumentPurpose::StructArgument(size)),
                "h" => AbiParam::special(types::I64, ArgumentPurpose::StructReturn),
                _ => crate::abi::c_signature(&*self.module, &[a.ty], Type::Unit).params[0],
            };
            sig.params.push(param);
            values.push(v);
        }
        match result {
            "v" => {}
            "n" => sig.returns = crate::abi::c_signature(&*self.module, &[], ty).returns,
            parts => sig.returns.extend(
                parts
                    .chars()
                    .map(|c| AbiParam::new(if c == 'f' { types::F64 } else { types::I64 })),
            ),
        }
        let sig = self.b.import_signature(sig);
        let inst = self.b.ins().call_indirect(sig, target, &values);
        let results = self.b.inst_results(inst).to_vec();
        match (result, out) {
            ("n", _) => results.first().copied(),
            (_, Some(out)) => {
                for (k, r) in results.into_iter().enumerate() {
                    self.b
                        .ins()
                        .store(MemFlags::trusted(), r, out, 8 * k as i32);
                }
                None
            }
            _ => None,
        }
    }
}
