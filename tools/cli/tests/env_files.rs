//! `.env` files (spec 10.13): `kavach run` reads the project's, `--env`
//! beats the real environment, which beats the file and `[env.vars]`;
//! `kavach build` copies them beside the executable, which reads that
//! copy, or the file `KAVACH_ENV_FILE` names; values are never compiled in.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(dir: &Path, program: &str, args: &[&str], vars: &[(&str, &str)]) -> Output {
    Command::new(program)
        .args(args)
        .env_remove("KAVACH_ENV_FILE")
        .env_remove("GREETING")
        .env_remove("PORT")
        .envs(vars.iter().copied())
        .current_dir(dir)
        .output()
        .unwrap()
}

fn kavach(dir: &Path, args: &[&str], vars: &[(&str, &str)]) -> String {
    let out = run(dir, env!("CARGO_BIN_EXE_kavach"), args, vars);
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

#[test]
fn programs_read_the_dot_env_file() {
    let dir = std::env::temp_dir().join(format!("kavach-env-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("kavach.toml"),
        "[package]\nname = \"envapp\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n[env]\nvars = { MODE = \"from-toml\" }\n",
    )
    .unwrap();
    fs::write(
        dir.join("src/main.kav"),
        "fn main() uses Env, Console {\n    const g = reveal(env(\"GREETING\") catch \"unset\")\n    const p = reveal(env(\"PORT\") catch \"unset\")\n    const m = reveal(env(\"MODE\") catch \"unset\")\n    print(\"{g} {p} {m}\")\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join(".env"),
        "# test\nGREETING=\"hi there\"\nPORT=8080\nMODE=from-file\n",
    )
    .unwrap();

    // `kavach run`: the file, `[env.vars]` over it, the real environment
    // over both, and `--env` over everything.
    assert_eq!(kavach(&dir, &["run"], &[]), "hi there 8080 from-toml\n");
    assert_eq!(
        kavach(&dir, &["run"], &[("PORT", "7000")]),
        "hi there 7000 from-toml\n"
    );
    let forced = kavach(&dir, &["run", "--env", "PORT=9000"], &[("PORT", "7000")]);
    assert_eq!(forced, "hi there 9000 from-toml\n");
    assert_eq!(
        kavach(&dir, &["run", "--no-env-file"], &[]),
        "unset unset from-toml\n"
    );
    fs::write(dir.join("test.env"), "GREETING=other\n").unwrap();
    let named = kavach(&dir, &["run", "--env-file", "test.env"], &[]);
    assert_eq!(named, "other unset from-toml\n");

    // `kavach build`: the copy beside the executable holds the values,
    // and the executable holds none of them.
    let built = kavach(&dir, &["build", "--env", "PORT=5000"], &[]);
    assert!(built.contains("wrote"), "{built}");
    let exe = dir
        .join(".kavach/bin")
        .join(format!("envapp{}", std::env::consts::EXE_SUFFIX));
    let bytes = fs::read(&exe).unwrap();
    assert!(
        !bytes.windows(8).any(|w| w == b"hi there"),
        "a value was compiled in"
    );
    let exe = exe.to_string_lossy().to_string();
    let shown = |vars: &[(&str, &str)]| {
        let out = run(&dir, &exe, &[], vars);
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
    };
    assert_eq!(shown(&[]), "hi there 5000 from-toml\n");
    // The place it runs has the last word (`docker run -e PORT=80`).
    assert_eq!(shown(&[("PORT", "80")]), "hi there 80 from-toml\n");
    assert_eq!(
        shown(&[("KAVACH_ENV_FILE", "test.env")]),
        "other unset unset\n"
    );
    fs::write(dir.join("bad.env"), "no equals sign\n").unwrap();
    let bad = run(&dir, &exe, &[], &[("KAVACH_ENV_FILE", "bad.env")]);
    assert_eq!(bad.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("bad.env:1: expected `NAME=value`"));

    // `--no-env-file` removes the copy an earlier build wrote.
    kavach(&dir, &["build", "--no-env-file"], &[]);
    assert!(!dir.join(".kavach/bin/.env").exists());
    assert_eq!(shown(&[]), "unset unset unset\n");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn values_in_kavach_toml_become_text() {
    let dir = std::env::temp_dir().join(format!("kavach-env-types-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    let toml = |vars: &str| {
        let text = format!(
            "[package]\nname = \"types\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n[env.vars]\n{vars}"
        );
        fs::write(dir.join("kavach.toml"), text).unwrap();
    };
    fs::write(
        dir.join("src/main.kav"),
        "fn main() ! Error uses Env, Console {\n    const port = try Int(reveal(try env(\"PORT\")))\n    const ratio = reveal(try env(\"RATIO\"))\n    const debug = try Bool(reveal(try env(\"DEBUG\")))\n    const when = reveal(try env(\"WHEN\"))\n    const hosts = reveal(try env(\"HOSTS\")).split(\",\")\n    print(\"{port + 1} {ratio} {debug} {when} {hosts.len()}\")\n}\n",
    )
    .unwrap();
    toml("PORT = 8080\nRATIO = 1.5\nDEBUG = true\nWHEN = 2026-11-12\nHOSTS = \"a,b,c\"\n");
    assert_eq!(kavach(&dir, &["run"], &[]), "8081 1.5 true 2026-11-12 3\n");

    // A list is refused: an environment variable holds text.
    toml("HOSTS = [\"a\", \"b\"]\n");
    let refused = kavach(&dir, &["run"], &[]);
    assert!(refused.contains("`HOSTS` in `[env.vars]`"), "{refused}");
    assert!(refused.contains("write a list as text"), "{refused}");
    // So is a name that isn't one, here and in `--env`.
    toml("\"MY-KEY\" = \"x\"\n");
    assert!(kavach(&dir, &["run"], &[]).contains("a name is letters"));
    toml("");
    let flag = kavach(&dir, &["run", "--env", "MY-KEY=x"], &[]);
    assert!(flag.contains("write `--env NAME=value`"), "{flag}");

    let _ = fs::remove_dir_all(&dir);
}
