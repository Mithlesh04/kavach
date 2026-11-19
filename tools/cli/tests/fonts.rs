// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Fonts (spec 14.31): a font file read, text shaped into placed glyphs
//! and drawn into a distance atlas, the same interpreted and in native
//! code. The font is `runtime/font/src/sample.ttf`, made by our own code:
//! `A` a square, ` ` a space, `B` a square with a hole.

use std::process::Command;

const PROGRAM: &str = "use std.font
use std.fs

fn main() ! Error uses Fs {
    const f = try font.load(try fs.read_bytes(\"sample.ttf\"))
    print(`{f.ascent} {f.descent} {f.line_gap}`)
    const line = f.shape(\"AB A\", 100.0)
    for g in line.glyphs {
        print(`{g.id} {g.x} {g.y}`)
    }
    print(line.width)
    const atlas = f.atlas(\"A BA\", 64.0)
    print(`{atlas.image.width} {atlas.image.height} {atlas.spread}`)
    for (id, b) in atlas.boxes {
        print(`{id}: {b.x} {b.y} {b.width} {b.height} {b.left} {b.top}`)
        // Deep inside the square, and in the hole of `B`.
        print(atlas.image.pixel(b.x + 24, b.y + 30))
    }
}
";

const EXPECTED: &str = "0.8 -0.2 0.1
1 0.0 0.0
3 70.0 0.0
2 140.0 0.0
1 170.0 0.0
240.0
128 61 8.0
1: 0 0 49 61 -2 53
(255, 255, 255, 255)
3: 50 0 49 61 -2 53
(27, 27, 27, 27)
";

#[test]
fn text_is_shaped_and_drawn_into_an_atlas() {
    let font = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../runtime/font/src/sample.ttf"
    );
    for native in [false, true] {
        let dir =
            std::env::temp_dir().join(format!("kavach-fonts-{native}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.kav"), PROGRAM).unwrap();
        std::fs::copy(font, dir.join("sample.ttf")).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd.arg("main.kav").current_dir(&dir).output().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "native {native}: {stderr}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            EXPECTED,
            "native {native}"
        );
    }
}
