//! Unwind tables in the object files `kavach build` writes (decision
//! 0065, step 1; decision 0074): what the system needs to walk back
//! through compiled Kavach code when a panic unwinds. Cranelift describes
//! each function's frame; this writes the descriptions as the platform
//! wants them: `.eh_frame` on Linux (through `gimli`), and `.pdata` with
//! `.xdata` on Windows. Nothing reads them yet: a panic still ends the
//! program (spec 9.1) until the runtime unwinds.

use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::isa::unwind::UnwindInfo;
use cranelift_module::FuncId;
use cranelift_object::ObjectProduct;
use cranelift_object::object::write::{Object, Relocation, SectionId, StandardSegment, SymbolId};
use cranelift_object::object::{RelocationEncoding, RelocationFlags, RelocationKind, SectionKind};
use gimli::write::{Address, EhFrame, EndianVec, FrameTable, Writer};

/// Each defined function's unwind description and code size, gathered as
/// the functions are compiled.
#[derive(Default)]
pub(crate) struct Unwinds(pub(crate) Vec<(FuncId, UnwindInfo, u32)>);

/// Adds the unwind tables for `unwinds` to `product`'s object file.
pub(crate) fn write(
    product: &mut ObjectProduct,
    isa: &dyn TargetIsa,
    unwinds: Unwinds,
) -> Result<(), String> {
    let functions: Vec<(SymbolId, UnwindInfo, u32)> = (unwinds.0.into_iter())
        .map(|(id, info, size)| (product.function_symbol(id), info, size))
        .collect();
    let obj = &mut product.object;
    if functions
        .iter()
        .any(|(_, i, _)| matches!(i, UnwindInfo::WindowsX64(_)))
    {
        windows(obj, &functions);
        return Ok(());
    }
    system_v(obj, isa, &functions)
}

/// `.eh_frame`: one CIE, and an FDE for each function, whose address is
/// relative to where it is written (PC-relative), so a
/// position-independent executable needs no fix-ups at load time.
fn system_v(
    obj: &mut Object<'static>,
    isa: &dyn TargetIsa,
    functions: &[(SymbolId, UnwindInfo, u32)],
) -> Result<(), String> {
    let Some(mut cie) = isa.create_systemv_cie() else {
        return Ok(());
    };
    cie.fde_address_encoding = gimli::DwEhPe(gimli::DW_EH_PE_pcrel.0 | gimli::DW_EH_PE_sdata4.0);
    let mut table = FrameTable::default();
    let cie = table.add_cie(cie);
    for (n, (_, info, _)) in functions.iter().enumerate() {
        if let UnwindInfo::SystemV(u) = info {
            let at = Address::Symbol {
                symbol: n,
                addend: 0,
            };
            table.add_fde(cie, u.to_fde(at));
        }
    }
    let mut eh = EhFrame(Relocating::default());
    table.write_eh_frame(&mut eh).map_err(|e| e.to_string())?;
    let Relocating { data, relocs } = eh.0;
    let segment = obj.segment_name(StandardSegment::Data).to_vec();
    let section = obj.add_section(segment, b".eh_frame".to_vec(), SectionKind::ReadOnlyData);
    obj.append_section_data(section, data.slice(), 8);
    for r in relocs {
        let kind = match r.pc_relative {
            true => RelocationKind::Relative,
            false => RelocationKind::Absolute,
        };
        relocate(
            obj,
            section,
            r.offset as u64,
            functions[r.symbol].0,
            r.addend,
            kind,
            r.size,
        );
    }
    Ok(())
}

/// `.xdata` with each function's unwind codes, and `.pdata` with its
/// start, end and codes, as image-relative addresses (Windows x64).
fn windows(obj: &mut Object<'static>, functions: &[(SymbolId, UnwindInfo, u32)]) {
    let segment = obj.segment_name(StandardSegment::Data).to_vec();
    let xdata = obj.add_section(
        segment.clone(),
        b".xdata".to_vec(),
        SectionKind::ReadOnlyData,
    );
    let pdata = obj.add_section(segment, b".pdata".to_vec(), SectionKind::ReadOnlyData);
    let codes = obj.section_symbol(xdata);
    for (symbol, info, size) in functions {
        let UnwindInfo::WindowsX64(w) = info else {
            continue;
        };
        let mut bytes = vec![0; w.emit_size()];
        w.emit(&mut bytes);
        let at = obj.append_section_data(xdata, &bytes, 4);
        let entry = obj.append_section_data(pdata, &[0; 12], 4);
        let image = RelocationKind::ImageOffset;
        relocate(obj, pdata, entry, *symbol, 0, image, 4);
        relocate(obj, pdata, entry + 4, *symbol, i64::from(*size), image, 4);
        relocate(obj, pdata, entry + 8, codes, at as i64, image, 4);
    }
}

