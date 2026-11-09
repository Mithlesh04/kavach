//! Spec 10.11 lists every setting: each `kavach.toml` key with its
//! default, and each environment variable. These tests fail when the code
//! and the list part ways: a key the code reads that the list lacks (or
//! the other way round), a default that differs, or a `KAVACH_` variable
//! the code reads that the list doesn't name.

use kavach_driver::Config;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The rows of the tables in spec 10.11, as cells.
fn rows() -> Vec<Vec<String>> {
    let spec =
        std::fs::read_to_string(repo().join("docs/spec/10-modules-and-packages.md")).unwrap();
    let start = spec.find("## 10.11").expect("spec 10.11");
    let end = spec[start..]
        .find("## Open questions")
        .map_or(spec.len(), |e| start + e);
    spec[start..end]
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("|---"))
        .map(|l| {
            l.trim_matches('|')
                .split(" | ")
                .map(|c| c.trim().to_string())
                .collect()
        })
        .collect()
}

/// The text between backticks in `cell`, or the cell.
fn code(cell: &str) -> String {
    cell.trim_matches('`').to_string()
}

/// The keys the code knows in `section`, from the error for one it
/// doesn't: "expected one of `a`, `b`", or "expected `a` or `b`".
fn known_keys(section: &str, before: &str) -> BTreeSet<String> {
    let text = format!("{section}\n{before}zz_not_a_key = 1\n");
    let err = Config::parse(&text).expect_err("an unknown key is an error");
    let list = err
        .split("expected")
        .nth(1)
        .unwrap_or_else(|| panic!("{err}"));
    list.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

#[test]
fn every_key_of_the_code_is_in_the_list() {
    let rows = rows();
    for (section, before) in [
        ("[package]", ""),
        ("[limits]", ""),
        ("[test]", ""),
        ("[ai]", ""),
        ("[log]", ""),
        ("[log.otlp]", "endpoint = \"x\"\n"),
    ] {
        let written = section.replace("<Name>", "Bot");
        let mut code_keys = known_keys(&written, before);
        // Read only to say where they went: into the code (decision 0056).
        if section == "[ai]" {
            code_keys.retain(|k| k != "model" && k != "max_tokens");
        }
        // And each `spawn` chooses how its task runs (decision 0069).
        if section == "[package]" {
            code_keys.retain(|k| k != "tasks");
        }
        let inside = format!("{}.", &section[..section.len() - 1]);
        let mut listed = BTreeSet::new();
        for r in rows.iter().filter(|r| r.len() == 5) {
            let s = code(&r[0]);
            if s == section {
                listed.insert(code(&r[1]));
            } else if let Some(sub) = s.strip_prefix(&inside) {
                // A table inside the section (`[log.otlp]`) is a key of it.
                listed.insert(sub.trim_end_matches(']').to_string());
            }
        }
        assert_eq!(
            code_keys, listed,
            "the keys of {section} in the code and in spec 10.11"
        );
    }
    // `[agents]` takes any table, only to say it moved into the code
    // (decision 0060), so its one key is checked by name.
    let listed: Vec<String> = (rows.iter())
        .filter(|r| r.len() == 5 && code(&r[0]) == "[agents]")
        .map(|r| code(&r[1]))
        .collect();
    assert_eq!(listed, ["folder"]);
}

#[test]
fn every_default_is_the_listed_one() {
    let c = Config::parse("").unwrap();
    let lower = |d: &dyn std::fmt::Debug| format!("\"{}\"", format!("{d:?}").to_lowercase());
    let actual = [
        ("[package]", "name", format!("{:?}", c.package.name)),
        ("[package]", "version", format!("{:?}", c.package.version)),
        ("[package]", "edition", format!("{:?}", c.package.edition)),
        ("[package]", "unsafe", c.package.allow_unsafe.to_string()),
        ("[limits]", "file_lines", c.limits.file_lines.to_string()),
        ("[limits]", "fn_lines", c.limits.fn_lines.to_string()),
        ("[limits]", "level", lower(&c.limits.level)),
        ("[limits]", "include", format!("{:?}", c.limits.include)),
        ("[limits]", "exclude", format!("{:?}", c.limits.exclude)),
        ("[test]", "cases", c.test.cases.to_string()),
        ("[test]", "deep_cases", c.test.deep_cases.to_string()),
        ("[test]", "time_budget", format!("{:?}", c.test.time_budget)),
        ("[ai]", "provider", format!("{:?}", c.ai.provider)),
        // `None` is the folder `agents`, as `Compiled::agent_config` reads it.
        (
            "[agents]",
            "folder",
            format!("{:?}", c.agents.folder.as_deref().unwrap_or("agents")),
        ),
        ("[log]", "level", format!("{:?}", c.log.level)),
        ("[log]", "console", c.log.console.to_string()),
        ("[log]", "max_bytes", c.log.max_bytes.to_string()),
        ("[log]", "keep", c.log.keep.to_string()),
    ];
    let rows = rows();
    for (section, key, value) in actual {
        let row = (rows.iter())
            .find(|r| code(&r[0]) == section && code(&r[1]) == key)
            .unwrap_or_else(|| panic!("{section} {key} is not in spec 10.11"));
        assert_eq!(code(&row[3]), value, "the default of {section} {key}");
    }
}

/// Every `"KAVACH_…"` text in the program's Rust code, outside test folders.
fn variables_in(dir: &Path, found: &mut BTreeSet<String>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !matches!(name.as_str(), "tests" | "target" | "node_modules") {
                variables_in(&path, found);
            }
        } else if name.ends_with(".rs") {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            for piece in text.split("\"KAVACH_").skip(1) {
                let var: String = piece
                    .chars()
                    .take_while(|c| c.is_ascii_uppercase() || *c == '_')
                    .collect();
                if piece[var.len()..].starts_with('"') {
                    found.insert(format!("KAVACH_{var}"));
                }
            }
        }
    }
}

