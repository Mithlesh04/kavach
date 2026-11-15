//! The capability a call into the standard library needs (spec 8.1): the
//! module's own (`stdlib.rs`'s `MODULES`), the ones read freely, the
//! narrower ones (`Fs.read`, `Input`), and the calls a contract may not
//! make.

use crate::checker::Checker;
use crate::stdlib::MODULES;
use kavach_ast::Span;

impl Checker<'_> {
    /// Notes the capability `module.name` needs where it is called.
    pub(crate) fn std_capability(&mut self, module: &str, name: &str, span: Span) {
        // Reading the clock needs no capability (spec 8.1); a contract still
        // may not read it, as its answer would change from run to run.
        let reads_clock =
            module == "time" && matches!(name, "now_ms" | "monotonic_ms" | "monotonic_ns");
        // Nor does reading a zone's rules, or freezing a test's clock (spec
        // 14.4), which only a `test` block may do.
        let reads_zone = module == "time" && matches!(name, "zone_offset" | "freeze");
        let in_test = self.ctx.as_ref().is_some_and(|c| c.name == "test");
        if module == "time" && name == "freeze" && !in_test {
            self.error(
                span,
                "`time.freeze` is only for tests: call it directly in a `test` block (spec 14.4)",
            );
        }
        if reads_clock && self.in_contract {
            self.error(span, format!("a contract can't call `time.{name}`: it reads the clock, and contracts must give the same answer every time (spec 7.5)"));
        }
        // `std.agents` needs nothing, except to answer approvals (14.16).
        let answers = module == "agents" && matches!(name, "request_fields" | "answer_request");
        // A hold on the program's stop needs nothing (decision 0088), nor
        // does working out a WebSocket handshake (0092).
        let holds = (module == "cli" && name.starts_with("stop_"))
            || (module == "net" && name.starts_with("websocket_"))
            || (module == "process" && matches!(name, "peak_bytes_id" | "cpu_ms_id"))
            || (module == "system" && name.starts_with("basic_"))
            // Letting a watcher go (spec 14.3).
            || (module == "fs" && name == "watch_close_id")
            // Sounds are data; playing them needs `Audio` (spec 14.27).
            || (module == "audio" && matches!(name, "sound_id" | "decode_id" | "info_id" | "drop_sound_id"));
        if let Some(cap) = (MODULES.iter())
            .find(|(m, _)| *m == module && !reads_clock && !reads_zone && !holds)
            .and_then(|(_, c)| if answers { Some("Approvals") } else { *c })
        {
            // Reading and changing files are granted apart (spec 8.1).
            let cap = match (module, name) {
                // Listening to the keyboard and mouse (spec 14.22).
                ("window", "listen_id" | "lock_cursor_id") => "Input",
                (
                    "fs",
                    "read_text" | "read_bytes" | "lines" | "chunks" | "exists" | "list" | "stat"
                    | "watch_open_id" | "watch_next_id" | "watch_poll_id",
                ) => "Fs.read",
                ("fs", _) => "Fs.write",
                // Folders and crash reports make folders; a dialog is a window (14.30).
                ("app", "dialog_id") => "Window",
                ("app", _) => "Fs.write",
                // Parts `System` doesn't cover (spec 14.25, decision 0112).
                ("system", "critical_id") => "System.critical.read",
                ("system", "act_id") => "System.critical.write",
                _ => cap,
            };
            self.note_cap(cap, span);
            if self.in_contract {
                self.error(span, format!("a contract can't call `{module}.{name}`: it uses {cap}, and contracts must be pure (spec 7.5)"));
            }
        }
    }
}
