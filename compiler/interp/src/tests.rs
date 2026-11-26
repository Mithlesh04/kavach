// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Tests of whole programs run by the interpreter: speed traps, and
//! what `kavach test` finds.

use std::time::{Duration, Instant};

/// Lists are shared until changed: `push` changes a variable's list in
/// place, and reading a list (`items.len()`, `items[i]`, passing it
/// to a function) doesn't copy it. Both used to copy the whole list,
/// so this program took minutes.
#[test]
fn lists_are_not_copied() {
    let src = "fn total(items: [Int]) -> Int {
    let sum = 0
    let i = 0
    while i < items.len() {
        sum += items[i]
        i += 1
    }
    sum
}

let items = []
for i in 0..100000 {
    items.push(i)
}
print(total(items))
";
    let compiled = kavach_driver::compile(src, "push.kav", Default::default());
    let start = Instant::now();
    let output = super::run(&compiled);
    assert_eq!(output.stdout, "4999950000\n");
    assert!(start.elapsed() < Duration::from_secs(10));
}

/// Finding a key in a map doesn't scan the map. It used to, so this
/// program took minutes.
#[test]
fn map_keys_are_indexed() {
    let src = "let m: {Int: Int} = {:}\nfor i in 0..50000 {\n    m[i] = i\n}\nlet total = 0\nfor i in 0..50000 {\n    total += m[i] ?? 0\n}\nprint(total)\n";
    let compiled = kavach_driver::compile(src, "map.kav", Default::default());
    let start = Instant::now();
    let output = super::run(&compiled);
    assert_eq!(output.stdout, "1249975000\n");
    assert!(start.elapsed() < Duration::from_secs(10));
}

/// A method of a generic type is skipped with that reason, not
/// reported as a `require` no input met (values of `Boxed<S>` aren't
/// generated).
#[test]
fn methods_of_generic_types_say_why_they_are_skipped() {
    let method = |t: &str| {
        format!(
            "    fn sized(self, n: Int) -> {t}
    require n > 0
    {{
        self with {{ size: n }}
    }}
"
        )
    };
    let src = format!(
        "type Boxed<S> {{
    state: S
    size: Int

{}}}

type Plain {{
    size: Int

{}}}
",
        method("Boxed<S>"),
        method("Plain")
    );
    let compiled = kavach_driver::compile(&src, "gen.kav", Default::default());
    let output = super::test(&compiled, &Default::default());
    assert!(
        output
            .stdout
            .contains("not generated: Boxed.sized: its type is generic"),
        "{}",
        output.stdout
    );
    assert!(output.stdout.contains("✓ Plain.sized"), "{}", output.stdout);
    assert!(
        !output.stdout.contains("no generated input"),
        "{}",
        output.stdout
    );
}

/// A value holding a `Shared` can't be generated, so a function taking
/// one is listed as not generated, not shown as passing with 0 cases;
/// one whose `require` no input met but whose examples pass says so.
#[test]
fn functions_that_get_no_generated_input_say_why() {
    let src = "type Keys {
    current: Shared<String>

    fn now(self) -> String {
        self.current.get()
    }
}

fn key(text: String) -> Int
    require text.starts_with(\"kv1:\") and text.len() == 6
    example key(\"kv1:ab\") == 6
{
    text.len()
}
";
    let compiled = kavach_driver::compile(src, "keys.kav", Default::default());
    let out = super::test(&compiled, &Default::default()).stdout;
    let skipped = "not generated: Keys.now: takes values that can't be generated yet";
    assert!(out.contains(skipped), "{out}");
    let examples = "✓ key  checked by its examples only: no generated input met its `require`";
    assert!(out.contains(examples), "{out}");
    assert!(!out.contains("0 cases"), "{out}");
}

/// A server's loop: each round spawns a task whose arguments and result
/// share text and lists with the spawner, waits for it or leaves its
/// error unread, and lets the task go when the function returns. The
/// task's thread must let go of its handle, and of every value, before
/// it hands the turn on: a value let go of after that changes `Rc`
/// counts while another task runs (`Baton` in `tasks.rs`), which the
/// handle's `Drop` checks in debug builds.
#[test]
fn tasks_let_their_values_go_before_the_turn_passes() {
    let src = "fn handle(name: String, items: [String], n: Int) -> [String] ! Error {
    if n % 3 == 0 {
        fail Error(name)
    }
    let out = items
    out.push(name)
    out
}

fn serve(name: String, items: [String], n: Int) -> Int {
    if n % 5 == 0 {
        const _ignored = spawn handle(name, items, n)
        return 0
    }
    const t = spawn handle(name, items, n)
    const got = t.wait() catch _ => [name]
    got.len()
}

fn main() {
    const name = \"request\"
    const items = [\"a\", \"b\", \"c\"]
    let total = 0
    for i in 0..3000 {
        total += serve(name, items, i)
    }
    print(total)
}
";
    let compiled = kavach_driver::compile(src, "serve.kav", Default::default());
    assert!(!compiled.has_errors(), "{}", compiled.render());
    for _ in 0..5 {
        let output = super::run(&compiled);
        assert_eq!(output.stdout, "7200\n", "{}", output.stderr);
        assert_eq!(output.exit_code, 0, "{}", output.stderr);
        let unread = output.stderr.lines();
        assert_eq!(unread.filter(|l| l.contains("request")).count(), 200);
    }
}
