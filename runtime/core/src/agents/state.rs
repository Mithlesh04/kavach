//! An agent's state file (spec 8.5): each limit's calls inside its
//! window, so totals hold across restarts.

use super::{Guard, Num, steady_ms, unix_ms};
use crate::json::{self, Tree};

impl Guard {
    /// Reads the state file, if there is one, into the rules' calls.
    pub(super) fn load_state(&mut self) {
        let Some(path) = &self.settings.state else {
            return;
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(Tree::Object(limits)) = json::parse(&text, &json::Limits::default()) else {
            return;
        };
        let (now, now_ms) = (steady_ms(), unix_ms());
        for rule in &mut self.rules {
            let key = rule.key();
            let Some((_, Tree::Array(calls))) = limits.iter().find(|(k, _)| *k == key) else {
                continue;
            };
            for call in calls {
                let Tree::Array(pair) = call else {
                    continue;
                };
                let (Some(Tree::Int(at)), Some(value)) = (pair.first(), pair.get(1)) else {
                    continue;
                };
                // A call the clock says is still to come was made before
                // the clock was set back: it counts as made now, so a
                // limit never loosens.
                let ago = now_ms.saturating_sub(*at).max(0);
                rule.history.push((now - ago, Num::from_tree(value), 0));
            }
        }
    }

    /// Writes every rule's calls to the state file, as `fs.write_durable`
    /// does, so neither a crash nor a power cut leaves half.
    pub(super) fn save_state(&self) {
        let Some(path) = &self.settings.state else {
            return;
        };
        let (now, now_ms) = (steady_ms(), unix_ms());
        let limits = self
            .rules
            .iter()
            .map(|rule| {
                let calls = rule
                    .history
                    .iter()
                    .map(|(t, v, _)| {
                        let at = now_ms - (now - t);
                        let value = v.as_ref().map_or(Tree::Null, Num::to_tree);
                        Tree::Array(vec![Tree::Int(at), value])
                    })
                    .collect();
                (rule.key(), Tree::Array(calls))
            })
            .collect();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Forced to the disk before it replaces the old one: a power cut
        // must not leave an empty file, which would count no calls.
        if let Some(p) = path.to_str() {
            let _ =
                crate::files::write_durably(p, json::write(&Tree::Object(limits), 2).as_bytes());
        }
    }
}
