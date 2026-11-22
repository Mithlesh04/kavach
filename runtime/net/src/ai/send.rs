// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Sending one try of an `ask` (spec 8.4): the key read and put where the
//! provider's `auth` says, the address `KAVACH_AI_URL` may change, and the
//! gate of the provider and key. Provider code never sees the key.

use super::wire::{Auth, Call, Failure, Kind};
use super::{Request, gate, http};
use std::sync::mpsc;

/// A request ready to send, as often as it is sent again.
pub(crate) struct Sending {
    url: String,
    headers: Vec<(String, String)>,
    body: String,
    timeout: std::time::Duration,
    gate: String,
}

/// What one try gave: a reply (its status, headers in lowercase, and
/// body), or no reply at all.
pub(crate) enum Sent {
    Reply(i64, Vec<(String, String)>, String),
    Failed(Kind, String),
}

/// A try in progress on a helper thread.
pub(crate) struct Pending {
    done: mpsc::Receiver<Sent>,
}

impl Pending {
    /// One look at whether the try has finished.
    pub(crate) fn poll(&self) -> Option<Sent> {
        match self.done.try_recv() {
            Ok(s) => Some(s),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Sent::Failed(Kind::Network, "the call stopped".into()))
            }
        }
    }
}

impl Sending {
    /// `call` with its key, or why it can't be sent: its key's variable
    /// (the `Model`'s `api_key_env`, else the provider's) is not set.
    pub(crate) fn new(call: Call, req: &Request) -> Result<Sending, Failure> {
        let variable = req.api_key_env.clone().unwrap_or(call.key_variable);
        // As the program sees variables: `--env`, the environment, then
        // `[env.vars]` and `.env` (spec 10.13).
        let get = kavach_runtime_core::env_file::get;
        let key = match call.auth {
            Auth::NoKey => None,
            _ => Some(get(&variable).ok_or_else(|| Failure::NoKey(variable.clone()))?),
        };
        let mut headers = Vec::new();
        match (&call.auth, key) {
            (Auth::Bearer, Some(k)) => {
                headers.push(("authorization".into(), format!("Bearer {}", k.trim())));
            }
            (Auth::InHeader(name), Some(k)) => headers.push((name.clone(), k.trim().to_string())),
            _ => {}
        }
        headers.extend(call.headers);
        let url = match (&req.api_url, get("KAVACH_AI_URL")) {
            (None, Some(to)) => moved(&call.url, &to),
            _ => call.url,
        };
        Ok(Sending {
            url,
            headers,
            body: call.body,
            timeout: req.timeout,
            gate: format!("{} {variable}", req.provider),
        })
    }

    /// A counting call (`ai.count`, spec 8.4): it passes a gate of its
    /// own, as providers limit counting calls apart from asks.
    pub(crate) fn counting(mut self) -> Sending {
        self.gate.push_str(" (count)");
        self
    }

    /// Sends it once, through its gate, on a helper thread.
    pub(crate) fn start(&self) -> Pending {
        let (send, done) = mpsc::channel();
        let (url, body, timeout) = (self.url.clone(), self.body.clone(), self.timeout);
        let headers = self.headers.clone();
        let gate = gate::of(&self.gate);
        std::thread::spawn(move || {
            gate.enter();
            let shown: Vec<(&str, &str)> = (headers.iter())
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let sent = match http::post(&url, &shown, body.as_bytes(), timeout) {
                Ok((status, headers, body)) => {
                    Sent::Reply(status, headers, String::from_utf8_lossy(&body).into_owned())
                }
                Err(http::Trouble::TimedOut(m)) => Sent::Failed(Kind::TimedOut, m),
                Err(http::Trouble::Network(m)) => Sent::Failed(Kind::Network, m),
            };
            // Turned away as rate limited (429): the provider took the
            // others under way, not this one.
            let busy = matches!(sent, Sent::Reply(429, ..));
            let answered = matches!(sent, Sent::Reply(200..=299, ..));
            gate.leave(busy, answered);
            let _ = send.send(sent);
        });
        Pending { done }
    }
}

/// `url` sent to `to` (`KAVACH_AI_URL`) instead: its path after `to`'s
/// own, at `to`'s scheme, host and port.
pub(crate) fn moved(url: &str, to: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let path = rest.find('/').map_or("", |i| &rest[i..]);
    format!("{}{path}", to.trim_end_matches('/'))
}
