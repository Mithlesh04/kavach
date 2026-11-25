// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Google access tokens from a service account's key (`Auth.GoogleToken`,
//! spec 8.4, *Google access tokens*), as Google's "OAuth 2.0 for Server to
//! Server Applications" says: a JWT signed RS256 with the account's
//! private key, exchanged at the key's `token_uri` for an access token,
//! which is kept until shortly before it ends. Nothing here prints or
//! keeps in an error the key, the JWT or the token.

use super::http;
use super::wire::Kind;
use kavach_runtime_core::base64;
use kavach_runtime_core::json::{self, Limits, Tree};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The variable naming the key file, when the `Model` names no other.
pub(crate) const VARIABLE: &str = "GOOGLE_APPLICATION_CREDENTIALS";

/// How long before its end a token is no longer used.
const MARGIN: Duration = Duration::from_secs(300);

/// How long a JWT is good for: the most Google takes.
const LIFETIME: i64 = 3600;

/// A service account's key, read from its file. It has no `Debug`, so it
/// can't be printed by mistake.
pub(crate) struct Account {
    email: String,
    key_id: Option<String>,
    /// The private key, PKCS #8.
    key: Vec<u8>,
    token_uri: String,
}

impl Account {
    /// The key file `path` (named by `variable`), read; else why not, in
    /// words that never quote the file.
    pub(crate) fn load(path: &str, variable: &str) -> Result<Account, String> {
        let why = |what: &str| {
            format!("the service account key file that {variable} names {what} (spec 8.4)")
        };
        let text = std::fs::read_to_string(path).map_err(|e| {
            why(&match e.kind() {
                std::io::ErrorKind::NotFound => "is not there".to_string(),
                _ => format!("can't be read: {}", e.kind()),
            })
        })?;
        Account::read(&text).map_err(why)
    }

    /// A key file's text, read.
    pub(crate) fn read(text: &str) -> Result<Account, &'static str> {
        let tree = json::parse(text, &Limits::default()).map_err(|_| "is not JSON")?;
        let field = |name: &str| match super::wire::field(&tree, name) {
            Some(Tree::Text(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
            _ => None,
        };
        if field("type").as_deref() != Some("service_account") {
            return Err("is not a service account's key (\"type\": \"service_account\")");
        }
        let email = field("client_email").ok_or("has no client_email")?;
        let pem = field("private_key").ok_or("has no private_key")?;
        let key = pkcs8(&pem).ok_or("has a private_key that is not an RSA key in PEM")?;
        let token_uri =
            field("token_uri").unwrap_or_else(|| "https://oauth2.googleapis.com/token".into());
        if !allowed(&token_uri) {
            return Err("has a token_uri that is not https:// (http:// only on this machine)");
        }
        Ok(Account {
            email,
            key_id: field("private_key_id"),
            key,
            token_uri,
        })
    }
}

/// The DER bytes of a PEM `PRIVATE KEY`, if `ring` takes it as RSA.
fn pkcs8(pem: &str) -> Option<Vec<u8>> {
    let body = pem
        .split("-----BEGIN PRIVATE KEY-----")
        .nth(1)?
        .split("-----END PRIVATE KEY-----")
        .next()?;
    let body: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    let der = base64::decode(&body, false).ok()?;
    ring::signature::RsaKeyPair::from_pkcs8(&der).ok()?;
    Some(der)
}

/// Whether a token is fetched from `uri`: over TLS, or over plain HTTP
/// to this machine only (a test server).
fn allowed(uri: &str) -> bool {
    if uri.starts_with("https://") {
        return true;
    }
    let Some(rest) = uri.strip_prefix("http://") else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or("");
    let name = match host.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => host.split(':').next().unwrap_or(""),
    };
    matches!(name, "127.0.0.1" | "localhost" | "::1")
}

/// The JWT that asks for a token for `scope`, issued at `now` (seconds
/// since 1970): RS256, header `alg`, `typ` and `kid`; claims `iss`,
/// `scope`, `aud` (the token server), `iat` and `exp` an hour on.
pub(crate) fn assertion(account: &Account, scope: &str, now: i64) -> Result<String, String> {
    let text = |s: &str| Tree::Text(s.to_string());
    let mut header = vec![("alg".into(), text("RS256")), ("typ".into(), text("JWT"))];
    if let Some(id) = &account.key_id {
        header.push(("kid".into(), text(id)));
    }
    let claims = vec![
        ("iss".into(), text(&account.email)),
        ("scope".into(), text(scope)),
        ("aud".into(), text(&account.token_uri)),
        ("iat".into(), Tree::Int(now)),
        ("exp".into(), Tree::Int(now + LIFETIME)),
    ];
    let part = |fields| {
        base64::encode(
            json::write(&Tree::Object(fields), 0).as_bytes(),
            true,
            false,
        )
    };
    let signed = format!("{}.{}", part(header), part(claims));
    let pair = ring::signature::RsaKeyPair::from_pkcs8(&account.key)
        .map_err(|_| "the service account's private key can't sign".to_string())?;
    let mut signature = vec![0; pair.public().modulus_len()];
    pair.sign(
        &ring::signature::RSA_PKCS1_SHA256,
        &ring::rand::SystemRandom::new(),
        signed.as_bytes(),
        &mut signature,
    )
    .map_err(|_| "the service account's private key can't sign".to_string())?;
    Ok(format!(
        "{signed}.{}",
        base64::encode(&signature, true, false)
    ))
}

