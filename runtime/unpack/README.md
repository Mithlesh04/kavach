# runtime/unpack

Packed executables ([spec 10.5](../../docs/spec/10-modules-and-packages/05-command-line.md), [decision 0081](../../docs/decisions/0081-packed-executables.md)): what `kavach build --compress` and `--compress=max` write, and the small starter in front of every packed program.

| File | What it holds |
|---|---|
| [src/lib.rs](src/lib.rs) | The format (starter, compressed program, 40-byte trailer), `packed` for `kavach build`, `unpack` for `kavach version`, and the embedded starter `STARTER` |
| [src/tests.rs](src/tests.rs) | Each method gives back the same bytes; a file without a trailer isn't packed; a damaged program is refused |
| [starter/starter.c](starter/starter.c) | The starter, in C so it stays small: unpacks the program once into the cache folder and runs the copy |
| [build.rs](build.rs) | Compiles the starter with the system's C compiler (MSVC on Windows, `cc` elsewhere) for the library to embed |
| [vendor/](vendor/README.md) | The two decoders the starter uses, copied unchanged from their projects |

The behaviour of packed programs, compared with plain ones on every run, is tested in [tools/cli/tests/packed.rs](../../tools/cli/tests/packed.rs).
