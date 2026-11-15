//! Once tests run, no action changes the machine (spec 14.25): each is
//! refused before any system call. In its own process, so the switch
//! doesn't reach the other tests. Nothing here can end a program or change
//! a user: every call is refused first.

use kavach_runtime_process::acts::{act, forbid, forbidden};

#[test]
fn while_tests_run_every_action_is_refused() {
    forbid();
    assert!(forbidden());
    for (which, text, number) in [
        (0, "", 1),
        (0, "force", 1),
        (1, "", 80),
        (2, "kavach-test-user\0Pw-1234567!\0admin", 0),
        (3, "kavach-test-user", 0),
        (4, "", 0),
    ] {
        let outcome = act(which, text, number);
        assert!(
            outcome.starts_with("Denied:a test can't change the machine"),
            "{which}: {outcome}"
        );
    }
}