/// The tokens fetched: by account, key, scope and token server, each
/// with the moment it is no longer used.
type Kept = HashMap<(String, String, String, String), (String, Instant)>;

fn kept() -> &'static Mutex<Kept> {
    static KEPT: OnceLock<Mutex<Kept>> = OnceLock::new();
    KEPT.get_or_init(Mutex::default)
}

/// An access token for `scope`: the one kept, while it has more than
/// `MARGIN` left, else a new one from the token server. Else what went
/// wrong, as the provider's trouble would be.
pub(crate) fn token(
    account: &Account,
    scope: &str,
    timeout: Duration,
) -> Result<String, (Kind, String)> {
    // One fetch at a time, so `ask`s started together share one token.
    static FETCHING: Mutex<()> = Mutex::new(());
    let _one = FETCHING.lock().unwrap_or_else(|e| e.into_inner());
    let digest = ring::digest::digest(&ring::digest::SHA256, &account.key);
    let id = (
        account.email.clone(),
        base64::encode(digest.as_ref(), true, false),
        scope.to_string(),
        account.token_uri.clone(),
    );
    let now = Instant::now();
    {
        let all = kept().lock().unwrap_or_else(|e| e.into_inner());
        if let Some((token, until)) = all.get(&id)
            && *until > now
        {
            return Ok(token.clone());
        }
    }
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let jwt = assertion(account, scope, seconds).map_err(|m| (Kind::BadKey, m))?;
    let body =
        format!("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion={jwt}");
    let headers = [("content-type", "application/x-www-form-urlencoded")];
    let (status, _, reply) =
        match http::post(&account.token_uri, &headers, body.as_bytes(), timeout) {
            Ok(r) => r,
            Err(http::Trouble::TimedOut(m)) => return Err((Kind::TimedOut, fetching(&m))),
            Err(http::Trouble::Network(m)) => return Err((Kind::Network, fetching(&m))),
        };
    let (token, lasts) = read_reply(status, &String::from_utf8_lossy(&reply))?;
    // A token that lasts no longer than the margin is used once.
    if let Some(lasts) = lasts.and_then(|s| s.checked_sub(MARGIN)) {
        let mut all = kept().lock().unwrap_or_else(|e| e.into_inner());
        all.insert(id, (token.clone(), now + lasts));
    }
    Ok(token)
}

fn fetching(why: &str) -> String {
    format!("getting a Google access token failed: {why}")
}

/// The token in the token server's reply, and how long it lasts; else
/// the trouble its error means. Only the error's code and description
/// are kept, never the reply itself.
pub(crate) fn read_reply(
    status: i64,
    body: &str,
) -> Result<(String, Option<Duration>), (Kind, String)> {
    let tree = json::parse(body, &Limits::default()).unwrap_or(Tree::Null);
    let field = |name| match super::wire::field(&tree, name) {
        Some(Tree::Text(s)) => Some(s.as_str()),
        _ => None,
    };
    if (200..300).contains(&status) {
        let token = field("access_token")
            .filter(|t| !t.is_empty() && t.bytes().all(|b| b.is_ascii_graphic()));
        let Some(token) = token else {
            let m = "the token server's reply has no access_token";
            return Err((Kind::Other(status), fetching(m)));
        };
        let lasts = match super::wire::field(&tree, "expires_in") {
            Some(Tree::Int(n)) if *n > 0 => Some(Duration::from_secs(*n as u64)),
            _ => None,
        };
        return Ok((token.to_string(), lasts));
    }
    let code = field("error").unwrap_or("");
    let kind = match status {
        400 if code == "invalid_scope" => Kind::BadRequest,
        400 | 401 | 403 => Kind::BadKey,
        429 => Kind::RateLimited,
        500 | 502 | 503 => Kind::Overloaded,
        504 => Kind::TimedOut,
        s => Kind::Other(s),
    };
    let mut why = format!("the token server said {status}");
    for part in [code, field("error_description").unwrap_or("")] {
        if !part.is_empty() {
            why.push_str(": ");
            why.push_str(part);
        }
    }
    Err((kind, fetching(&why)))
}
