//! One program per agent (spec 8.5, decision 0073): the program that
//! starts an agent locks its folder, so a second program can't share its
//! state file and journal. The lock is the operating system's, so a
//! program that crashed holds none.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The folders this program has locked, with their lock files, kept open
/// until the program ends. Starting an agent again in the same program
/// finds its folder here.
static HELD: Mutex<Vec<(PathBuf, std::fs::File)>> = Mutex::new(Vec::new());

/// How long a second program waits for an agent's folder: a program that
/// is just ending (a restart) lets go of it in that time.
const WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Locks the folder of `file` (an agent's state file or journal): the
/// file `lock` in it. `Err` with the panic's message when another program
/// still holds it after `WAIT`.
pub fn lock_folder(file: &Path) -> Result<(), String> {
    let Some(dir) = file.parent() else {
        return Ok(());
    };
    let mut held = HELD.lock().unwrap_or_else(|e| e.into_inner());
    if held.iter().any(|(d, _)| d == dir) {
        return Ok(());
    }
    let _ = std::fs::create_dir_all(dir);
    let opened = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("lock"));
    let Ok(f) = opened else {
        // A folder that can't hold a lock file: the agent's own files
        // fail there too, with their own message.
        return Ok(());
    };
    let until = std::time::Instant::now() + WAIT;
    let mut taken = f.try_lock().is_ok();
    while !taken && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(50));
        taken = f.try_lock().is_ok();
    }
    match taken {
        true => {
            held.push((dir.to_path_buf(), f));
            Ok(())
        }
        false => {
            let agent = dir
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned());
            Err(format!(
                "the agent {agent} already runs in another program: its folder {} is locked",
                crate::files::shown_path(dir)
            ))
        }
    }
}

/// Lets go of every folder this program locked: when the interpreter's
/// run of a program ends, so the next program (in a test, in the same
/// process) can start the same agent.
pub fn unlock_all() {
    HELD.lock().unwrap_or_else(|e| e.into_inner()).clear();
}
