// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Hardware registers (spec 6.5), on ordinary memory: volatile reads and
//! writes give the right values in native code, as `kavach run --native`
//! (Cranelift) and as an executable from `kavach build` (LLVM when its
//! tools are installed); and a module from `kavach svd` sets and reads
//! fields with the right masks and shifts. That LLVM's optimizer keeps
//! every volatile access is tested in `compiler/backend/llvm`.

use std::path::PathBuf;
use std::process::Command;

fn kavach() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kavach"))
}

/// Runs the package made of `files` (it allows `unsafe`) natively and as
/// a built executable: each must print `expected`. Gives what the native
/// run said on standard error, with the variables `env` set.
fn native(name: &str, files: &[(&str, &str)], env: &[(&str, &str)], expected: &str) -> String {
    let dir = std::env::temp_dir().join(format!("kavach-reg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let toml = format!("[package]\nname = \"{name}\"\nunsafe = true\n");
    std::fs::write(dir.join("kavach.toml"), toml).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join("src").join(file), text).unwrap();
    }
    let file = dir.join("src/main.kav");
    let jit = kavach()
        .envs(env.iter().copied())
        .args(["run", "--native"])
        .arg(&file)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&jit.stderr).to_string();
    assert_eq!(String::from_utf8_lossy(&jit.stdout), expected, "{said}");
    let exe = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    let built = kavach()
        .arg("build")
        .arg(&file)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(&exe).output().unwrap();
    let ran_said = String::from_utf8_lossy(&ran.stderr);
    assert_eq!(String::from_utf8_lossy(&ran.stdout), expected, "{ran_said}");
    std::fs::remove_dir_all(&dir).unwrap();
    said
}

/// Each kind of value read and written volatile, a record field by
/// field, a loop of writes, and a register reached by its address.
const VOLATILE: &str = "type Pair {
    a: Int32
    b: Float
}

fn main() {
    unsafe {
        const p: Ptr<UInt32> = alloc(2)
        p.write_volatile(7)
        (p + 1).write_volatile(8)
        print(p.read_volatile() + (p + 1).read_volatile())
        let n: UInt32 = 0
        while n < 1000 {
            p.write_volatile(n)
            n += 1
        }
        print(p.read(0))
        free(p)

        const f: Ptr<Float> = alloc(1)
        f.write_volatile(2.5)
        print(f.read_volatile())
        free(f)
        const g: Ptr<Float32> = alloc(1)
        g.write_volatile(0.25)
        print(g.read_volatile())
        free(g)
        const b: Ptr<Bool> = alloc(1)
        b.write_volatile(true)
        print(b.read_volatile())
        free(b)
        const r: Ptr<Pair> = alloc(1)
        r.write_volatile(Pair(a: 3, b: 0.5))
        print(`{r.read_volatile().a} {r.read(0).b}`)
        free(r)

        const regs: Ptr<UInt8> = alloc(8)
        const word: Ptr<UInt32> = Ptr.at(regs.address() + 4)
        word.write_volatile(0x0102_0304)
        print(`{regs.read(4)} {regs.read(7)}`)
        free(regs)
    }
}
";

#[test]
fn volatile_reads_and_writes_give_the_right_values() {
    let expected = "15\n999\n2.5\n0.25\ntrue\n3 0.5\n4 1\n";
    // Cranelift has no volatile flag: its atomics stand in (decision 0145).
    let said = native(
        "volatile",
        &[("main.kav", VOLATILE)],
        &[("KAVACH_CLIF", "1")],
        expected,
    );
    assert!(said.contains("atomic_store"), "{said}");
    assert!(said.contains("atomic_load"), "{said}");
}

fn chip_svd() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../svd/tests/chip.svd")
}

/// The module `kavach svd` makes for `tools/svd/tests/chip.svd`, given
/// memory of its own: each field set and read, and the words checked.
const FIELDS: &str = "use chip.{Gpioa, Uart1, gpioa_address}

fn main() {
    unsafe {
        const mem: Ptr<UInt8> = alloc(32)
        const words: Ptr<UInt32> = Ptr.at(mem.address())
        const gpio = Gpioa.at(mem)
        words.write(0, 0xa800_0000)
        gpio.set_moder_moder5(2)
        gpio.set_moder_moder0(3)
        print(`{words.read(0) == 0xa800_0803} {gpio.moder_moder5()} {gpio.moder_moder0()}`)
        words.write(4, 0x20)
        print(gpio.idr_idr5())
        words.write(5, 0xf00)
        gpio.set_odr_odr5(true)
        print(`{words.read(5)} {gpio.odr_in_()}`)
        gpio.set_odr_odr5(false)
        print(words.read(5))
        gpio.set_bsrr_br(5)
        print(words.read(6) == 0x5_0100)
        gpio.set_bsrr_bs5(true)
        print(words.read(6) == 0x120)
        free(mem)

        const mem2: Ptr<UInt8> = alloc(48)
        const uart = Uart1.at(mem2)
        uart.set_brr_div(0x123)
        uart.set_brr_frac(4)
        print(`{uart.brr() == 0x1234} {uart.brr_div() == 0x123}`)
        uart.set_dr_data(200)
        print(mem2.read(4))
        uart.set_stamp_high(7)
        print(`{uart.stamp() == 7 << 32} {uart.stamp_high()}`)
        uart.set_ch1_en(true)
        print(`{uart.ch0_en()} {uart.ch1_en()} {mem2.read(0x24)}`)
        free(mem2)
        print(gpioa_address == 0x4800_0000)
    }
}
";

#[test]
fn a_module_from_kavach_svd_sets_and_reads_fields() {
    let made = kavach().arg("svd").arg(chip_svd()).output().unwrap();
    let said = String::from_utf8_lossy(&made.stderr);
    assert!(made.status.success(), "{said}");
    let chip = String::from_utf8(made.stdout).unwrap();
    let expected =
        "true 2 3\ntrue\n3872 15\n3840\ntrue\ntrue\ntrue true\n200\ntrue 7\nfalse true 1\ntrue\n";
    let files = [("main.kav", FIELDS), ("chip.kav", chip.as_str())];
    native("fields", &files, &[], expected);
}

#[test]
fn kavach_svd_says_what_is_wrong_with_a_file() {
    let dir = std::env::temp_dir().join(format!("kavach-svd-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.svd");
    std::fs::write(&bad, "<device>\n  <peripherals>\n    <peripheral>\n").unwrap();
    let made = kavach().arg("svd").arg(&bad).output().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(made.status.code(), Some(2));
    let said = String::from_utf8_lossy(&made.stderr);
    assert!(
        said.contains("bad.svd: line 3: `<peripheral>` doesn't end"),
        "{said}"
    );
}
