use super::*;

fn guard(mode: Mode, state: Option<PathBuf>) -> Guard {
    let settings = Settings {
        name: "Bot".into(),
        mode,
        audit: None,
        state,
        audit_json: false,
        otlp: None,
    };
    let rules = vec![
        Rule::new("Payments.refund", "amount <= 100", None),
        Rule::new("Payments", "count <= 2", Some("day")),
    ];
    Guard::new(settings, rules)
}

#[test]
fn a_rule_covers_its_method_or_its_whole_capability() {
    for (target, call, covered) in [
        ("Payments.refund", "Payments.refund", true),
        ("Payments", "Payments.refund", true),
        ("Payments.charge", "Payments.refund", false),
        ("Pay", "Payments.refund", false),
        ("Ai", "Ai.ask", true),
    ] {
        assert_eq!(covers(target, call), covered, "{target} {call}");
    }
}

#[test]
fn numbers_of_one_kind_add_up() {
    let d = |s: &str| Num::Decimal(Decimal::parse(s).unwrap());
    assert_eq!(Num::Int(2).add(&Num::Int(3)), Some(Num::Int(5)));
    assert_eq!(Num::Int(i64::MAX).add(&Num::Int(1)), None);
    assert_eq!(d("0.1").add(&d("0.2")), Some(d("0.3")));
    assert_eq!(Num::Int(1).add(&Num::Float(1.0)), None);
}

#[test]
fn only_calls_that_ran_count() {
    let mut g = guard(Mode::Enforce, None);
    assert_eq!(g.rules_for("Payments.refund"), vec![0, 1]);
    for _ in 0..2 {
        let mut c = g.begin("Payments.refund", "refund()");
        assert!(!g.limit(&mut c, 1, true, true, None));
        g.record(&c);
    }
    // A checked call that never ran doesn't count.
    let mut c = g.begin("Payments.refund", "refund()");
    assert!(!g.limit(&mut c, 1, true, true, None));
    assert_eq!(g.totals(1), (None, 2));
}

#[test]
fn modes_decide_what_a_broken_limit_does() {
    let g = guard(Mode::Observe, None);
    let mut c = g.begin("Payments.refund", "refund()");
    assert!(!g.limit(&mut c, 0, false, false, None));
    assert_eq!(c.notes, ["would be refused (limit broken: amount <= 100)"]);
    assert_eq!(c.refused, None);

    let g = guard(Mode::Enforce, None);
    let mut c = g.begin("Payments.refund", "refund()");
    assert!(g.limit(&mut c, 0, false, false, None));
    assert_eq!(c.refused.as_deref(), Some("limit broken: amount <= 100"));
}

/// With no folder to keep a request in, and no terminal, no one can
/// answer: the call is refused at once (decision 0072).
#[test]
fn without_a_terminal_no_one_approves() {
    let g = guard(Mode::Suggest, None);
    let mut c = g.begin("Payments.refund", "refund()");
    g.need(&mut c, "finance", "amount > 5", None);
    // Tests run without a terminal on standard input.
    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        assert_eq!(g.step(&mut c, None), Step::Done(true));
        assert_eq!(
            c.refused.as_deref(),
            Some("not approved by finance (amount > 5)")
        );
    }
}

#[test]
fn totals_survive_in_the_state_file() {
    let dir = std::env::temp_dir().join(format!("kavach-agents-{}", std::process::id()));
    let path = dir.join("state.json");
    let mut g = guard(Mode::Enforce, Some(path.clone()));
    let mut c = g.begin("Payments.refund", "refund()");
    g.limit(
        &mut c,
        1,
        true,
        true,
        Some(Num::Decimal(Decimal::parse("2.5").unwrap())),
    );
    g.record(&c);
    let mut again = guard(Mode::Enforce, Some(path));
    let total = Some(Num::Decimal(Decimal::parse("2.5").unwrap()));
    assert_eq!(again.totals(1), (total, 1));
    assert_eq!(again.totals(0), (None, 0));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Calls saved long before the program (and the machine) started still
/// count, and one the clock says is to come counts as made now.
#[test]
fn saved_calls_count_after_a_reboot_and_a_clock_set_back() {
    let dir = std::env::temp_dir().join(format!("kavach-agents-old-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("state.json");
    let day = 86_400_000;
    let (old, ahead) = (unix_ms() - 29 * day, unix_ms() + day);
    let key = "Payments: count <= 5 per month";
    std::fs::write(
        &path,
        format!("{{\"{key}\": [[{old}, null], [{ahead}, null]]}}"),
    )
    .unwrap();
    let settings = Settings {
        name: "Bot".into(),
        mode: Mode::Enforce,
        audit: None,
        state: Some(path),
        audit_json: false,
        otlp: None,
    };
    let rules = vec![Rule::new("Payments", "count <= 5", Some("month"))];
    let mut g = Guard::new(settings, rules);
    assert_eq!(g.totals(0), (None, 2));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A call counts from its start (decision 0062): one that fails stops
/// counting, and one a crash cut off is still counted after a restart.
#[test]
fn a_call_counts_from_its_start() {
    let dir = std::env::temp_dir().join(format!("kavach-agents-start-{}", std::process::id()));
    let path = dir.join("state.json");
    let mut g = guard(Mode::Enforce, Some(path.clone()));
    let mut c = g.begin("Payments.refund", "refund()");
    g.limit(&mut c, 1, true, true, None);
    g.start(&mut c);
    assert_eq!(g.totals(1), (None, 1));
    g.unrecord(&c);
    assert_eq!(g.totals(1), (None, 0));
    let mut c = g.begin("Payments.refund", "refund()");
    g.limit(&mut c, 1, true, true, None);
    g.start(&mut c);
    // The program stops here, while the call runs.
    let mut again = guard(Mode::Enforce, Some(path));
    assert_eq!(again.totals(1), (None, 1));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn utc_time_is_written_like_iso_8601() {
    let t = utc_now();
    assert_eq!(t.len(), 20, "{t}");
    assert!(t.ends_with('Z') && t.as_bytes()[10] == b'T', "{t}");
}
