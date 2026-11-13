//! Approvals across a restart (spec 8.3, 8.6, decision 0072): an answer
//! given while the program was down, events cut short, joined and equal
//! events run again.

use crate::setup::*;
use std::io::{BufRead, BufReader};

fn across_a_restart(way: Way) {
    let dir = project(&format!("restart-{way:?}"), "1.hours");
    let mut child = start(&dir, way, &["send"]);
    let id = request_id(&mut child);
    // The program stops while it waits.
    let _ = child.kill();
    let _ = child.wait();
    let listed = answer(&dir, &["approvals"]);
    assert!(
        listed.starts_with(&format!("{id}  Payments.pay(amount: 50)  by finance")),
        "{listed}"
    );
    answer(&dir, &["approve", &id, "--as", "asha"]);
    // Starting again finishes the event: the answer is there.
    let (said, code) = output(start(&dir, way, &[]));
    assert_eq!(
        (said.as_str(), code),
        ("[pay] 50\ndone\n", Some(0)),
        "{way:?}"
    );
    assert_eq!(
        answer(&dir, &["approvals"]),
        "no requests wait for approval\n"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A program that ends by itself (`cli.exit`) while an event waits
/// leaves the event unfinished, as a crash does: it finishes next time.
fn ended_by_itself(way: Way) {
    let dir = project(&format!("exit-{way:?}"), "1.hours");
    let mut child = start(&dir, way, &["exit"]);
    let id = request_id(&mut child);
    assert_eq!(output(child), (String::new(), Some(3)), "{way:?}");
    answer(&dir, &["approve", &id, "--as", "asha"]);
    let (said, code) = output(start(&dir, way, &[]));
    assert_eq!(
        (said.as_str(), code),
        (
            "[pay] 50
done
",
            Some(0)
        ),
        "{way:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_event_cut_short_by_the_program_ending_finishes_next_time() {
    ended_by_itself(Way::Interpreted);
    ended_by_itself(Way::Native);
}

#[test]
fn an_answer_given_while_the_program_was_down_is_found() {
    across_a_restart(Way::Interpreted);
}

#[test]
fn native_code_finds_it_too() {
    across_a_restart(Way::Native);
}

/// A send that joined a replayed event fails with its panic (spec 8.6,
/// decision 0076), though the handler has no error of its own.
fn joined_event_panics(way: Way) {
    let dir = project(&format!("join-panic-{way:?}"), "1.hours");
    let mut child = start(&dir, way, &["boom"]);
    let id = request_id(&mut child);
    let _ = child.kill();
    let _ = child.wait();
    let mut again = start(&dir, way, &["again", "boom"]);
    // After `start`, so the send joins the event, not a new one of its own.
    let mut first = String::new();
    BufReader::new(again.stdout.as_mut().unwrap())
        .read_line(&mut first)
        .unwrap();
    assert_eq!(
        first,
        "started
",
        "{way:?}: start waited for the person"
    );
    answer(&dir, &["approve", &id, "--as", "asha"]);
    let (said, code) = output(again);
    assert_eq!(code, Some(0), "{way:?}: {said}");
    assert!(
        said.contains("failed: panicked: index 50 is out of range"),
        "{way:?}: {said}"
    );
    let audit = std::fs::read_to_string(dir.join("agents/Bot/audit.log")).unwrap();
    assert!(
        audit.contains("boom panicked: index 50 is out of range"),
        "{way:?}: {audit}"
    );
    assert!(
        audit.contains("finished after a restart: event #1 of boom: error panicked: index 50"),
        "{way:?}: {audit}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_joined_send_gets_the_panic() {
    joined_event_panics(Way::Interpreted);
}

#[test]
fn native_code_gives_a_joined_send_the_panic_too() {
    joined_event_panics(Way::Native);
}

/// Two equal events left unfinished each run again and ask a person: a
/// replayed event never joins the other (spec 8.3).
fn equal_events_each_run_again(way: Way) {
    let dir = project(&format!("equal-{way:?}"), "1.hours");
    let mut child = start(&dir, way, &["twice"]);
    let _ = request_id(&mut child);
    wait_for_requests(&dir, 2);
    let _ = child.kill();
    let _ = child.wait();
    let mut again = start(&dir, way, &["again"]);
    wait_for_requests(&dir, 2);
    let listed = answer(&dir, &["approvals"]);
    let mut refused = Vec::new();
    for line in listed.lines() {
        let id = line.split(' ').next().unwrap();
        // The listed requests are on disk from the first run, so the
        // restarted program may still be reading its journal: an answer
        // is tried again for up to 2 s.
        let approved = (0..20).any(|_| {
            let out = kavach()
                .args(["approve", id, "--as", "asha"])
                .current_dir(&dir)
                .output()
                .unwrap();
            if !out.status.success() {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            out.status.success()
        });
        if !approved {
            refused.push(id.to_string());
        }
    }
    // Ended before asserting, so a failure doesn't leave it waiting.
    let _ = again.kill();
    let _ = again.wait();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(refused.is_empty(), "{way:?}: not waiting: {refused:?}");
}

#[test]
fn equal_events_each_run_again_after_a_restart() {
    equal_events_each_run_again(Way::Interpreted);
}

#[test]
fn native_code_runs_equal_events_again_too() {
    equal_events_each_run_again(Way::Native);
}

/// After a restart, `start` returns while the event waits for a person,
/// and sending the same event again joins it (decision 0072).
fn joined_after_a_restart(way: Way) {
    let dir = project(&format!("join-{way:?}"), "1.hours");
    let mut child = start(&dir, way, &["send"]);
    let id = request_id(&mut child);
    let _ = child.kill();
    let _ = child.wait();
    let mut again = start(&dir, way, &["again", "send"]);
    let mut first = String::new();
    BufReader::new(again.stdout.as_mut().unwrap())
        .read_line(&mut first)
        .unwrap();
    assert_eq!(
        first,
        "started
",
        "{way:?}: start waited for the person"
    );
    // Still one request: the second send joined the first.
    let listed = answer(&dir, &["approvals"]);
    assert_eq!(listed.lines().count(), 1, "{way:?}: {listed}");
    assert!(listed.starts_with(&format!("{id} ")), "{way:?}: {listed}");
    answer(&dir, &["approve", &id, "--as", "asha"]);
    assert_eq!(
        output(again),
        (
            "[pay] 50
paid 50
done
"
            .to_string(),
            Some(0)
        ),
        "{way:?}"
    );
    let audit = std::fs::read_to_string(dir.join("agents/Bot/audit.log")).unwrap();
    assert!(
        audit.contains("finished after a restart: event #1 of request: ok 50"),
        "{way:?}: {audit}"
    );
    assert_eq!(
        audit.matches("Payments.pay(amount: 50)").count(),
        1,
        "{audit}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn start_returns_and_the_same_event_joins_it() {
    joined_after_a_restart(Way::Interpreted);
}

#[test]
fn native_code_joins_it_too() {
    joined_after_a_restart(Way::Native);
}
