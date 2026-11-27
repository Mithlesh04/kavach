// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! How long each stage of the compiler takes, for work on the compiler:
//! with `KAVACH_TIMES=1` (spec 10.11), each stage prints a line to
//! standard error as it ends, `time: check 41.2 ms`. The stages: parse,
//! resolve, check and rewrites here in the driver; lowering and passes
//! in KIR; codegen and link in the backends.

use std::sync::OnceLock;
use std::time::Instant;

/// Whether `KAVACH_TIMES` is set (to anything but `0`), read once.
pub fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("KAVACH_TIMES").is_some_and(|v| v != "0"))
}

/// Times stages that run one after another. Nothing is timed while
/// timing is off.
pub struct Clock(Option<Instant>);

impl Clock {
    /// Starts the first stage.
    pub fn start() -> Clock {
        Clock(on().then(Instant::now))
    }

    /// Ends the stage `name`, printing its time, and starts the next.
    pub fn lap(&mut self, name: &str) {
        if let Some(start) = self.0 {
            let now = Instant::now();
            eprintln!("{}", line(name, (now - start).as_secs_f64()));
            self.0 = Some(now);
        }
    }
}

/// The line a stage prints.
fn line(name: &str, seconds: f64) -> String {
    format!("time: {name} {:.1} ms", seconds * 1000.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_stage_prints_its_name_and_milliseconds() {
        assert_eq!(super::line("check", 0.04123), "time: check 41.2 ms");
        assert_eq!(super::line("link", 1.5), "time: link 1500.0 ms");
    }
}
