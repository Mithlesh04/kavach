//! `std.term`'s prompts (spec 14.13) answered through a pipe, in the
//! interpreter and native code: each question is printed before the
//! program reads its answer.

use std::io::Write;
use std::process::{Command, Stdio};

const PROGRAM: &str = "use std.term

fn main() {
    const name = term.prompt(\"Name?\") ?? \"nobody\"
    print(\"hi {name}\")
    print(term.confirm(\"Sure?\", default: false))
    print(term.confirm(\"Really?\", default: true))
    print(term.choose(\"Color?\", [\"red\", \"green\"]) ?? -1)
    print(term.choose(\"Again?\", [\"red\", \"green\"]) ?? -1)
    print(term.prompt_int(\"Age?\") ?? -1)
    print(term.prompt_float(\"Height?\") ?? -1.0)
    const s = term.prompt_secret(\"Password?\") ?? Secret(text: \"\")
    print(s)
    print(reveal(s.text).len())
    print(term.prompt(\"More?\") ?? \"end\")
}
";

#[test]
fn prompts_read_answers_from_standard_input() {
    let dir = std::env::temp_dir().join(format!("kavach-prompts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("ask.kav");
    std::fs::write(&file, PROGRAM).unwrap();
    for native in [false, true] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.env("KAVACH_CHECK_LEAKS", "1").arg("run");
        if native {
            cmd.arg("--native");
        }
        let mut child = cmd
            .arg(&file)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // A name, a yes, an empty line (the default), option 2, a number
        // out of range, an age that isn't one and then is, a height; then
        // the input ends.
        let mut input = child.stdin.take().unwrap();
        input
            .write_all(b"Asha\nyes\n\n2\n7\nold\n12\n1.5\nhunter22\n")
            .unwrap();
        drop(input);
        let out = child.wait_with_output().unwrap();
        let printed = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            printed,
            "Name? hi Asha\nSure? [y/N] true\nReally? [Y/n] true\n\
             Color?\n  1. red\n  2. green\nChoose 1-2: 1\n\
             Again?\n  1. red\n  2. green\nChoose 1-2: -1\n\
             Age? Please type a whole number.\nAge? 12\nHeight? 1.5\n\
             Password? Secret(text: <secret>)\n8\nMore? end\n",
            "native: {native}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_old_ask_names_say_the_new_ones() {
    // Decision 0053: `ask` is the AI's call; the terminal's is `prompt`.
    let dir = std::env::temp_dir().join(format!("kavach-old-ask-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("old.kav");
    std::fs::write(
        &file,
        "use std.term

fn main() {
    print(term.ask(\"Name?\") ?? \"nobody\")
}
",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("check")
        .arg(&file)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let said =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{said}");
    assert!(said.contains("`term.ask` is now `term.prompt`"), "{said}");
}
