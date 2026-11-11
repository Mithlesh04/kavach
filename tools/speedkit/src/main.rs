//! `compare`, the runner of the speed kit: a download with the benchmark
//! programs already built in Kavach and in C++, and their source, so
//! anyone can time them on their own machine with nothing installed.
//!
//! It reads `kit.txt` next to itself (what built the programs, and one
//! `<suite> <name>` line per benchmark), checks that each pair prints the
//! same result, then runs the two in turns and prints the best time of
//! each, with how many times faster Kavach is. The table also goes to
//! `results.txt`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

mod machine;

/// The kit's folder: where `compare` itself is.
fn kit() -> Result<PathBuf, String> {
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(me.parent().ok_or("no folder")?.to_path_buf())
}

/// One benchmark of the kit.
struct Bench {
    suite: String,
    name: String,
}

/// What `kit.txt` says: lines starting with `#` describe the build;
/// the others are `<suite> <name>`.
fn read_kit(dir: &Path) -> Result<(Vec<String>, Vec<Bench>), String> {
    let text = std::fs::read_to_string(dir.join("kit.txt"))
        .map_err(|e| format!("can't read kit.txt next to compare: {e}"))?;
    let mut about = Vec::new();
    let mut benches = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(note) = line.strip_prefix('#') {
            about.push(note.trim().to_string());
        } else if let Some((suite, name)) = line.split_once(' ') {
            benches.push(Bench {
                suite: suite.into(),
                name: name.trim().into(),
            });
        }
    }
    Ok((about, benches))
}

fn program(dir: &Path, lang: &str, name: &str) -> PathBuf {
    dir.join("programs")
        .join(lang)
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

/// Runs `exe` once: its output, and how long it took in milliseconds.
fn run(exe: &Path) -> Result<(String, f64), String> {
    let start = Instant::now();
    let out = Command::new(exe)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("can't run {}: {e}", exe.display()))?;
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    if !out.status.success() {
        return Err(format!("{} failed: {}", exe.display(), out.status));
    }
    Ok((String::from_utf8_lossy(&out.stdout).into_owned(), ms))
}

/// The best times of `kv` and `cpp` over `runs` runs in turns, once
/// their results match; otherwise `None`, with why said to `say`.
fn time_pair(
    kv: &Path,
    cpp: &Path,
    runs: usize,
    say: &mut impl FnMut(String),
) -> Option<(f64, f64)> {
    let timed = || -> Result<(f64, f64), String> {
        let (kv_out, mut kv_best) = run(kv)?;
        let (cpp_out, mut cpp_best) = run(cpp)?;
        if result(&kv_out) != result(&cpp_out) {
            return Err("results differ: not timed".into());
        }
        for _ in 1..runs {
            kv_best = kv_best.min(run(kv)?.1);
            cpp_best = cpp_best.min(run(cpp)?.1);
        }
        Ok((kv_best, cpp_best))
    };
    timed().map_err(say).ok()
}

/// The result a program prints, without the lines that report its own
/// timings (`... 188 ms`), which differ from run to run.
fn result(output: &str) -> String {
    output
        .lines()
        .filter(|l| !l.contains(" ms"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// How `ratio` (C++'s time over Kavach's) reads. Within 5% either way
/// is within what runs vary by.
fn verdict(ratio: f64) -> String {
    if (0.95..=1.05).contains(&ratio) {
        "about the same".into()
    } else if ratio > 1.0 {
        format!("{ratio:.1}x faster")
    } else {
        format!("{:.1}x slower", 1.0 / ratio)
    }
}

/// The options: `--runs N` (default 5) and `--only <name>`.
fn options() -> Result<(usize, Option<String>), String> {
    let mut runs = 5;
    let mut only = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--runs" => {
                runs = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|n| *n > 0)
                    .ok_or("--runs needs a number above 0")?
            }
            "--only" => only = Some(args.next().ok_or("--only needs a benchmark's name")?),
            "--help" | "-h" => {
                println!(
                    "compare [--runs N] [--only <name>]: times Kavach against C++ on this machine"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown option {other}; try --help")),
        }
    }
    Ok((runs, only))
}

fn main() {
    if let Err(e) = compare() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn compare() -> Result<(), String> {
    let (runs, only) = options()?;
    let dir = kit()?;
    let (about, benches) = read_kit(&dir)?;
    let mut table = String::new();
    let mut say = |line: String| {
        println!("{line}");
        table.push_str(&line);
        table.push('\n');
    };
    say("Kavach against C++, on this machine".into());
    for line in machine::describe() {
        say(format!("  {line}"));
    }
    for line in &about {
        say(format!("  {line}"));
    }
    say(format!(
        "  each time is the best of {runs} runs, the two languages in turns"
    ));
    say(String::new());
    say(format!(
        "{:<24} {:>10} {:>10}  {}",
        "benchmark", "Kavach ms", "C++ ms", "Kavach is"
    ));
    let (mut faster, mut even, mut timed, mut log_sum) = (0, 0, 0, 0.0);
    for b in benches
        .iter()
        .filter(|b| only.as_ref().is_none_or(|o| *o == b.name))
    {
        let label = format!("{}/{}", b.suite, b.name);
        let (kv, cpp) = (
            program(&dir, "kavach", &b.name),
            program(&dir, "cpp", &b.name),
        );
        let Some((kv_best, cpp_best)) =
            time_pair(&kv, &cpp, runs, &mut |m| say(format!("{label:<24} {m}")))
        else {
            continue;
        };
        let ratio = cpp_best / kv_best.max(0.001);
        let verdict = verdict(ratio);
        say(format!(
            "{label:<24} {kv_best:>10.0} {cpp_best:>10.0}  {verdict}"
        ));
        timed += 1;
        faster += usize::from(ratio > 1.05);
        even += usize::from((0.95..=1.05).contains(&ratio));
        log_sum += ratio.ln();
    }
    if timed > 0 {
        let mean = (log_sum / timed as f64).exp();
        say(String::new());
        say(format!(
            "Of {timed}: Kavach faster in {faster}, about the same in {even}, slower in {}; on average (geometric mean) {mean:.2}x C++'s speed.",
            timed - faster - even
        ));
    }
    let saved = dir.join("results.txt");
    match std::fs::write(&saved, &table) {
        Ok(()) => println!("(saved in {})", saved.display()),
        Err(e) => eprintln!("note: can't save results.txt: {e}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_result_leaves_out_the_lines_that_report_times() {
        let kavach = "true
one by one 188 ms, parallel_map 269 ms
";
        let cpp = "true
one by one 4725 ms, parallel_map 641 ms
";
        assert_eq!(super::result(kavach), super::result(cpp));
        assert_ne!(
            super::result(
                "15
"
            ),
            super::result(
                "16
"
            )
        );
    }

    #[test]
    fn a_difference_within_five_percent_is_about_the_same() {
        assert_eq!(super::verdict(1.04), "about the same");
        assert_eq!(super::verdict(0.96), "about the same");
        assert_eq!(super::verdict(2.5), "2.5x faster");
        assert_eq!(super::verdict(0.5), "2.0x slower");
    }
}
