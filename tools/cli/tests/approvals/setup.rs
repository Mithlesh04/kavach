//! What every approvals test needs: the program, a project for it, and
//! starting, answering and reading it.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

pub const PROGRAM: &str = "use std.cli
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

pub fn main() ! Error uses Console, Env, Clock {
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
pub enum Way {
    Interpreted,
    Native,
}

pub fn kavach() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kavach"))
}

/// A project whose agent's requests wait `wait` (Kavach, `1.hours`).
pub fn project(name: &str, wait: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kavach-approvals-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"shop\"\n").unwrap();
    std::fs::write(dir.join("src/main.kav"), PROGRAM.replace("WAIT", wait)).unwrap();
    dir
}

/// The program started in `dir`, with standard input not a terminal, so
/// its requests wait for an answer from outside.
pub fn start(dir: &Path, way: Way, args: &[&str]) -> Child {
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
pub fn request_id(child: &mut Child) -> String {
    let mut line = String::new();
    BufReader::new(child.stderr.as_mut().unwrap())
        .read_line(&mut line)
        .unwrap();
    let start = line.find("(request ").expect(&line) + "(request ".len();
    line[start..].split(')').next().unwrap().to_string()
}

pub fn answer(dir: &Path, words: &[&str]) -> String {
    let out = kavach().args(words).current_dir(dir).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

pub fn output(child: Child) -> (String, Option<i32>) {
    let out = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

/// Waits until `n` requests wait for a person.
pub fn wait_for_requests(dir: &Path, n: usize) {
    for _ in 0..600 {
        if answer(dir, &["approvals"]).lines().count() == n {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("{n} requests never came: {}", answer(dir, &["approvals"]));
}
