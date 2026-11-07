//! Events of one durable agent that are under way at once (spec 8.5):
//! each call goes to the journal under its own event, whichever task
//! makes it, so a restart gives each event its own outcomes.

use std::process::Command;

const PROGRAM: &str = "use std.time

type PayError = Declined | Denied(reason: String)

capability Payments {
    fn pay(amount: Int) -> Int ! PayError
}

type Till: Payments {
    fn pay(self, amount: Int) -> Int ! PayError {
        amount
    }
}

agent Bot {
    can Payments.pay, Clock

    on request(n: Int) -> Int ! Error {
        const a = try Payments.pay(n)
        time.sleep(100.ms)          // the other event goes on meanwhile
        const b = try Payments.pay(n * 10)
        a + b
    }
}

fn main() ! Error uses Console {
    const bot = Bot.start(Payments: Till())
    const first = spawn bot.request(1)
    const second = spawn bot.request(2)
    print(try first.wait())
    print(try second.wait())
}
";

fn journal(name: &str, native: bool) -> (String, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("kavach-overlap-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"overlap\"\n").unwrap();
    std::fs::write(dir.join("src/main.kav"), PROGRAM).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.arg("run");
    if native {
        cmd.arg("--native");
    }
    let out = cmd.arg(dir.join("src/main.kav")).output().unwrap();
    let said =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    let lines = std::fs::read_to_string(dir.join("agents/Bot/journal"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    (said, lines)
}

fn each_call_under_its_event(name: &str, native: bool) {
    let (said, lines) = journal(name, native);
    assert_eq!(said, "11\n22\n", "{name}");
    let at = |line: &str| lines.iter().position(|l| l == line);
    for (event, second) in [(1, 10), (2, 20)] {
        let call = format!("{{\"of\":{event},\"call\":\"Payments.pay\",\"ok\":{second}}}");
        let done = format!("{{\"done\":{event}}}");
        let (Some(call), Some(done)) = (at(&call), at(&done)) else {
            panic!("{name}: event {event}'s second call or end is missing: {lines:#?}");
        };
        assert!(
            call < done,
            "{name}: event {event} ended before its call: {lines:#?}"
        );
    }
}

#[test]
fn each_call_goes_under_its_own_event() {
    each_call_under_its_event("interp", false);
}

#[test]
fn native_code_keeps_them_apart_too() {
    each_call_under_its_event("native", true);
}
