//! Another program answers the requests in code (spec 14.16).

use crate::setup::*;

/// Another program in the project answers in code (spec 14.16).
const ANSWERS: &str = "use std.agents

fn main() ! Error uses Approvals, Console {
    const found = agents.waiting()
    for r in found {
        print(\"{r.id} {r.agent_name} {r.call} by {r.by} when {r.why}\")
    }
    try agents.answer(found[0].id, approved: true, by: \"asha\")
    agents.answer(found[0].id, approved: false, by: \"ravi\") catch e => {
        print(\"again: {e}\")
    }
}
";

fn answered_in_code(way: Way) {
    // Short, so a failed answer ends the test instead of hanging it.
    let dir = project(&format!("code-{way:?}"), "30.seconds");
    // A project of its own, such as a chat bot's, with the shop's agents.
    let answerer = dir.join("answerer");
    std::fs::create_dir_all(answerer.join("src")).unwrap();
    let toml = "[package]\nname = \"answerer\"\n\n[agents]\nfolder = \"../agents\"\n";
    std::fs::write(answerer.join("kavach.toml"), toml).unwrap();
    std::fs::write(answerer.join("src/main.kav"), ANSWERS).unwrap();
    let mut child = start(&dir, way, &["send"]);
    let id = request_id(&mut child);
    let mut answers = kavach();
    if let Way::Native = way {
        answers.args(["run", "--native"]);
    } else {
        answers.arg("run");
    }
    let out = answers
        .arg(answerer.join("src/main.kav"))
        .current_dir(&answerer)
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!(
            "{id} Bot Payments.pay(amount: 50) by finance when amount > 10\nagain: NotWaiting(id: \"{id}\")\n"
        ),
        "{way:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        output(child),
        ("[pay] 50\npaid 50\ndone\n".to_string(), Some(0)),
        "{way:?}"
    );
    let audit = std::fs::read_to_string(dir.join("agents/Bot/audit.log")).unwrap();
    assert!(audit.contains("approved by asha for finance"), "{audit}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_program_answers_in_code() {
    answered_in_code(Way::Interpreted);
}

#[test]
fn native_code_answers_in_code_too() {
    answered_in_code(Way::Native);
}
