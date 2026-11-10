//! Compiles the starter of packed programs (`starter/starter.c`, with the
//! vendored tinf and xz-embedded decoders) into a small executable that
//! the library embeds (`STARTER`, decision 0081), with the C compiler
//! Rust's own builds need.

use std::path::PathBuf;

fn main() {
    let here = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets it"));
    let tinf = here.join("vendor/tinf");
    let xz = here.join("vendor/xz-embedded");
    let sources = [
        here.join("starter/starter.c"),
        tinf.join("tinflate.c"),
        xz.join("xz_crc32.c"),
        xz.join("xz_dec_lzma2.c"),
        xz.join("xz_dec_stream.c"),
    ];
    println!("cargo:rerun-if-changed=starter");
    println!("cargo:rerun-if-changed=vendor");

    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|e| e == "msvc");
    let macos = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|o| o == "macos");
    let tool = cc::Build::new()
        .static_crt(true)
        .cargo_metadata(false)
        .get_compiler();
    let mut cmd = tool.to_command();
    let exe = out.join(if msvc { "starter.exe" } else { "starter" });
    if msvc {
        // Small code, no stack cookies; the C runtime's start linked in,
        // the Universal CRT used from Windows (every Windows from 10 on
        // has it), as `kavach build`'s programs do (decision 0016).
        cmd.args([
            "/nologo",
            "/O1",
            "/GS-",
            "/std:c11",
            "/DXZ_DEC_SINGLE",
            "/W3",
        ]);
        cmd.arg(format!("/I{}", tinf.display()));
        cmd.arg(format!("/I{}", xz.display()));
        cmd.arg(format!("/Fo{}\\", out.display()));
        cmd.arg(format!("/Fe{}", exe.display()));
        cmd.args(&sources);
        cmd.args([
            "/link",
            "/NODEFAULTLIB:libucrt.lib",
            "ucrt.lib",
            "/OPT:REF",
            "/OPT:ICF",
        ]);
    } else {
        cmd.args([
            "-Os",
            "-std=c99",
            "-DXZ_DEC_SINGLE",
            "-ffunction-sections",
            "-fdata-sections",
        ]);
        cmd.arg("-I").arg(&tinf).arg("-I").arg(&xz);
        cmd.args(&sources).arg("-o").arg(&exe);
        cmd.arg(if macos {
            "-Wl,-dead_strip"
        } else {
            "-Wl,--gc-sections,-s"
        });
    }
    let output = cmd.output().expect("the C compiler runs");
    if !output.status.success() {
        panic!(
            "compiling the starter of packed programs failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    std::fs::copy(&exe, out.join("starter.bin")).expect("the starter was written");
}
