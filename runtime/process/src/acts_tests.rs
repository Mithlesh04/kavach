//! What changes the machine (spec 14.25), tested so the machine that runs
//! the tests never changes. Outside a container, a test only ends a program
//! it started itself (kept unreaped, so its id can't be another program's),
//! and checks what is refused before any system call. Freeing ports, and
//! adding and removing users, run only in a throwaway container: both
//! `KAVACH_ISOLATED=1` and Docker's own `/.dockerenv` must be there. The
//! session is never locked. `tests/forbidden.rs` checks that tests can't
//! change the machine at all.

use crate::acts::act;
use crate::parts::rows;

/// Whether this is a throwaway container: asked for, and really one.
fn isolated() -> bool {
    std::env::var("KAVACH_ISOLATED").as_deref() == Ok("1")
        && std::path::Path::new("/.dockerenv").exists()
}

/// Whether this program runs with administrator or root rights.
fn admin() -> bool {
    rows(16).get(2).is_some_and(|a| a == "true")
}

/// A program that waits about a minute, started by this test.
fn sleeper() -> std::process::Child {
    let mut c = if cfg!(windows) {
        let mut c = std::process::Command::new("ping");
        c.args(["-n", "60", "127.0.0.1"]);
        c
    } else {
        let mut c = std::process::Command::new("sleep");
        c.arg("60");
        c
    };
    c.stdout(std::process::Stdio::null())
        .spawn()
        .expect("a program to end")
}

#[test]
fn a_program_this_test_started_is_ended() {
    let mut child = sleeper();
    let pid = child.id() as i64;
    assert!(
        rows(14).chunks(4).any(|r| r[0] == pid.to_string()),
        "listed while it runs"
    );
    // `child` isn't waited for until after: its id stays its own.
    assert_eq!(act(0, "", pid), "");
    let status = child.wait().expect("its end");
    assert!(!status.success(), "it was ended, not finished");
}

#[test]
fn what_cant_be_is_refused_before_any_system_call() {
    // No program has this id on any system: Linux's ids fit in 32 bits
    // with a sign, Windows refuses it.
    assert!(act(0, "", 4_000_000_000).starts_with("NotFound:"));
    assert!(
        act(0, "", -1).starts_with("NotFound:"),
        "never a group of programs"
    );
    assert!(
        act(0, "", 0).starts_with("NotFound:"),
        "never this program's group"
    );
    assert!(act(9, "", 0).starts_with("Failed:"), "no such action");
    assert!(
        act(2, "bad/name\0pw", 0).starts_with("Failed:"),
        "not a user's name"
    );
}

/// Only in a throwaway container: a port held by a program this test
/// started is freed, and a port nothing holds ends nothing.
#[test]
fn in_a_throwaway_container_a_port_is_freed() {
    if !isolated() {
        return;
    }
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    assert_eq!(act(1, "", port as i64), "ended:");
}

/// Only in a throwaway container: a user is added, found and removed as
/// root, and refused to anyone else.
#[test]
fn in_a_throwaway_container_users_are_added_and_removed() {
    if !isolated() {
        return;
    }
    let name = "kavach-test-user";
    if !admin() {
        assert!(act(2, &format!("{name}\0Pw-1234567!"), 0).starts_with("Denied:"));
        assert!(act(3, name, 0).starts_with("Denied:"));
        return;
    }
    assert_eq!(act(2, &format!("{name}\0Pw-1234567!\0admin"), 0), "");
    let users = rows(17);
    let row = users
        .chunks(3)
        .find(|r| r[0] == name)
        .expect("the new user listed");
    assert_eq!(row[2], "true", "an administrator");
    assert!(
        act(2, &format!("{name}\0x"), 0).starts_with("Failed:"),
        "it exists already"
    );
    assert_eq!(act(3, name, 0), "");
    assert!(!rows(17).chunks(3).any(|r| r[0] == name), "gone");
    assert!(act(3, name, 0).starts_with("NotFound:"), "gone already");
}
