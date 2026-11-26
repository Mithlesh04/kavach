// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What changes the machine (spec 14.25, decision 0112): ending programs,
//! freeing ports, adding and removing users, locking the session. Each
//! needs `System.critical.write` in the program, and the system's own
//! permission. The outcome is text: `""` when done, otherwise
//! `"NotFound:…"`, `"Denied:…"` or `"Failed:…"`, which `system.kav` turns
//! into a `SystemError`; `free_port` gives `"ended:"` and the pids.

#[cfg(unix)]
#[path = "acts_unix.rs"]
mod system;
#[cfg(windows)]
#[path = "acts_windows.rs"]
mod system;

/// Set while tests run: then no action changes the machine (spec 14.25).
static FORBIDDEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// From now on in this process, every action is refused: tests run here,
/// on a machine people use, so nothing may end a program, free a port,
/// change a user or lock the session, whatever the test calls.
pub fn forbid() {
    FORBIDDEN.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Whether actions are refused in this process.
pub fn forbidden() -> bool {
    FORBIDDEN.load(std::sync::atomic::Ordering::SeqCst)
}

/// Why an action wasn't done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    NotFound(String),
    Denied(String),
    Failed(String),
}

fn outcome(r: Result<String, Refused>) -> String {
    match r {
        Ok(done) => done,
        Err(Refused::NotFound(w)) => format!("NotFound:{w}"),
        Err(Refused::Denied(w)) => format!("Denied:{w}"),
        Err(Refused::Failed(w)) => format!("Failed:{w}"),
    }
}

/// Action `which` with its text and number: 0 ends program `number`
/// (`text` "force" to force it), 1 frees port `number` (the same), 2 adds
/// the user `name`, `password` and `admin` in `text`, separated by zero
/// characters, 3 removes user `text`, 4 locks the session.
pub fn act(which: i64, text: &str, number: i64) -> String {
    if forbidden() {
        return "Denied:a test can't change the machine (spec 14.25)".to_string();
    }
    let force = text == "force";
    outcome(match which {
        0 => system::terminate(number, force).map(|()| String::new()),
        1 => free_port(number, force),
        2 => account(text).and_then(|(name, password, admin)| {
            system::add_user(name, password, admin).map(|()| String::new())
        }),
        3 => old_user(text)
            .and_then(system::remove_user)
            .map(|()| String::new()),
        4 => system::lock().map(|()| String::new()),
        _ => Err(Refused::Failed("no such action".into())),
    })
}

/// Ends every program listening on `port`; gives their pids.
fn free_port(port: i64, force: bool) -> Result<String, Refused> {
    let rows = crate::parts::port_rows().map_err(Refused::Failed)?;
    let holders: Vec<&[String]> = (rows.chunks(6))
        .filter(|r| r[2] == port.to_string() && (r[3] == "listen" || r[0] == "udp"))
        .collect();
    if holders.iter().any(|r| unseen(r)) {
        return Err(Refused::Denied(format!(
            "port {port} is held by a program this user can't see or end"
        )));
    }
    let mut pids: Vec<i64> = (holders.iter())
        .filter_map(|r| r[4].parse().ok())
        .filter(|pid| *pid > 0 && *pid != std::process::id() as i64)
        .collect();
    pids.sort();
    pids.dedup();
    for pid in &pids {
        system::terminate(*pid, force)?;
    }
    let list: Vec<String> = pids.iter().map(i64::to_string).collect();
    Ok(format!("ended:{}", list.join(",")))
}

/// A port's holder this user can't see: it holds the port, and this user
/// can't end it (spec 14.25). Windows shows it with pid 0, Linux with no
/// pid at all.
fn unseen(row: &[String]) -> bool {
    row[4].is_empty() || row[4] == "0"
}

/// Whether `name` may name a new user (spec 14.25): 1 to 20 characters,
/// ASCII letters, digits, `_`, `-` and `.`, the first a letter or `_`.
/// Every system takes such a name: 20 is Windows' limit, and Linux's
/// `useradd` takes these characters.
fn user_name(name: &str) -> bool {
    let first = name.chars().next();
    (1..=20).contains(&name.len())
        && first.is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && (name.chars()).all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// The user `remove_user` removes: any name, as old accounts may have
/// others (spec 14.25), but never one holding a NUL. The system would
/// read it as the name's end, so `"guest\0x"` would remove `guest`.
fn old_user(name: &str) -> Result<&str, Refused> {
    if name.contains('\0') {
        return Err(Refused::Failed("a user's name can't hold a NUL".into()));
    }
    Ok(name)
}

/// The account `add_user` makes: `name`, `password` and `admin` or `""`,
/// NUL between them. A NUL inside one would add a part (`"pw\0admin"`
/// makes an administrator), and a line break ends a line the system's
/// tools read, so either is refused before anything is done (spec 14.25).
fn account(text: &str) -> Result<(&str, &str, bool), Refused> {
    let parts: Vec<&str> = text.split('\0').collect();
    match parts[..] {
        [name, _, _] if !user_name(name) => Err(Refused::Failed(format!(
            "`{name}` can't name a user: write 1 to 20 letters, digits, `_`, `-` or `.`, starting with a letter or `_`"
        ))),
        [_, password, _] if password.contains(['\n', '\r']) => Err(Refused::Failed(
            "a password can't hold a line break or a NUL".into(),
        )),
        [name, password, admin @ ("" | "admin")] => Ok((name, password, admin == "admin")),
        _ => Err(Refused::Failed(
            "a user's name and password can't hold a NUL".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_holder_with_no_pid_is_unseen_on_every_system() {
        let row = |pid: &str| -> Vec<String> {
            ["tcp", "0.0.0.0", "8081", "listen", pid, ""]
                .map(String::from)
                .to_vec()
        };
        // Linux leaves the pid empty, Windows writes 0; a seen one has its pid.
        assert!(unseen(&row("")));
        assert!(unseen(&row("0")));
        assert!(!unseen(&row("4242")));
    }

    #[test]
    fn an_account_is_three_parts_with_no_line_breaks() {
        assert_eq!(account("asha\0pw:1\0"), Ok(("asha", "pw:1", false)));
        assert_eq!(account("asha\0pw\0admin"), Ok(("asha", "pw", true)));
        for text in [
            "probe\0first\nroot:injected\0",
            "probe\0first\rroot:injected\0",
            "probe\0pw\0admin\0",
            "probe\0pw\0yes",
            "root:x\0pw\0",
            "a\nroot\0pw\0",
            "\0pw\0",
        ] {
            assert!(matches!(account(text), Err(Refused::Failed(_))), "{text:?}");
        }
    }

    #[test]
    fn a_user_to_remove_never_holds_a_nul() {
        // Only the check: nothing here removes a user.
        assert_eq!(old_user("Web Team"), Ok("Web Team"), "an old name");
        for bad in ["guest\0x", "\0", "guest\0"] {
            assert!(matches!(old_user(bad), Err(Refused::Failed(_))), "{bad:?}");
        }
    }

    #[test]
    fn a_user_name_is_short_and_plain() {
        for good in [
            "asha",
            "_build",
            "web-01",
            "Asha.K",
            "a",
            "abcdefghij0123456789",
        ] {
            assert!(user_name(good), "{good}");
        }
        for bad in [
            "",
            "1st",
            "-x",
            ".hidden",
            "a b",
            "a/b",
            "a\\b",
            "a:b",
            "ásha",
            "abcdefghij01234567890",
        ] {
            assert!(!user_name(bad), "{bad:?}");
        }
    }
}