#[test]
fn every_environment_variable_is_in_the_list() {
    let mut found = BTreeSet::new();
    for dir in ["runtime", "compiler", "tools"] {
        variables_in(&repo().join(dir), &mut found);
    }
    let listed: BTreeSet<String> = (rows().iter())
        .map(|r| code(&r[0]))
        .filter(|v| v.starts_with("KAVACH_"))
        .collect();
    assert_eq!(
        found, listed,
        "the KAVACH_ variables in the code and in spec 10.11"
    );
}

#[test]
fn a_misspelled_key_or_section_is_an_error() {
    let key = Config::parse("[package]\ntaks = \"threads\"\n").expect_err("a misspelled key");
    assert!(key.contains("taks") && key.contains("`tasks`"), "{key}");
    let section = Config::parse("[limts]\nfile_lines = 3\n").expect_err("a misspelled section");
    assert!(
        section.contains("limts") && section.contains("`limits`"),
        "{section}"
    );
}

/// `model` and `max_tokens` moved from `[ai]` into the code (decision
/// 0056): a `kavach.toml` that still has one says where.
#[test]
fn the_model_in_kavach_toml_says_where_it_went() {
    let dir = std::env::temp_dir().join(format!("kavach-moved-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("kavach.toml"),
        "[ai]\nmodel = \"claude-opus-5-5\"\n",
    )
    .unwrap();
    let err = Config::for_file(&dir.join("main.kav")).expect_err("moved");
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(
        err.contains("`model` in `[ai]` moved into the code"),
        "{err}"
    );
}

/// `[agents.<Name>]` moved into the code (decision 0060): an error that
/// says where its keys went.
#[test]
fn agent_tables_moved_into_the_code() {
    let dir = std::env::temp_dir().join(format!("kavach-agent-table-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("kavach.toml"),
        "[agents.Bot]\nmode = \"observe\"\n",
    )
    .unwrap();
    let err = Config::for_file(&dir.join("main.kav")).expect_err("moved");
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(err.contains("`[agents.Bot]` moved into the code"), "{err}");
    assert!(
        err.contains("`mode = \"observe\"` is `mode: AgentMode.Observe`"),
        "{err}"
    );
}

/// `Audit(otlp: true)` sends to the `[log.otlp]` collector: without one,
/// it is a compile error that says what to add (spec 8.6).
#[test]
fn audit_otlp_needs_a_collector() {
    let src = "agent Bot {\n    settings AgentSettings(audit: Audit(otlp: true))\n    can Ai\n}\n\nfn main() {\n    print(1)\n}\n";
    let c = kavach_driver::compile(src, "main.kav", Default::default());
    let found = c
        .messages
        .iter()
        .any(|m| m.text.contains("add a `[log.otlp]` section"));
    assert!(found, "{:?}", c.messages);
}

/// An edition Kavach doesn't have is an error that names the ones it has
/// (spec 10.8).
#[test]
fn an_unknown_edition_is_an_error() {
    let dir = std::env::temp_dir().join(format!("kavach-edition-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.kav");
    std::fs::write(&file, "").unwrap();
    std::fs::write(
        dir.join("kavach.toml"),
        "[package]\nname = \"x\"\nedition = \"2062\"\n",
    )
    .unwrap();
    let error = Config::for_file(&file).unwrap_err();
    assert!(
        error.starts_with("Kavach has no edition `2062`: its editions are `2026`"),
        "{error}"
    );
    std::fs::write(
        dir.join("kavach.toml"),
        "[package]\nname = \"x\"\nedition = \"2026\"\n",
    )
    .unwrap();
    assert!(Config::for_file(&file).is_ok());
    std::fs::remove_dir_all(&dir).unwrap();
}
