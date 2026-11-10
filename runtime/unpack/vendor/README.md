# runtime/unpack/vendor

Third-party code the starter of packed programs uses ([decision 0081](../../../docs/decisions/0081-packed-executables.md)), copied unchanged; only the files the starter needs. The repository check doesn't count `vendor/` folders against the 500-line limit. To update one, copy the same files from a newer commit and run `cargo test -p kavach-runtime-unpack` and `cargo test -p kavach-cli --test packed`.

| Folder | Project | Commit | Licence | Files |
|---|---|---|---|---|
| `tinf/` | [tinf](https://github.com/jibsen/tinf), a small deflate decoder by Jørgen Ibsen | `57ffa1f1d5e3` (2022-02-12) | zlib ([tinf/LICENSE](tinf/LICENSE)) | `tinflate.c`, `tinf.h` |
| `xz-embedded/` | [XZ Embedded](https://github.com/tukaani-project/xz-embedded), the xz decoder for small systems | `ae63ae3a36ed` (2024-12-30) | 0BSD ([xz-embedded/COPYING](xz-embedded/COPYING)) | `xz.h` (from `linux/include/linux/`), `xz_config.h` (from `userspace/`), and `xz_private.h`, `xz_lzma2.h`, `xz_stream.h`, `xz_crc32.c`, `xz_dec_lzma2.c`, `xz_dec_stream.c` (from `linux/lib/xz/`) |
