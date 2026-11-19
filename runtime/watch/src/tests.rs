// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Watching a folder made for the test, in the system's temporary folder.

use crate::{CHANGED, CREATED, REMOVED, changes, close, own_temp, poll, shown, watch};
use notify::event::{CreateKind, EventKind, ModifyKind, RenameMode};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[test]
fn a_rename_is_a_removal_and_a_creation() {
    let e = notify::Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
        .add_path(PathBuf::from("/w/a.txt"))
        .add_path(PathBuf::from("/w/b.txt"));
    let got = changes(&e);
    assert_eq!(
        got,
        vec![
            (REMOVED, PathBuf::from("/w/a.txt")),
            (CREATED, PathBuf::from("/w/b.txt"))
        ]
    );
    let made =
        notify::Event::new(EventKind::Create(CreateKind::File)).add_path(PathBuf::from("/w/c"));
    assert_eq!(changes(&made), vec![(CREATED, PathBuf::from("/w/c"))]);
}

#[test]
fn paths_are_shown_under_the_watched_path_as_written() {
    let full = Path::new("/home/me/game/shaders");
    assert_eq!(
        shown("shaders", full, &full.join("sky.wgsl")),
        "shaders/sky.wgsl"
    );
    assert_eq!(
        shown("shaders/", full, &full.join("a").join("b.wgsl")),
        "shaders/a/b.wgsl"
    );
    assert_eq!(shown("shaders", full, full), "shaders");
}

#[test]
fn a_watched_folder_tells_of_a_new_file() {
    let dir = std::env::temp_dir().join(format!("kavach-watch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let given = dir.to_string_lossy().to_string();
    let n = watch(&given).expect("watching");
    std::fs::write(dir.join("level.txt"), "1").expect("written");
    let start = Instant::now();
    let mut seen = Vec::new();
    while start.elapsed() < Duration::from_secs(5)
        && !seen
            .iter()
            .any(|(_, p): &(u8, String)| p.ends_with("level.txt"))
    {
        seen.extend(poll(n));
        std::thread::sleep(Duration::from_millis(20));
    }
    close(n);
    let _ = std::fs::remove_dir_all(&dir);
    let found = seen
        .iter()
        .find(|(_, p)| p.ends_with("level.txt"))
        .expect("the new file");
    assert!(matches!(found.0, CREATED | CHANGED), "{seen:?}");
    assert!(watch("no/such/folder/here").is_err());
}

#[test]
fn a_whole_file_write_shows_only_the_file() {
    // The hidden file `fs.write_text` goes through is never a change.
    assert!(own_temp(Path::new("data/.level.txt.kavach-40070-0.tmp")));
    assert!(!own_temp(Path::new("data/level.txt")));
    assert!(!own_temp(Path::new("data/.hidden.tmp")));
    let dir = std::env::temp_dir().join(format!("kavach-watch-temp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let n = watch(&dir.to_string_lossy()).expect("watching");
    let temp = dir.join(".level.txt.kavach-1-0.tmp");
    std::fs::write(&temp, "1").expect("written");
    std::fs::rename(&temp, dir.join("level.txt")).expect("renamed");
    let start = Instant::now();
    let mut seen: Vec<(u8, String)> = Vec::new();
    while start.elapsed() < Duration::from_millis(500) {
        seen.extend(poll(n));
        std::thread::sleep(Duration::from_millis(20));
    }
    close(n);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        seen.iter().any(|(_, p)| p.ends_with("level.txt")),
        "{seen:?}"
    );
    assert!(
        seen.iter().all(|(_, p)| !p.contains(".kavach-")),
        "{seen:?}"
    );
}
