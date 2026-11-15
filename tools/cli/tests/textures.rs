//! Textures (spec 15.3) on the CPU: `sample` gives what the GPU gives, a
//! color made linear from sRGB, blended between the nearest pixels,
//! interpreted and in native code alike. A machine with no GPU can't
//! make a texture, so there the test only checks that it says so.

use std::process::Command;

const SAMPLED: &str = "use std.gpu

fn main() ! Error uses Window {
    // A red pixel, then a blue one.
    const skin = try gpu.texture(2, 1, [255, 0, 0, 255, 0, 0, 255, 255])
    print(skin.sample(0.25, 0.5))
    print(skin.sample(0.5, 0.5))
    print(skin.sample(-3.0, 9.0))
    const grey = try gpu.texture(1, 1, [128, 128, 128, 255])
    const (r, _, _, _) = grey.sample(0.5, 0.5)
    print(r > 0.21 and r < 0.22)
}
";

fn run(native: bool) -> (String, String) {
    let dir = std::env::temp_dir().join(format!("kavach-textures-{native}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.kav");
    std::fs::write(&file, SAMPLED).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.arg("run");
    if native {
        cmd.arg("--native");
    }
    let out = cmd
        .arg(&file)
        .env("KAVACH_CHECK_LEAKS", "1")
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).to_string();
    (text(&out.stdout), text(&out.stderr))
}

#[test]
fn a_texture_samples_on_the_cpu_as_on_the_gpu() {
    for native in [false, true] {
        let (out, err) = run(native);
        if err.contains("Unavailable") {
            assert!(out.is_empty(), "{out}");
            continue;
        }
        assert_eq!(
            out, "(1.0, 0.0, 0.0, 1.0)\n(0.5, 0.0, 0.5, 1.0)\n(1.0, 0.0, 0.0, 1.0)\ntrue\n",
            "native: {native}; {err}"
        );
    }
}
