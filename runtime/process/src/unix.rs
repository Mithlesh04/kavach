// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A program's tree on Linux and macOS (decision 0105): a process group
//! of its own, which `killpg` ends whole while its first program is
//! unreaped (`reaped.rs`). `wait4` gives the program's end with its
//! processor time and largest resident set, its own and those of the
//! programs it waited for.
#![allow(unsafe_code)]

use super::Exit;
use crate::reaped::Leader;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

/// The process group holding a program's tree: its first program's id.
pub struct Tree {
    group: libc::pid_t,
    leader: Leader,
}

impl Tree {
    /// Ends every program in the group, unless its first program was
    /// reaped: then the group's id may be a stranger's.
    pub fn kill(&self) {
        self.leader.signal(|| {
            // SAFETY: sending a signal touches no memory of ours. The
            // group's first program isn't reaped (`signal` runs this only
            // then, under the lock `wait` reaps it under), so the id is
            // still this group's and no other group's.
            unsafe {
                libc::killpg(self.group, libc::SIGKILL);
            }
        });
    }
}

/// Starts `command` in a process group of its own.
pub fn spawn(command: &mut Command) -> std::io::Result<(Child, Tree)> {
    let child = unblocked(command.process_group(0)).spawn()?;
    let group = child.id() as libc::pid_t;
    let leader = Leader::default();
    Ok((child, Tree { group, leader }))
}

/// `command` with no signal blocked in the program it starts. A Kavach
/// program blocks the stop signals in its threads, to wait for them on
/// one (spec 5.6), and a started program would keep that mask: `SIGTERM`
/// and Ctrl+C couldn't end it (14.23).
pub fn unblocked(command: &mut Command) -> &mut Command {
    // SAFETY: runs in the new process between `fork` and `exec`, where
    // only async-signal-safe calls are allowed; `sigemptyset` and
    // `sigprocmask` are, and touch only the local `set`.
    unsafe {
        command.pre_exec(|| {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigprocmask(libc::SIG_SETMASK, &set, std::ptr::null_mut());
            Ok(())
        })
    }
}

/// `call`, again while a signal interrupts it; what it gave at last.
fn retried(mut call: impl FnMut() -> libc::c_int) -> libc::c_int {
    loop {
        let r = call();
        if r >= 0 || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            return r;
        }
    }
}

/// Waits for the program's end with `wait4`, which the standard library
/// can't give: it reaps the program, so `child` is never waited for again.
/// It first waits without reaping, so `kill` can end the group until the
/// reap, which takes the lock `kill` takes. If the wait fails, the code
/// is -1, never a success.
pub fn wait(child: Child, tree: &Tree) -> Exit {
    let pid = child.id() as libc::pid_t;
    // SAFETY: `info` is a plain C value the call fills. `WNOWAIT` leaves
    // the program unreaped, so its id stays its own.
    let ended = retried(|| unsafe {
        let mut info: libc::siginfo_t = std::mem::zeroed();
        libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOWAIT,
        )
    }) == 0;
    // SAFETY: `status` and `usage` are plain C values the call fills. The
    // program has ended, so `wait4` returns at once, with the lock held.
    let (r, status, usage) = tree.leader.reap(|| unsafe {
        let mut status: libc::c_int = 0;
        let mut usage: libc::rusage = std::mem::zeroed();
        let r = if ended {
            retried(|| libc::wait4(pid, &mut status, 0, &mut usage))
        } else {
            -1
        };
        (r, status, usage)
    });
    drop(child);
    let code = if r < 0 {
        -1
    } else if libc::WIFEXITED(status) {
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

/// The processor time this program has spent, in milliseconds.
pub fn cpu_self() -> i64 {
    // SAFETY: `usage` is a plain C value the call fills.
    let usage = unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &mut usage);
        usage
    };
    let ms = |t: libc::timeval| t.tv_sec as i64 * 1000 + t.tv_usec as i64 / 1000;
    ms(usage.ru_utime) + ms(usage.ru_stime)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn a_started_program_can_be_stopped_though_this_one_blocks_stops() {
        // SAFETY: blocks `SIGTERM` in this test's thread only, through a
        // set filled in here.
        unsafe {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, libc::SIGTERM);
            libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
        }
        // The shell stops itself: `$$` is its own id.
        let mut command = Command::new("sh");
        command.args(["-c", "kill -TERM $$; exit 3"]);
        let status = unblocked(&mut command).status().unwrap();
        assert_eq!(status.signal(), Some(libc::SIGTERM));
    }

    #[test]
    fn a_group_is_ended_while_it_runs_and_never_signalled_once_reaped() {
        let (child, tree) = spawn(Command::new("sleep").arg("60")).unwrap();
        tree.kill();
        let exit = wait(child, &tree);
        assert_eq!(exit.code, 128 + i64::from(libc::SIGKILL));
        // Reaped: its id may be a stranger's group now.
        assert!(!tree.leader.signal(|| panic!("signalled after the reap")));
    }

    #[test]
    fn a_wait_that_fails_is_not_a_success() {
        let (mut child, tree) = spawn(&mut Command::new("true")).unwrap();
        // Reaped here first, so `wait` finds no program to wait for.
        assert!(child.wait().unwrap().success());
        assert_eq!(wait(child, &tree).code, -1);
        assert!(!tree.leader.signal(|| panic!("signalled after the reap")));
    }
}
