//! The compiler never panics on hostile input (to-do item 4): any text,
//! however broken, gets error messages, not a crash. Deterministic and
//! seeded, so a failure reproduces; it prints the input that crashed.
//! `KAVACH_FUZZ_CASES` raises the case count for a long run.

use kavach_driver::{Config, Options, compile};

/// xorshift64*: deterministic, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn cases(default: usize) -> usize {
    std::env::var("KAVACH_FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Runs every case with panic printing off, then reports the inputs that
/// crashed. The hook comes back before the report, so the report shows.
fn run(inputs: Vec<String>, f: impl Fn(&str) + std::panic::RefUnwindSafe) {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut failed = Vec::new();
    for src in &inputs {
        if std::panic::catch_unwind(|| f(src)).is_err() {
            failed.push(format!("panicked on:\n{src}"));
        }
    }
    std::panic::set_hook(hook);
    assert!(failed.is_empty(), "\n{}", failed.join("\n---\n"));
}

/// Characters hostile text is made of: syntax, quotes, digits, a few
/// multi-byte ones.
fn soup(rng: &mut Rng, len: usize) -> String {
    const PIECES: &[&str] = &[
        "{", "}", "(", ")", "[", "]", "\"", "\\", "'", "\n", " ", "\t", "fn", "let", "var", "type",
        "if", "else", "for", "while", "match", "try", "catch", "fail", "spawn", "use", "0", "9",
        "_", ".", ",", ":", ";", "=", "=>", "->", "+", "-", "*", "/", "%", "<", ">", "!", "?", "|",
        "&", "a", "x", "é", "💥", "\u{0}", "\r", "//", "/*", "e9", "0x", "1_000", "\"{", "}\"",
        "..", "...",
    ];
    let mut out = String::new();
    while out.len() < len {
        out.push_str(PIECES[rng.below(PIECES.len())]);
    }
    out
}

/// A cut of `s` at a random character boundary at or after `at`.
fn boundary(s: &str, at: usize) -> usize {
    (at..=s.len()).find(|i| s.is_char_boundary(*i)).unwrap()
}

#[test]
fn random_text_never_crashes_the_compiler() {
    let mut rng = Rng(0x0005_EED1);
    let inputs: Vec<String> = (0..cases(300))
        .map(|_| {
            let len = rng.below(400);
            soup(&mut rng, len)
        })
        .collect();
    run(inputs, |src| {
        compile(src, "fuzz.kav", Options::default());
    });
}

/// Every conformance test, broken in random ways: truncated at a
/// character, a line removed or doubled, or two programs spliced.
#[test]
fn mutated_programs_never_crash_the_compiler() {
    let mut sources = Vec::new();
    collect(
        std::path::Path::new("../../tests/conformance"),
        &mut sources,
    );
    assert!(sources.len() > 100, "found only {}", sources.len());
    let mut rng = Rng(0x0005_EED2);
    let inputs: Vec<String> = (0..cases(300))
        .map(|_| {
            let a = &sources[rng.below(sources.len())];
            match rng.below(4) {
                0 => a[..boundary(a, rng.below(a.len() + 1))].to_string(),
                1 => {
                    let lines: Vec<&str> = a.lines().collect();
                    lines[..rng.below(lines.len() + 1)].join("\n")
                }
                2 => {
                    let lines: Vec<&str> = a.lines().collect();
                    let i = rng.below(lines.len());
                    let mut out = lines.clone();
                    out.insert(i, lines[i]);
                    out.join("\n")
                }
                _ => {
                    let b = &sources[rng.below(sources.len())];
                    let cut = boundary(a, rng.below(a.len() + 1));
                    let cut_b = boundary(b, rng.below(b.len() + 1));
                    format!("{}{}", &a[..cut], &b[cut_b..])
                }
            }
        })
        .collect();
    run(inputs, |src| {
        compile(src, "fuzz.kav", Options::default());
    });
}

#[test]
fn hostile_kavach_toml_never_crashes() {
    let valid = "[package]\nname = \"app\"\nunsafe = true\n\n[limits]\nfn_lines = 40\n\n[log]\nlevel = \"info\"\nmax_bytes = 1000\n\n[log.levels]\n\"a.b\" = \"warn\"\n";
    let mut rng = Rng(0x0005_EED3);
    let inputs: Vec<String> = (0..cases(300))
        .map(|_| match rng.below(2) {
            0 => {
                let len = rng.below(200);
                soup(&mut rng, len)
            }
            _ => {
                let cut = boundary(valid, rng.below(valid.len() + 1));
                let len = rng.below(60);
                format!("{}{}", &valid[..cut], soup(&mut rng, len))
            }
        })
        .collect();
    run(inputs, |src| {
        let _ = Config::parse(src);
    });
}

fn collect(dir: &std::path::Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|x| x == "kav")
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            out.push(text);
        }
    }
}
