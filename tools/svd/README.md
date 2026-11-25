# tools/svd (`kavach-svd`)

The tool behind `kavach svd chip.svd` (spec [6.5](../../docs/spec/06-memory.md#65-hardware-registers-volatile-memory), [decision 0145](../../docs/decisions/0145-volatile-pointer-access.md)). It reads a CMSIS-SVD file, the XML a chip vendor writes to describe its registers, and prints a Kavach module: for each peripheral a constant with its address and a type with a method for each register and field. Registers are read and written with volatile operations; fields are masked and shifted from them.

It reads the XML itself, so it needs no outside crate. Clusters and enumerated values aren't read yet: the module names each one it skipped in a comment.

| File | Job |
|---|---|
| `src/lib.rs` | `generate`: the SVD file's text in, the module's text out |
| `src/xml.rs` | A small XML reader: elements, attributes, text, comments, CDATA and entities |
| `src/model.rs` | The device, peripherals, registers and fields, with sizes, access and reset values passed down, and `dim` registers made one for each index |
| `src/emit.rs` | The Kavach module |
| `src/names.rs` | SVD names in Kavach's styles: `GPIOA` is `Gpioa`, `MODER` is `moder` |
| `tests/chip.svd` | A small chip written by hand: one register of each kind, and the parts the tool skips |
| `tests/generate.rs` | The module made from it compiles, each field's mask and shift are its bits, and a wrong file says what is wrong |

The module running on memory of its own, natively, is tested in [tools/cli/tests/registers.rs](../cli/tests/registers.rs).
