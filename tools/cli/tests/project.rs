//! Projects (spec 10.3, 10.5): `kavach new`, then `run`, `build` and
//! `check` with no file, from the project folder or below it.

use std::path::Path;
use std::process::{Command, Output};

fn kavach(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kavach"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn new_projects_run_build_and_check_without_a_file() {
    let base = std::env::temp_dir().join(format!("kavach-project-{}", std::process::id()));
    std::fs::create_dir_all(&base).unwrap();
    assert!(!kavach(&base, &["new", "Not_Kebab"]).status.success());
    assert!(kavach(&base, &["new", "demo-app"]).status.success());
    let root = base.join("demo-app");
    for file in ["kavach.toml", "src/main.kav", ".gitignore"] {
        assert!(root.join(file).is_file(), "{file} is missing");
    }
    assert!(
        !kavach(&base, &["new", "demo-app"]).status.success(),
        "it must not overwrite"
    );

    let run = kavach(&root.join("src"), &["run"]);
    assert_eq!(String::from_utf8_lossy(&run.stdout), "Hello, Kavach!\n");
    assert!(kavach(&root, &["check"]).status.success());

    std::fs::write(
        root.join("src/main.kav"),
        "use std.cli\n\nfn main() {\n    print(cli.args())\n}\n",
    )
    .unwrap();
    let run = kavach(&root, &["run", "--", "a", "b"]);
    assert_eq!(String::from_utf8_lossy(&run.stdout), "[\"a\", \"b\"]\n");
    // With a file, the `--` is Kavach's in every way of running it.
    for way in [&["run"][..], &["run", "--native"], &["run", "--dry-run"]] {
        let args = [way, &["src/main.kav", "--", "a", "b"]].concat();
        let run = kavach(&root, &args);
        let out = String::from_utf8_lossy(&run.stdout);
        assert!(out.ends_with("[\"a\", \"b\"]\n"), "{way:?}: {out}");
    }

    assert!(kavach(&root, &["build"]).status.success());
    let exe = root
        .join(".kavach/bin")
        .join(format!("demo-app{}", std::env::consts::EXE_SUFFIX));
    let out = Command::new(&exe).arg("x").output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "[\"x\"]\n");
    let _ = std::fs::remove_dir_all(&base);
}

/// Inside a project, `--dry-run` stays Kavach's flag: no capability call
/// runs and `ask` calls no model (spec 8.5), with or without the file.
#[test]
fn a_dry_run_inside_a_project_executes_nothing() {
    let bot = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/05-refund-bot");
    for args in [
        &["run", "--dry-run"][..],
        &["run", "--dry-run", "src/main.kav"],
    ] {
        let out = kavach(&bot, args);
        // A dry run reports each call it didn't make on the error stream.
        let said = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        assert!(
            said.contains("[dry run] Payments.refund(") && said.contains("[dry run] ask("),
            "{args:?}: {said}"
        );
    }
}