fn relocate(
    obj: &mut Object<'static>,
    section: SectionId,
    offset: u64,
    symbol: SymbolId,
    addend: i64,
    kind: RelocationKind,
    bytes: u8,
) {
    let flags = RelocationFlags::Generic {
        kind,
        encoding: RelocationEncoding::Generic,
        size: bytes * 8,
    };
    let relocation = Relocation {
        offset,
        symbol,
        addend,
        flags,
    };
    // Every symbol here is one this object file defines.
    obj.add_relocation(section, relocation)
        .expect("a relocation the object file can hold");
}

/// A place in `.eh_frame` that holds a function's address.
struct Reloc {
    offset: usize,
    symbol: usize,
    addend: i64,
    size: u8,
    pc_relative: bool,
}

/// A `gimli` writer that leaves room for each address and notes where, so
/// the linker fills it in.
struct Relocating {
    data: EndianVec<gimli::LittleEndian>,
    relocs: Vec<Reloc>,
}

impl Default for Relocating {
    fn default() -> Relocating {
        Relocating {
            data: EndianVec::new(gimli::LittleEndian),
            relocs: Vec::new(),
        }
    }
}

impl Relocating {
    fn note(&mut self, address: Address, size: u8, pc_relative: bool) -> gimli::write::Result<()> {
        match address {
            Address::Constant(v) => self.write_udata(v, size),
            Address::Symbol { symbol, addend } => {
                self.relocs.push(Reloc {
                    offset: self.len(),
                    symbol,
                    addend,
                    size,
                    pc_relative,
                });
                self.write_udata(0, size)
            }
        }
    }
}

impl Writer for Relocating {
    type Endian = gimli::LittleEndian;

    fn endian(&self) -> Self::Endian {
        gimli::LittleEndian
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn write(&mut self, bytes: &[u8]) -> gimli::write::Result<()> {
        self.data.write(bytes)
    }

    fn write_at(&mut self, offset: usize, bytes: &[u8]) -> gimli::write::Result<()> {
        self.data.write_at(offset, bytes)
    }

    fn write_address(&mut self, address: Address, size: u8) -> gimli::write::Result<()> {
        self.note(address, size, false)
    }

    fn write_eh_pointer(
        &mut self,
        address: Address,
        eh_pe: gimli::DwEhPe,
        size: u8,
    ) -> gimli::write::Result<()> {
        let pc_relative = eh_pe.application() == gimli::DW_EH_PE_pcrel;
        // `size` is the address size; the encoding's format says how wide
        // the pointer really is (`sdata4`: 4 bytes).
        let width = match eh_pe.format() {
            gimli::DW_EH_PE_sdata2 | gimli::DW_EH_PE_udata2 => 2,
            gimli::DW_EH_PE_sdata4 | gimli::DW_EH_PE_udata4 => 4,
            gimli::DW_EH_PE_sdata8 | gimli::DW_EH_PE_udata8 => 8,
            _ => size,
        };
        self.note(address, width, pc_relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, InstBuilder, UserFuncName, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_module::{Linkage, Module, default_libcall_names};
    use cranelift_object::object::{Object as _, ObjectSection as _};
    use cranelift_object::{ObjectBuilder, ObjectModule};

    /// A function that calls another has a frame, and the object file
    /// gets its unwind entry: on Linux an FDE whose address the linker
    /// fills in; on Windows a `.pdata` entry (start, end, codes).
    #[test]
    fn a_compiled_function_gets_an_unwind_entry() {
        let isa = crate::new_isa(!cfg!(windows)).unwrap();
        let builder = ObjectBuilder::new(isa.clone(), "t", default_libcall_names()).unwrap();
        let mut module = ObjectModule::new(builder);
        let mut sig = module.make_signature();
        sig.returns.push(AbiParam::new(types::I64));
        let callee = module.declare_function("g", Linkage::Import, &sig).unwrap();
        let f = module.declare_function("f", Linkage::Export, &sig).unwrap();
        let mut ctx = module.make_context();
        ctx.func.signature = sig;
        ctx.func.name = UserFuncName::user(0, f.as_u32());
        let mut fctx = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
        let block = b.create_block();
        b.switch_to_block(block);
        let g = module.declare_func_in_func(callee, b.func);
        let call = b.ins().call(g, &[]);
        let v = b.inst_results(call)[0];
        b.ins().return_(&[v]);
        b.seal_all_blocks();
        b.finalize();
        module.define_function(f, &mut ctx).unwrap();
        let code = ctx.compiled_code().unwrap();
        let info = code.create_unwind_info(module.isa()).unwrap().unwrap();
        let mut unwinds = Unwinds::default();
        unwinds.0.push((f, info, code.code_info().total_size));
        let mut product = module.finish();
        write(&mut product, &*isa, unwinds).unwrap();
        let bytes = product.emit().unwrap();
        let file = cranelift_object::object::File::parse(&*bytes).unwrap();
        let section = |name: &str| file.section_by_name(name).unwrap();
        if cfg!(windows) {
            let pdata = section(".pdata");
            assert_eq!(pdata.size(), 12, "one entry: start, end, codes");
            assert_eq!(pdata.relocations().count(), 3);
            assert!(section(".xdata").size() > 0);
        } else {
            let eh = section(".eh_frame");
            assert_eq!(eh.relocations().count(), 1, "the FDE's start");
        }
    }
}
