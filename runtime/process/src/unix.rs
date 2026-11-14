//! A program's tree on Linux and macOS (decision 0105): a process group
//! of its own, which `killpg` ends whole. `wait4` gives the program's end
//! with its processor time and largest resident set, its own and those of
//! the programs it waited for.
#![allow(unsafe_code)]

use super::Exit;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

/// The process group holding a program's tree: its first program's id.
pub struct Tree {
    group: libc::pid_t,
}

impl Tree {
    /// Ends every program in the group.
    pub fn kill(&self) {
        // SAFETY: sending a signal touches no memory of ours. A group id
        // isn't given to another group while any program is still in it.
        unsafe {
            libc::killpg(self.group, libc::SIGKILL);
        }
    }
}

/// Starts `command` in a process group of its own.
pub fn spawn(command: &mut Command) -> std::io::Result<(Child, Tree)> {
    let child = command.process_group(0).spawn()?;
    let group = child.id() as libc::pid_t;
    Ok((child, Tree { group }))
}

/// Waits for the program's end with `wait4`, which the standard library
/// can't give: it reaps the program, so `child` is never waited for again.
pub fn wait(child: Child, _tree: &Tree) -> Exit {
    let pid = child.id() as libc::pid_t;
    // SAFETY: `status` and `usage` are plain C values the call fills;
    // `wait4` is retried when a signal interrupts it.
    let (status, usage) = unsafe {
        let mut status: libc::c_int = 0;
        let mut usage: libc::rusage = std::mem::zeroed();
        loop {
            let r = libc::wait4(pid, &mut status, 0, &mut usage);
            if r >= 0 || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
                break;
            }
        }
        (status, usage)
    };
    drop(child);
    let code = if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else if libc::WIFSIGNALED(status) {
        128 + libc::WTERMSIG(status)
    } else {
        -1
    };
    let ms = |t: libc::timeval| t.tv_sec as i64 * 1000 + t.tv_usec as i64 / 1000;
    Exit {
        code: i64::from(code),
        cpu_ms: ms(usage.ru_utime) + ms(usage.ru_stime),
        peak_bytes: max_rss(usage.ru_maxrss),
    }
}

/// `ru_maxrss` in bytes: Linux counts kilobytes, macOS bytes.
fn max_rss(n: libc::c_long) -> i64 {
    if cfg!(target_os = "macos") {
        n as i64
    } else {
        n as i64 * 1024
    }
}

/// This program's largest resident set so far.
pub fn peak_self() -> i64 {
    // SAFETY: `usage` is a plain C value the call fills.
    let usage = unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &mut usage);
        usage
    };
    max_rss(usage.ru_maxrss)
}
