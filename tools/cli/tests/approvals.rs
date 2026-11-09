//! Approvals that wait (spec 8.3, decision 0072): a request waits in the
//! agent's folder until `kavach approve` or `kavach deny` answers it, or
//! its time runs out; a program stopped while it waits finds the answer
//! when it starts again.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const PROGRAM: &str = "use std.cli
use std.time

type PayError = Declined | Denied(reason: String)

capability Payments {
    fn pay(amount: Int) -> Int ! PayError
}

type Till: Payments {
    fn pay(self, amount: Int) -> Int ! PayError uses Console {
        print(\"[pay] {amount}\")
        amount
    }
}

agent Bot {
    settings AgentSettings(audit: Audit())

    can Payments.pay
    approve Payments.pay when amount > 10 by \"finance\" within WAIT

    on request(amount: Int) -> Int ! Error {
        try Payments.pay(amount)
    }

    // No error of its own: it panics after the person answers.
    on boom(amount: Int) -> Int {
        const paid = Payments.pay(amount) catch _ => 0
        [paid][amount]
    }
}

fn main() ! Error uses Console, Env, Clock {
    const bot = Bot.start(Payments: Till())
    if cli.args().contains(\"again\") {
        print(\"started\")
    }
    // The program ends by itself while the event waits for a person.
    if cli.args().contains(\"exit\") {
        const _w = spawn {
            bot.request(50) catch e => {
                print(\"refused: {e}\")
                0
            }
        }
        time.sleep(1.seconds)
        cli.exit(3)
    }
    if cli.args().contains(\"boom\") {
        const paid = bot.boom(50) catch e => {
            print(\"failed: {e}\")
            return
        }
        print(\"paid {paid}\")
    }
    if cli.args().contains(\"twice\") {
        const a = spawn bot.request(50)
        const b = spawn bot.request(50)
        print(\"paid {(try a.wait()) + (try b.wait())}\")
    }
    if cli.args().contains(\"send\") {
        const paid = bot.request(50) catch e => {
            print(\"refused: {e}\")
            return
        }
        print(\"paid {paid}\")
    }
    print(\"done\")
}
";

#[derive(Clone, Copy, Debug)]
enum Way {
    Interpreted,
    Native,
}

fn kavach() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kavach"))
}

/// A project whose agent's requests wait `wait` (Kavach, `1.hours`).
fn project(name: &str, wait: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kavach-approvals-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"shop\"\n").unwrap();
    std::fs::write(dir.join("src/main.kav"), PROGRAM.replace("WAIT", wait)).unwrap();
    dir
}

/// The program started in `dir`, with standard input not a terminal, so
/// its requests wait for an answer from outside.
fn start(dir: &Path, way: Way, args: &[&str]) -> Child {
    let mut cmd = kavach();
    match way {
        Way::Interpreted => cmd.arg("run"),
        Way::Native => cmd.args(["run", "--native"]),
    };
    cmd.arg(dir.join("src/main.kav"))
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

/// The number of the request the program says it waits on.
fn request_id(child: &mut Child) -> String {
    let mut line = String::new();
    BufReader::new(child.stderr.as_mut().unwrap())
        .read_line(&mut line)
        .unwrap();
    let start = line.find("(request ").expect(&line) + "(request ".len();
    line[start..].split(')').next().unwrap().to_string()
}

fn answer(dir: &Path, words: &[&str]) -> String {
    let out = kavach().args(words).current_dir(dir).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn output(child: Child) -> (String, Option<i32>) {
    let out = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

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
    for line in listed.lines() {
        let id = line.split(' ').next().unwrap();
        answer(&dir, &["approve", id, "--as", "asha"]);
    }
    let _ = again.kill();
    let _ = again.wait();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn equal_events_each_run_again_after_a_restart() {
    equal_events_each_run_again(Way::Interpreted);
}

#[test]
fn native_code_runs_equal_events_again_too() {
    equal_events_each_run_again(Way::Native);
}

/// Waits until `n` requests wait for a person.
fn wait_for_requests(dir: &Path, n: usize) {
    for _ in 0..600 {
        if answer(dir, &["approvals"]).lines().count() == n {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("{n} requests never came: {}", answer(dir, &["approvals"]));
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
