//! What several test targets share: a watch that names a program still
//! running after a minute, so a run that hangs says where.

use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The programs running now, one place per worker: each its name and
/// when it started.
pub struct Running {
    now: Mutex<Vec<Option<(String, Instant)>>>,
    done: AtomicBool,
}

impl Running {
    pub fn new(workers: usize) -> Self {
        Running {
            now: Mutex::new(vec![None; workers]),
            done: AtomicBool::new(false),
        }
    }

    /// Worker `w` starts `name`, or (`None`) finished its program.
    pub fn set(&self, w: usize, name: Option<String>) {
        let mut now = self.now.lock().unwrap_or_else(|e| e.into_inner());
        now[w] = name.map(|n| (n, Instant::now()));
    }

    /// Ends `watch` when dropped, even when the test panics, so a scope
    /// waiting for the watch still ends.
    pub fn finishing(&self) -> Finishing<'_> {
        Finishing(self)
    }

    /// Until `finish`: each minute a program is still running, its name on
    /// the terminal, past the test's captured output.
    pub fn watch(&self) {
        let mut said: Vec<(String, u64)> = Vec::new();
        while !self.done.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(500));
            let now = self.now.lock().unwrap_or_else(|e| e.into_inner());
            for (name, since) in now.iter().flatten() {
                let minutes = since.elapsed().as_secs() / 60;
                if minutes > 0 && !said.contains(&(name.clone(), minutes)) {
                    said.push((name.clone(), minutes));
                    let _ = writeln!(
                        std::io::stderr(),
                        "still running after {minutes} min: {name}"
                    );
                }
            }
        }
    }
}

/// Ends a `Running`'s watch when dropped.
pub struct Finishing<'a>(&'a Running);

impl Drop for Finishing<'_> {
    fn drop(&mut self) {
        self.0.done.store(true, Ordering::Relaxed);
    }
}
