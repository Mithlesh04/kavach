//! Changing the machine on Linux and macOS (spec 14.25): `kill` for
//! programs, the system's own `useradd`, `userdel` and `chpasswd` for
//! users (run with arguments, no shell), and `loginctl` to lock.
#![allow(unsafe_code)]

use super::Refused;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn terminate(pid: i64, force: bool) -> Result<(), Refused> {
    let Ok(pid) = i32::try_from(pid) else {
        return Err(Refused::NotFound(format!("program {pid}")));
    };
    if pid <= 0 {
        return Err(Refused::NotFound(format!("program {pid}")));
    }
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    // SAFETY: sends a signal to one process, `pid` > 0, never a group.
    if unsafe { libc::kill(pid, signal) } == 0 {
        return Ok(());
    }
    match std::io::Error::last_os_error().raw_os_error() {
        Some(libc::ESRCH) => Err(Refused::NotFound(format!("program {pid}"))),
        Some(libc::EPERM) => Err(Refused::Denied(format!(
            "program {pid} belongs to another user"
        ))),
        _ => Err(Refused::Failed(std::io::Error::last_os_error().to_string())),
    }
}

fn root() -> bool {
    // SAFETY: takes nothing and can't fail.
    unsafe { libc::geteuid() == 0 }
}

/// Runs `program` with `args`, giving it `input`; its error text if it fails.
fn run(program: &str, args: &[&str], input: Option<&str>) -> Result<(), Refused> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Refused::Failed(format!("{program}: {e}")))?;
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        let _ = stdin.write_all(text.as_bytes());
    }
    let out = child
        .wait_with_output()
        .map_err(|e| Refused::Failed(e.to_string()))?;
    if out.status.success() {
        return Ok(());
    }
    let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if why.contains("already exists") {
        return Err(Refused::Failed(why));
    }
    if why.contains("does not exist") {
        return Err(Refused::NotFound(why));
    }
    Err(Refused::Failed(if why.is_empty() {
        format!("{program} failed")
    } else {
        why
    }))
}

pub fn add_user(name: &str, password: &str, admin: bool) -> Result<(), Refused> {
    if !root() {
        return Err(Refused::Denied("adding a user needs root".into()));
    }
    // The administrators' group: `sudo` on Debian and Ubuntu, `wheel` elsewhere.
    let groups = std::fs::read_to_string("/etc/group").unwrap_or_default();
    let group = ["sudo", "wheel"]
        .into_iter()
        .find(|g| groups.lines().any(|l| l.starts_with(&format!("{g}:"))));
    let mut args = vec!["-m", "-s", "/bin/sh"];
    if let (true, Some(g)) = (admin, group) {
        args.extend(["-G", g]);
    }
    args.extend(["--", name]);
    run("useradd", &args, None)?;
    if !password.is_empty() {
        run("chpasswd", &[], Some(&format!("{name}:{password}\n")))?;
    }
    Ok(())
}

pub fn remove_user(name: &str) -> Result<(), Refused> {
    if !root() {
        return Err(Refused::Denied("removing a user needs root".into()));
    }
    run("userdel", &["-r", "--", name], None)
}

pub fn lock() -> Result<(), Refused> {
    run("loginctl", &["lock-session"], None)
}
