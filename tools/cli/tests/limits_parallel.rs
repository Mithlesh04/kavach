//! A limit's total holds when many handlers call at once (spec 8.3,
//! 8.4): 30 refunds of 100 started together against `sum(amount) <= 1000
//! per day` pay exactly 10, in the interpreter and native code, every
//! time. Each call is counted when it starts, so two can't both slip
//! under the total.

use std::process::Command;

const PROGRAM: &str = "type Receipt {
    id: Int
}
type PayError = Failed | Denied(reason: String)

pub capability Payments {
    fn refund(order_id: Int, amount: Int) -> Receipt ! PayError
}

type Books: Payments {
    fn refund(self, order_id: Int, amount: Int) -> Receipt ! PayError {
        Receipt(id: order_id)
    }
}

agent Desk {
    can Payments.refund
    limit Payments.refund {
        sum(amount) <= 1000 per day
    }
    on request(id: Int) -> Bool ! Error {
        const _r = Payments.refund(id, 100) catch _ => {
            return false
        }
        true
    }
}

fn main() ! Error uses Console, Payments.refund {
    const d = Desk.start(Payments: Books())
    let asked: [Task<Bool>] = []
    for i in 0..30 {
        asked.push(spawn d.request(i))
    }
    let paid = 0
    for a in asked {
        if try a.wait() {
            paid += 1
        }
    }
    print(\"paid {paid}\")
}
";

#[test]
fn a_total_holds_when_thirty_handlers_pay_at_once() {
    for native in [false, true] {
        for round in 0..3 {
            let dir = std::env::temp_dir().join(format!(
                "kavach-limits-{}-{native}-{round}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            // No project and no folder: the total lives only while the
            // program runs, so each round starts from 0.
            let file = dir.join("desk.kav");
            std::fs::write(&file, PROGRAM).unwrap();
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
            cmd.arg("run");
            if native {
                cmd.arg("--native");
            }
            let out = cmd.arg(&file).current_dir(&dir).output().unwrap();
            std::fs::remove_dir_all(&dir).unwrap();
            let text = String::from_utf8_lossy(&out.stdout).into_owned()
                + &String::from_utf8_lossy(&out.stderr);
            assert_eq!(text, "paid 10\n", "native: {native}, round {round}");
        }
    }
}