/// `orders.test.kav` is the tests of `orders` (spec 11.8): it sees the
/// module's private names, `run` leaves its tests out, and one with no
/// module beside it is an error.
#[test]
fn test_files_sit_beside_their_module() {
    let root = std::env::temp_dir().join(format!("kavach-test-files-{}", std::process::id()));
    std::fs::create_dir_all(root.join("src/billing")).unwrap();
    std::fs::write(root.join("kavach.toml"), "[package]\nname = \"shop\"\n").unwrap();
    let write = |file: &str, text: &str| std::fs::write(root.join(file), text).unwrap();
    write(
        "src/main.kav",
        "use orders\n\nfn main() {\n    print(orders.total([3, 4]))\n}\n",
    );
    write(
        "src/orders.kav",
        "pub fn total(xs: [Int]) -> Int\n    require xs.len() < 100\n{\n    xs.sum(x => double(x % 1000))\n}\n\nfn double(x: Int) -> Int\n    require x > -1000 and x < 1000\n{\n    x * 2\n}\n",
    );
    write(
        "src/orders.test.kav",
        "test \"total doubles every item\" {\n    assert total([1, 2]) == 6\n    assert double(5) == 10\n}\n",
    );
    let run = kavach(&root, &["run"]);
    assert_eq!(String::from_utf8_lossy(&run.stdout), "14\n");
    let test = kavach(&root, &["test", "src/main.kav"]);
    let out =
        String::from_utf8_lossy(&test.stdout).to_string() + &String::from_utf8_lossy(&test.stderr);
    assert!(out.contains("✓ test \"total doubles every item\""), "{out}");
    // A build leaves the tests out (spec 11.8), so native code builds it.
    let built = kavach(&root, &["run", "--native"]);
    assert_eq!(
        String::from_utf8_lossy(&built.stdout),
        "14
",
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    write(
        "src/billing/invoice.test.kav",
        "test \"alone\" {\n    assert true\n}\n",
    );
    let check = kavach(&root, &["check"]);
    let err = String::from_utf8_lossy(&check.stderr);
    assert!(!check.status.success());
    assert!(
        err.contains("`invoice.test.kav` holds the tests of `invoice.kav`, but there is no `invoice.kav` beside it"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `kavach test` in a project (spec 11.4): found cases go to the file of
/// the module they were found in, and the report's paths are relative.
#[test]
fn found_cases_go_to_their_modules_file() {
    let root = std::env::temp_dir().join(format!("kavach-found-{}", std::process::id()));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("kavach.toml"), "[package]\nname = \"shop\"\n").unwrap();
    std::fs::write(
        root.join("src/main.kav"),
        "use orders\n\nfn main() {\n    print(orders.fee(1))\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/orders.kav"),
        "pub fn fee(amount: Int) -> Int {\n    amount + 1\n}\n\npub fn note() -> String {\n    \"ok\"\n}\n",
    )
    .unwrap();
    let test = kavach(&root, &["test", "--seed", "7"]);
    let out =
        String::from_utf8_lossy(&test.stdout).to_string() + &String::from_utf8_lossy(&test.stderr);
    assert!(out.contains("    src/orders.kav:2   amount + 1"), "{out}");
    assert!(out.contains("  src/orders.kav:5   note"), "{out}");
    assert!(out.contains("saved to tests/found/orders.kavf"), "{out}");
    assert!(root.join("tests/found/orders.kavf").is_file());
    assert!(!root.join("tests/found/main.kavf").exists());
    let _ = std::fs::remove_dir_all(&root);
}

/// `kavach test` names each module nothing written by a person checks
/// (spec 11.9): no example, ensure, invariant or test, in it or beside it.
#[test]
fn the_report_names_modules_nothing_checks() {
    let root = std::env::temp_dir().join(format!("kavach-missed-{}", std::process::id()));
    std::fs::create_dir_all(root.join("src/billing")).unwrap();
    std::fs::write(root.join("kavach.toml"), "[package]\nname = \"shop\"\n").unwrap();
    let write = |file: &str, text: &str| std::fs::write(root.join(file), text).unwrap();
    write(
        "src/main.kav",
        "use orders\nuse billing.invoice\n\nfn main() {\n    print(orders.fee(1))\n    print(invoice.label(2))\n}\n",
    );
    write(
        "src/orders.kav",
        "pub fn fee(amount: Int) -> Int\n    require amount < 1000\n    example fee(1) == 2\n{\n    amount + 1\n}\n",
    );
    write(
        "src/billing/invoice.kav",
        "pub fn label(n: Int) -> String {\n    \"n{n}\"\n}\n",
    );
    let report = || {
        let out = kavach(&root, &["test", "--seed", "7"]);
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr)
    };
    let first = report();
    write(
        "src/billing/invoice.test.kav",
        "test \"label\" {\n    assert label(2) == \"n2\"\n}\n",
    );
    let second = report();
    let _ = std::fs::remove_dir_all(&root);
    let heading = "nothing checked by hand (no example, ensure, invariant or test) in:\n";
    assert!(
        first.contains(&format!("{heading}  src/billing/invoice.kav\n")),
        "{first}"
    );
    assert!(!first.contains("  src/orders.kav\n"), "{first}");
    assert!(!second.contains(heading), "{second}");
}
