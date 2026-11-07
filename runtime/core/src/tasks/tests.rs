use super::*;
use std::time::Duration;

/// Starts task `body` as a thread taking turns in `s`.
fn start(
    s: &Arc<Scheduler>,
    body: impl FnOnce(&Scheduler, TaskId) + Send + 'static,
) -> (TaskId, std::thread::JoinHandle<()>) {
    let id = s.add();
    let s2 = Arc::clone(s);
    let handle = std::thread::spawn(move || {
        if s2.begin(id).is_ok() {
            body(&s2, id);
        }
        s2.end(id);
    });
    (id, handle)
}

type Log = Arc<Mutex<Vec<String>>>;

fn note(log: &Log, s: &str) {
    log.lock().unwrap().push(s.to_string());
}

#[test]
fn a_spawned_task_runs_when_the_running_one_waits() {
    let s = Scheduler::new();
    let log: Log = Arc::default();
    let l = Arc::clone(&log);
    let (a, ha) = start(&s, move |_, _| note(&l, "a"));
    let l = Arc::clone(&log);
    let (b, hb) = start(&s, move |_, _| note(&l, "b"));
    note(&log, "main");
    s.wait_for(0, b).unwrap();
    note(&log, "main again");
    assert!(s.is_done(a));
    s.end(0);
    ha.join().unwrap();
    hb.join().unwrap();
    assert_eq!(*log.lock().unwrap(), ["main", "a", "b", "main again"]);
}

#[test]
fn sleeping_tasks_overlap_and_wake_in_time_order() {
    let s = Scheduler::new();
    let log: Log = Arc::default();
    let started = Instant::now();
    let mut handles = Vec::new();
    let mut ids = Vec::new();
    for (name, ms) in [("slow", 120), ("fast", 60)] {
        let l = Arc::clone(&log);
        let (id, h) = start(&s, move |s, me| {
            s.sleep_until(me, started + Duration::from_millis(ms))
                .unwrap();
            note(&l, name);
        });
        ids.push(id);
        handles.push(h);
    }
    for id in ids {
        s.wait_for(0, id).unwrap();
    }
    let took = started.elapsed();
    s.end(0);
    handles.into_iter().for_each(|h| h.join().unwrap());
    assert_eq!(*log.lock().unwrap(), ["fast", "slow"]);
    assert!(took < Duration::from_millis(170), "took {took:?}");
}

#[test]
fn stopping_ends_every_waiting_task() {
    let s = Scheduler::new();
    let (t, h) = start(&s, |s, me| {
        let far = Instant::now() + Duration::from_secs(3600);
        assert_eq!(s.sleep_until(me, far), Err(Stop::Stopped));
    });
    // Let the task start sleeping, then end the program.
    s.sleep_until(0, Instant::now()).unwrap();
    s.stop();
    assert_eq!(s.wait_for(0, t), Err(Stop::Stopped));
    s.end(0);
    h.join().unwrap();
}

#[test]
fn a_parked_task_runs_again_when_unparked() {
    let s = Scheduler::new();
    let log: Log = Arc::default();
    let l = Arc::clone(&log);
    let (t, h) = start(&s, move |s, me| {
        note(&l, "parks");
        s.park(me).unwrap();
        note(&l, "unparked");
    });
    s.sleep_until(0, Instant::now()).unwrap();
    note(&log, "main unparks");
    s.unpark(t);
    s.wait_for(0, t).unwrap();
    s.end(0);
    h.join().unwrap();
    assert_eq!(*log.lock().unwrap(), ["parks", "main unparks", "unparked"]);
}

#[test]
fn parking_with_no_one_to_unpark_is_a_deadlock() {
    let s = Scheduler::new();
    assert_eq!(s.park(0), Err(Stop::Deadlock));
}

#[test]
fn a_cancelled_task_stops_waiting() {
    let s = Scheduler::new();
    let log: Log = Arc::default();
    let l = Arc::clone(&log);
    let (t, h) = start(&s, move |s, me| {
        let far = Instant::now() + Duration::from_secs(3600);
        note(&l, &format!("{:?}", s.sleep_until(me, far)));
        note(&l, &format!("{:?}", s.park(me)));
    });
    s.sleep_until(0, Instant::now()).unwrap();
    s.cancel(t);
    s.join(0, t).unwrap();
    s.end(0);
    h.join().unwrap();
    assert_eq!(*log.lock().unwrap(), ["Err(Cancelled)", "Err(Cancelled)"]);
}

#[test]
fn a_join_waits_even_when_cancelled() {
    let s = Scheduler::new();
    let (t, h) = start(&s, |s, me| {
        s.sleep_until(me, Instant::now() + Duration::from_millis(30))
            .unwrap();
    });
    let (u, hu) = start(&s, move |s, me| {
        s.sleep_until(me, Instant::now()).unwrap();
        s.cancel(me);
        assert_eq!(s.join(me, t), Ok(()));
        assert!(s.is_done(t));
    });
    s.join(0, u).unwrap();
    s.end(0);
    h.join().unwrap();
    hu.join().unwrap();
}

#[test]
fn waits_with_a_time_limit_time_out() {
    let s = Scheduler::new();
    let (t, h) = start(&s, |s, me| {
        s.sleep_until(me, Instant::now() + Duration::from_millis(200))
            .unwrap();
    });
    let soon = Instant::now() + Duration::from_millis(20);
    assert_eq!(s.wait_for_until(0, t, soon), Err(Stop::TimedOut));
    assert_eq!(
        s.park_until(0, Instant::now() + Duration::from_millis(5)),
        Err(Stop::TimedOut)
    );
    // The first wait's timer is over; it must not end this one early.
    assert_eq!(s.wait_for(0, t), Ok(()));
    s.end(0);
    h.join().unwrap();
}

#[test]
fn ended_tasks_give_their_slots_to_new_ones() {
    let s = Scheduler::new();
    let mut first = Vec::new();
    for _ in 0..3 {
        let (id, h) = start(&s, |_, _| {});
        s.join(0, id).unwrap();
        h.join().unwrap();
        first.push(id);
    }
    // Each task ended before the next began, so they all took slot 1.
    assert!(first.iter().all(|id| id & SLOT_MASK == 1));
    assert_eq!(s.lock(0).tasks.len(), 2);
    // An old id still reads as ended, and waiting for it ends at once.
    let (live, h) = start(&s, |s, me| s.park(me).unwrap_or(()));
    assert!(s.is_done(first[0]));
    s.join(0, first[1]).unwrap();
    s.unpark(first[2]);
    s.cancel(first[0]);
    assert!(!s.is_done(live));
    s.unpark(live);
    s.join(0, live).unwrap();
    h.join().unwrap();
    s.end(0);
}

/// `cancel()` stops a task that only computes (decision 0075): it checks
/// `must_stop` in its loop.
#[test]
fn cancel_now_stops_a_task_that_never_waits() {
    let s = Scheduler::new();
    let rounds: Arc<std::sync::atomic::AtomicUsize> = Arc::default();
    let r = Arc::clone(&rounds);
    let (busy, handle) = start(&s, move |s, me| {
        while !s.must_stop(me) {
            r.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    });
    assert!(!s.must_stop(busy), "not cancelled yet");
    s.cancel_now(busy);
    s.join(0, busy).unwrap();
    assert!(s.is_done(busy));
    s.end(0);
    handle.join().unwrap();
}
