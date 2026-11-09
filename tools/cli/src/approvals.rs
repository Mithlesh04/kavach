//! `kavach approvals`, `kavach approve` and `kavach deny` (spec 8.3,
//! 10.5, decision 0072): the requests a project's agents wait on, and a
//! person's answer to one. The answer goes into the agent's folder, where
//! the running program sees it within a second, or the next start finds
//! it.

use kavach_runtime_core::agents::{Answer, AnswerError, answer_request, waiting};
use kavach_runtime_core::json::Tree;
use std::path::PathBuf;
use std::process::ExitCode;

/// The folder holding the project's agents' folders: `[agents] folder` in
/// the nearest `kavach.toml`, or `agents` beside it.
fn agents_folder() -> Result<PathBuf, String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let config = kavach_driver::Config::for_file(&here.join("kavach.toml"))?;
    let root = config
        .root
        .ok_or("no kavach.toml here or in a folder above: approvals belong to a project")?;
    Ok(root.join(config.agents.folder.as_deref().unwrap_or("agents")))
}

/// `kavach approvals`: one line for each request waiting.
pub(crate) fn list() -> ExitCode {
    let agents = match agents_folder() {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };
    let mut requests = waiting(&agents);
    let text = |fields: &[(String, Tree)], name: &str| {
        fields.iter().find_map(|(k, v)| match (k == name, v) {
            (true, Tree::Text(s)) => Some(s.clone()),
            _ => None,
        })
    };
    requests.sort_by_key(|(_, f)| text(f, "asked"));
    if requests.is_empty() {
        println!("no requests wait for approval");
    }
    for (_, f) in &requests {
        let get = |name: &str| text(f, name).unwrap_or_default();
        println!(
            "{}  {}  by {}  asked {}  until {}",
            get("id"),
            get("call"),
            get("by"),
            get("asked"),
            get("until")
        );
    }
    ExitCode::SUCCESS
}

/// `kavach approve <id> --as <name>` and `kavach deny <id> --as <name>
/// [--reason <text>]`.
pub(crate) fn answer(approved: bool, id: &str, rest: &[&str]) -> ExitCode {
    let mut by = None;
    let mut reason = None;
    let mut words = rest.iter();
    while let Some(w) = words.next() {
        match *w {
            "--as" => by = words.next().map(|s| s.to_string()),
            "--reason" => reason = words.next().map(|s| s.to_string()),
            other => return fail(&format!("unknown option `{other}`")),
        }
    }
    let Some(by) = by else {
        return fail("say who answers: `--as <name>`, written to the audit log");
    };
    let agents = match agents_folder() {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };
    let answer = Answer {
        approved,
        by,
        reason,
    };
    match answer_request(&agents, id, &answer) {
        Ok(()) => {
            let word = if approved { "approved" } else { "denied" };
            println!("{id}: {word}");
            ExitCode::SUCCESS
        }
        Err(AnswerError::NotWaiting) => fail(&format!(
            "no request {id} waits for an answer (see `kavach approvals`)"
        )),
        Err(AnswerError::Failed(e)) => fail(&e),
    }
}

fn fail(why: &str) -> ExitCode {
    eprintln!("error: {why}");
    ExitCode::from(1)
}
