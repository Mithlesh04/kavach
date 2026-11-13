//! Approvals that wait (spec 8.3, decision 0072): a request waits in the
//! agent's folder until `kavach approve` or `kavach deny` answers it, or
//! its time runs out; a program stopped while it waits finds the answer
//! when it starts again.

mod in_code;
mod restarts;
mod setup;

use setup::*;

fn approve_and_deny(way: Way) {
    let dir = project(&format!("answer-{way:?}"), "1.hours");
    let mut child = start(&dir, way, &["send"]);
    let id = request_id(&mut child);
    assert!(id.starts_with("Bot-1-"), "{id}");
    assert_eq!(
        answer(&dir, &["approve", &id, "--as", "asha"]),
        format!("{id}: approved\n")
    );
    assert_eq!(
        output(child),
        ("[pay] 50\npaid 50\ndone\n".to_string(), Some(0)),
        "{way:?}"
    );
    let audit = std::fs::read_to_string(dir.join("agents/Bot/audit.log")).unwrap();
    assert!(
        audit.contains("approved by asha for finance (amount > 10)"),
        "{audit}"
    );

    let mut child = start(&dir, way, &["send"]);
    let id = request_id(&mut child);
    answer(
        &dir,
        &["deny", &id, "--as", "ravi", "--reason", "duplicate"],
    );
    let (said, code) = output(child);
    assert_eq!(code, Some(0), "{way:?}: {said}");
    assert!(
        said.contains("not approved by finance (amount > 10): duplicate"),
        "{said}"
    );
    assert!(
        !said.contains("[pay]"),
        "{way:?}: paid without approval: {said}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_person_approves_or_denies_a_waiting_call() {
    approve_and_deny(Way::Interpreted);
}

#[test]
fn native_code_waits_for_the_answer_too() {
    approve_and_deny(Way::Native);
}

fn deadline(way: Way) {
    let dir = project(&format!("deadline-{way:?}"), "1.seconds");
    let child = start(&dir, way, &["send"]);
    let (said, code) = output(child);
    assert_eq!(code, Some(0), "{way:?}: {said}");
    assert!(
        said.contains("not approved by finance (amount > 10) within 1 second"),
        "{said}"
    );
    assert!(!said.contains("[pay]"), "{said}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_request_runs_out_of_time() {
    deadline(Way::Interpreted);
    deadline(Way::Native);
}
