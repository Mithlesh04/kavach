// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Signing a request with AWS Signature Version 4 (`Auth.AwsSignature`,
//! spec 8.4, *Signing AWS requests*), as AWS's SigV4 reference says: a
//! canonical request, a string to sign over its hash, a signing key made
//! from the secret, the date, the region and the service, and the
//! `authorization` header. Nothing here prints or keeps the secret.

use ring::{digest, hmac};

/// The variables the credentials come from.
pub(crate) const KEY_ID: &str = "AWS_ACCESS_KEY_ID";
pub(crate) const SECRET: &str = "AWS_SECRET_ACCESS_KEY";
pub(crate) const TOKEN: &str = "AWS_SESSION_TOKEN";

/// An AWS access key, and the token of short-term credentials. It has no
/// `Debug`, so it can't be printed by mistake.
pub(crate) struct Keys {
    pub id: String,
    pub secret: String,
    pub token: Option<String>,
}

impl Keys {
    /// The credentials, read with `get` as a key is (spec 8.4): a
    /// variable set to empty text counts as not set. Else the name of the
    /// first variable missing.
    pub(crate) fn read(get: impl Fn(&str) -> Option<String>) -> Result<Keys, &'static str> {
        let read = |name| {
            get(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        Ok(Keys {
            id: read(KEY_ID).ok_or(KEY_ID)?,
            secret: read(SECRET).ok_or(SECRET)?,
            token: read(TOKEN),
        })
    }
}

/// Where a signature is good: the service and the region.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Scope {
    pub service: String,
    pub region: String,
}

impl Scope {
    /// Whether both are made of letters, digits and `-` (spec 8.4).
    pub(crate) fn valid(&self) -> bool {
        let ok =
            |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
        ok(&self.service) && ok(&self.region)
    }
}

/// A request signed: each step, which only the tests read (they check
/// each against AWS's suite), and the headers it gains.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct Signed {
    pub canonical: String,
    pub to_sign: String,
    pub signature: String,
    /// `x-amz-date`, `x-amz-security-token` with a token, and
    /// `authorization`.
    pub headers: Vec<(String, String)>,
}

/// Signs a request: `method`, its path and query as sent (`target`), the
/// headers to sign (`host` among them), and `body`, at `amz_date`
/// (`20150830T123600Z`).
pub(crate) fn sign(
    method: &str,
    target: &str,
    headers: &[(String, String)],
    body: &[u8],
    keys: &Keys,
    scope: &Scope,
    amz_date: &str,
) -> Signed {
    let mut added = vec![("x-amz-date".to_string(), amz_date.to_string())];
    if let Some(token) = &keys.token {
        added.push(("x-amz-security-token".into(), token.clone()));
    }
    // Header names in lowercase, sorted; values trimmed, with runs of
    // spaces made one; a name given twice has its values joined by `,`.
    let mut all: Vec<(String, String)> = (headers.iter().chain(&added))
        .map(|(k, v)| {
            (
                k.to_ascii_lowercase(),
                v.split_whitespace().collect::<Vec<_>>().join(" "),
            )
        })
        .collect();
    all.sort_by(|a, b| a.0.cmp(&b.0));
    let mut joined: Vec<(String, String)> = Vec::new();
    for (k, v) in all {
        match joined.last_mut() {
            Some((last, values)) if *last == k => {
                values.push(',');
                values.push_str(&v);
            }
            _ => joined.push((k, v)),
        }
    }
    let names: Vec<&str> = joined.iter().map(|(k, _)| k.as_str()).collect();
    let signed_headers = names.join(";");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let path = if path.is_empty() { "/" } else { path };
    let mut canonical = format!(
        "{method}\n{}\n{}\n",
        encode(path.as_bytes(), true),
        canonical_query(query)
    );
    for (k, v) in &joined {
        canonical.push_str(&format!("{k}:{v}\n"));
    }
    canonical.push_str(&format!(
        "\n{signed_headers}\n{}",
        hex(digest::digest(&digest::SHA256, body).as_ref())
    ));

    let date = &amz_date[..amz_date.len().min(8)];
    let credential_scope = format!("{date}/{}/{}/aws4_request", scope.region, scope.service);
    let to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        hex(digest::digest(&digest::SHA256, canonical.as_bytes()).as_ref())
    );
    let mut key = format!("AWS4{}", keys.secret).into_bytes();
    for part in [date, &scope.region, &scope.service, "aws4_request"] {
        key = mac(&key, part.as_bytes());
    }
    let signature = hex(&mac(&key, to_sign.as_bytes()));
    added.push((
        "authorization".into(),
        format!(
            "AWS4-HMAC-SHA256 Credential={}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}",
            keys.id
        ),
    ));
    Signed {
        canonical,
        to_sign,
        signature,
        headers: added,
    }
}

/// The moment `ms` (milliseconds since 1970, UTC) as `x-amz-date` writes
/// it: `20150830T123600Z`.
pub(crate) fn amz_date(ms: i64) -> String {
    let iso = kavach_runtime_core::agents::utc_of(ms);
    iso.chars().filter(|c| *c != '-' && *c != ':').collect()
}

fn mac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, key);
    hmac::sign(&key, data).as_ref().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `bytes` with every byte but letters, digits, `-`, `.`, `_`, `~` (and
/// `/` when `slash`) written as `%XX`.
fn encode(bytes: &[u8], slash: bool) -> String {
    let mut out = String::new();
    for &b in bytes {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) || (slash && b == b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `text` with each `%XX` made its byte again, and `+` left as it is.
fn decode(text: &str) -> Vec<u8> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let pair = b
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok());
        match (b[i], pair.and_then(|h| u8::from_str_radix(h, 16).ok())) {
            (b'%', Some(v)) => {
                out.push(v);
                i += 3;
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// The query's names and values, each encoded as AWS says, sorted by
/// name and then value, joined by `&`.
fn canonical_query(query: &str) -> String {
    let mut pairs: Vec<(String, String)> = (query.split('&'))
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (encode(&decode(k), false), encode(&decode(v), false))
        })
        .collect();
    pairs.sort();
    let pairs: Vec<String> = pairs.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
    pairs.join("&")
}
