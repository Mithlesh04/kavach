use super::kv_par_for;
use std::collections::HashSet;
use std::sync::Mutex;
use std::thread::ThreadId;

static SEEN: Mutex<Option<HashSet<ThreadId>>> = Mutex::new(None);

/// A chunk that notes which thread ran it, slowly enough for the
/// workers to join.
extern "C-unwind" fn note(_lo: i64, _hi: i64) -> i64 {
    let id = std::thread::current().id();
    SEEN.lock().unwrap().get_or_insert_default().insert(id);
    std::thread::sleep(std::time::Duration::from_micros(200));
    0
}

#[test]
fn on_cores_n_runs_a_loop_on_at_most_n_threads() {
    if kavach_runtime_core::cores() < 3 {
        return; // the limit can't show on fewer cores
    }
    // `cores(1)` is the plain loop; `cores(2)` uses two threads at most.
    assert_eq!(
        kv_par_for(0, 1 << 20, note as *const u8, std::ptr::null(), 0, 1),
        1
    );
    assert_eq!(
        kv_par_for(0, 1 << 20, note as *const u8, std::ptr::null(), 0, 2),
        0
    );
    let seen = SEEN.lock().unwrap().take().unwrap_or_default();
    assert!((1..=2).contains(&seen.len()), "{} threads", seen.len());
}
