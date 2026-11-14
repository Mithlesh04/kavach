//! Programs run through the system's shell here, only to have small
//! programs to run: `std.process` itself never uses one.

use super::*;
use std::time::{Duration, Instant};

fn shell(script: &str) -> (String, Vec<String>) {
    if cfg!(windows) {
        ("cmd".into(), vec!["/C".into(), script.into()])
    } else {
        ("sh".into(), vec!["-c".into(), script.into()])
    }
}

/// Child `id`'s events until its end, or `None` after `limit`.
fn events(id: i64, limit: Duration) -> Option<Vec<Event>> {
    let until = Instant::now() + limit;
    let mut got = Vec::new();
    while Instant::now() < until {
        match next(id) {
            Some(e @ Event::Ended(_)) => {
                got.push(e);
                return Some(got);
            }
            Some(e) => got.push(e),
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    None
}

#[test]
fn output_and_errors_come_as_lines_then_the_end() {
    let (program, args) = shell("echo one&& (echo two)1>&2&& exit 3");
    let id = start(&program, &args, &[]).unwrap();
    let got = events(id, Duration::from_secs(20)).expect("the program ended");
    close(id);
    let lines: Vec<_> = got
        .iter()
        .filter(|e| !matches!(e, Event::Ended(_)))
        .collect();
    assert!(lines.contains(&&Event::Out("one".into())), "{got:?}");
    assert!(lines.contains(&&Event::Err("two".into())), "{got:?}");
    let Some(Event::Ended(exit)) = got.last() else {
        unreachable!()
    };
    assert_eq!(exit.code, 3);
    assert!(exit.cpu_ms >= 0 && exit.peak_bytes >= 0);
}

#[test]
fn the_folder_and_environment_reach_the_program() {
    let script = if cfg!(windows) {
        "echo %KV_TEST%& cd"
    } else {
        "echo $KV_TEST; pwd"
    };
    let (program, args) = shell(script);
    let folder = std::env::temp_dir();
    let setup = [
        folder.to_string_lossy().to_string(),
        "KV_TEST".into(),
        "given".into(),
    ];
    let id = start(&program, &args, &setup).unwrap();
    let got = events(id, Duration::from_secs(20)).expect("the program ended");
    close(id);
    assert_eq!(got[0], Event::Out("given".into()), "{got:?}");
    let Event::Out(cwd) = &got[1] else {
        panic!("{got:?}")
    };
    let same = std::fs::canonicalize(cwd).ok() == std::fs::canonicalize(&folder).ok();
    assert!(same, "{cwd} is not {}", folder.display());
}

#[test]
fn kill_ends_the_programs_it_started_too() {
    // The first program starts a second that would run 60 seconds; both
    // hold the output open, so the end comes only once both are gone.
    let script = if cfg!(windows) {
        "ping -n 60 127.0.0.1 >nul"
    } else {
        "sleep 60 & wait"
    };
    let (program, args) = shell(script);
    let id = start(&program, &args, &[]).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let began = Instant::now();
    kill(id);
    assert!(
        events(id, Duration::from_secs(20)).is_some(),
        "the tree kept running"
    );
    assert!(began.elapsed() < Duration::from_secs(20));
    close(id);
}

#[test]
fn a_missing_program_or_folder_is_an_error() {
    let missing = start("kavach-no-such-program", &[], &[]);
    assert_eq!(
        missing,
        Err(Fail::NotFound("kavach-no-such-program".into()))
    );
    let (program, args) = shell("echo hi");
    let folder = start(&program, &args, &["kavach-no-such-folder".to_string()]);
    assert!(matches!(folder, Err(Fail::Failed(_))), "{folder:?}");
}

#[test]
fn the_end_comes_again_after_it_was_taken() {
    let (program, args) = shell("exit 4");
    let id = start(&program, &args, &[]).unwrap();
    let got = events(id, Duration::from_secs(20)).expect("the program ended");
    assert!(matches!(got.last(), Some(Event::Ended(e)) if e.code == 4));
    assert_eq!(next(id), got.last().cloned());
    close(id);
    assert_eq!(next(id), None);
    assert!(peak_bytes() > 0);
}
