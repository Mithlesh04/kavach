//! An agent's audit log (spec 8.6): one entry per call, as text or as a
//! JSON line, and to an OpenTelemetry collector too when the agent's
//! `Audit` says `otlp: true`.

use super::{Check, Guard, SINK, unix_ms};
use crate::json::{self, Tree};
use std::io::Write;

impl Guard {
    /// Appends one call to the audit log (spec 8.6): its verdict and what
    /// came of it (`ok …`, `error …`, `denied: …`).
    pub fn audit(&self, c: &Check, result: &str) {
        let Some(path) = &self.settings.audit else {
            return;
        };
        let verdict = if c.notes.is_empty() {
            "allowed".to_string()
        } else {
            c.notes.join("; ")
        };
        if let (Some((endpoint, service)), Some(sink)) = (&self.settings.otlp, SINK.get()) {
            let mut attributes = vec![
                ("agent".to_string(), self.settings.name.clone()),
                ("request".to_string(), self.request.to_string()),
                ("call".to_string(), c.call.clone()),
                ("verdict".to_string(), verdict.clone()),
            ];
            if let Some(m) = &c.model {
                attributes.push(("model".into(), m.model.clone()));
                if let Some((i, o)) = m.tokens {
                    attributes.push(("input_tokens".into(), i.to_string()));
                    attributes.push(("output_tokens".into(), o.to_string()));
                }
            }
            attributes.push(("result".into(), result.to_string()));
            sink(
                endpoint,
                service,
                c.refused.is_some(),
                &c.shown,
                &attributes,
            );
        }
        let entry = if self.settings.audit_json {
            let text = |s: &str| Tree::Text(s.to_string());
            let mut fields = vec![
                ("time".into(), text(&utc_now())),
                ("agent".into(), text(&self.settings.name)),
                ("request".into(), Tree::Int(self.request as i64)),
                ("call".into(), text(&c.call)),
                ("shown".into(), text(&c.shown)),
                ("verdict".into(), text(&verdict)),
            ];
            if let Some(m) = &c.model {
                fields.push(("model".into(), text(&m.model)));
                if let Some((i, o)) = m.tokens {
                    fields.push(("input_tokens".into(), Tree::Int(i as i64)));
                    fields.push(("output_tokens".into(), Tree::Int(o as i64)));
                }
            }
            fields.push(("result".into(), text(result)));
            format!("{}\n", json::write(&Tree::Object(fields), 0))
        } else {
            let model = c
                .model
                .as_ref()
                .map(|m| format!("  model: {}\n", m.line()))
                .unwrap_or_default();
            format!(
                "{} {} request#{} {}\n  verdict: {verdict}\n{model}  result: {result}\n",
                utc_now(),
                self.settings.name,
                self.request,
                c.shown
            )
        };
        if let Err(why) = append(path, &entry) {
            self.lost(path, &why, &entry);
        }
    }

    /// Writes `text` as a line of its own in the audit log: a set-aside
    /// event, or the log working again (decision 0073).
    pub fn note(&self, text: &str) -> Result<(), String> {
        let Some(path) = &self.settings.audit else {
            return Ok(());
        };
        let entry = if self.settings.audit_json {
            let fields = vec![
                ("time".into(), Tree::Text(utc_now())),
                ("agent".into(), Tree::Text(self.settings.name.clone())),
                ("note".into(), Tree::Text(text.to_string())),
            ];
            format!("{}\n", json::write(&Tree::Object(fields), 0))
        } else {
            format!("{} {} {text}\n", utc_now(), self.settings.name)
        };
        append(path, &entry)
    }

    /// An entry that couldn't be written: it goes to standard error, and
    /// the agent's next calls are refused until the log works again.
    fn lost(&self, path: &std::path::Path, why: &str, entry: &str) {
        let mut broken = self.broken.lock().unwrap_or_else(|e| e.into_inner());
        if broken.is_none() {
            eprintln!(
                "kavach: the audit log {} can't be written ({why}): {}'s calls are refused until it can (spec 8.6)",
                crate::files::shown_path(path),
                self.settings.name
            );
        }
        eprint!("{entry}");
        let n = broken.as_ref().map_or(0, |(_, n)| *n);
        *broken = Some((why.to_string(), n + 1));
    }

    /// Why a call must be refused now: the audit log can't be written,
    /// and a line saying it works again can't be either.
    pub(super) fn audit_refuses(&self) -> Option<String> {
        let mut broken = self.broken.lock().unwrap_or_else(|e| e.into_inner());
        let (_, n) = broken.clone()?;
        let entries = if n == 1 { "entry" } else { "entries" };
        match self.note(&format!(
            "audit log writable again ({n} {entries} printed to standard error)"
        )) {
            Ok(()) => {
                *broken = None;
                None
            }
            Err(now) => Some(format!("the audit log can't be written: {now}")),
        }
    }

    /// The events the journal set aside when it was opened, in the audit
    /// log (decision 0073).
    pub fn set_aside(&self, events: &[(i64, String)]) {
        for (n, handler) in events {
            let text = format!(
                "set aside: event #{n} of {handler} (started {} times without finishing)",
                crate::journal::STARTS
            );
            if let Err(why) = self.note(&text)
                && let Some(path) = &self.settings.audit
            {
                self.lost(path, &why, &format!("{text}\n"));
            }
        }
    }
}

/// Appends `entry` to the audit log at `path`.
fn append(path: &std::path::Path, entry: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| f.write_all(entry.as_bytes()))
        .map_err(|e| e.to_string())
}

/// `2026-11-02T10:14:07Z`.
pub fn utc_now() -> String {
    utc_of(unix_ms())
}

/// The moment `ms` (milliseconds since 1970, UTC) as `utc_now` writes it.
pub fn utc_of(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Days since 1970-01-01 to a calendar date (Howard Hinnant's method).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let (h, min, s) = (rest / 3600, rest % 3600 / 60, rest % 60);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}Z")
}
