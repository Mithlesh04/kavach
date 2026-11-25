// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Google access tokens from a service account's key (spec 8.4, *Google
//! access tokens*), with a FAKE key (`fake_google_key`) and a fake token
//! server on this machine: no real account or Google server is used.

use super::super::google::{self, Account};
use super::super::wire::Kind;
use super::fake_google_key::{EMAIL, key_file, modulus};
use super::*;

const SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

fn part(text: &str) -> String {
    let bytes = kavach_runtime_core::base64::decode(text, true).unwrap();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn the_jwt_has_googles_header_and_claims_and_an_rs256_signature() {
    let account = Account::read(&key_file("https://oauth2.googleapis.com/token")).ok();
    let jwt = google::assertion(&account.unwrap(), SCOPE, 1_700_000_000).unwrap();
    let parts: Vec<&str> = jwt.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(
        part(parts[0]),
        r#"{"alg":"RS256","typ":"JWT","kid":"fake-key-id"}"#
    );
    assert_eq!(
        part(parts[1]),
        format!(
            r#"{{"iss":"{EMAIL}","scope":"{SCOPE}","aud":"https://oauth2.googleapis.com/token","iat":1700000000,"exp":1700003600}}"#
        )
    );
    // Base64url with no padding, as Google asks.
    assert!(!jwt.contains('=') && !jwt.contains('+') && !jwt.contains('/'));
    // Checked with the public key openssl gave, not the private key.
    let signed = format!("{}.{}", parts[0], parts[1]);
    let signature = kavach_runtime_core::base64::decode(parts[2], true).unwrap();
    let e = [1, 0, 1];
    let n = modulus();
    let verify = |data: &str| crate::crypto::rsa_verify(&n, &e, data.as_bytes(), &signature, false);
    assert!(verify(&signed));
    // Claims changed after signing don't match the signature.
    let later = part(parts[1]).replace("1700003600", "1700007200");
    let later = kavach_runtime_core::base64::encode(later.as_bytes(), true, false);
    assert!(!verify(&format!("{}.{later}", parts[0])));
}

#[test]
fn key_files_that_cant_be_used_say_why() {
    let good = key_file("https://oauth2.googleapis.com/token");
    let cases: [(String, &str); 6] = [
        ("not json".into(), "is not JSON"),
        (
            good.replace("service_account", "authorized_user"),
            "is not a service account's key",
        ),
        (good.replace("client_email", "email"), "has no client_email"),
        (
            good.replace("MIIEvgIBADANBgkqhkiG9w0BAQEF", "AAAA"),
            "has a private_key that is not an RSA key in PEM",
        ),
        (
            key_file("http://oauth2.googleapis.com/token"),
            "has a token_uri that is not https://",
        ),
        (
            key_file("ftp://127.0.0.1/token"),
            "has a token_uri that is not https://",
        ),
    ];
    for (text, why) in cases {
        match Account::read(&text) {
            Err(m) => assert!(m.starts_with(why), "{m}"),
            Ok(_) => panic!("{text} was taken"),
        }
    }
    // Plain HTTP to this machine is a test server's.
    for uri in [
        "http://127.0.0.1:9/token",
        "http://localhost/t",
        "http://[::1]:9/t",
    ] {
        assert!(Account::read(&key_file(uri)).is_ok(), "{uri}");
    }
}

#[test]
fn a_key_file_that_is_not_there_is_named_by_its_variable_only() {
    let path = std::env::temp_dir().join("kavach-no-such-google-key.json");
    let Err(m) = Account::load(path.to_str().unwrap(), "SHOP_GOOGLE_KEY") else {
        panic!("a missing file was read");
    };
    assert_eq!(
        m,
        "the service account key file that SHOP_GOOGLE_KEY names is not there (spec 8.4)"
    );
}

#[test]
fn the_token_servers_replies_are_read_as_troubles() {
    let ok = google::read_reply(
        200,
        r#"{"access_token":"ya29.fake","expires_in":3599,"token_type":"Bearer"}"#,
    );
    assert_eq!(
        ok.unwrap(),
        ("ya29.fake".to_string(), Some(Duration::from_secs(3599)))
    );
    let cases: [(i64, &str, Kind, &str); 7] = [
        (
            200,
            "{}",
            Kind::Other(200),
            "the token server's reply has no access_token",
        ),
        // A token that would break the header is no token.
        (
            200,
            r#"{"access_token":"a\r\nx: y"}"#,
            Kind::Other(200),
            "no access_token",
        ),
        (
            400,
            r#"{"error":"invalid_grant","error_description":"Invalid JWT Signature."}"#,
            Kind::BadKey,
            "the token server said 400: invalid_grant: Invalid JWT Signature.",
        ),
        (
            400,
            r#"{"error":"invalid_scope"}"#,
            Kind::BadRequest,
            "invalid_scope",
        ),
        (429, "", Kind::RateLimited, "the token server said 429"),
        (503, "", Kind::Overloaded, "503"),
        (418, "", Kind::Other(418), "418"),
    ];
    for (status, body, kind, says) in cases {
        let Err((k, m)) = google::read_reply(status, body) else {
            panic!("{body} gave a token");
        };
        assert_eq!(k, kind, "{body}");
        assert!(
            m.starts_with("getting a Google access token failed: "),
            "{m}"
        );
        assert!(m.contains(says), "{m}");
    }
}

/// A token server's reply that gives `token`, lasting `seconds`.
fn token_reply(token: &str, seconds: i64) -> &'static str {
    let body = format!(r#"{{"access_token":"{token}","expires_in":{seconds}}}"#);
    let reply = format!(
        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    );
    Box::leak(reply.into_boxed_str())
}

fn account_at(url: &str) -> Account {
    Account::read(&key_file(&format!("{url}/token")))
        .ok()
        .unwrap()
}

#[test]
fn a_token_is_fetched_once_and_kept_while_it_lasts() {
    let (url, server) = fake_server(vec![token_reply("ya29.first", 3599)]);
    let account = account_at(&url);
    let wait = Duration::from_secs(5);
    // A scope of its own: the tokens kept are shared by the tests.
    let scope = "https://www.googleapis.com/auth/kept";
    assert_eq!(
        google::token(&account, scope, wait).ok().unwrap(),
        "ya29.first"
    );
    assert_eq!(
        google::token(&account, scope, wait).ok().unwrap(),
        "ya29.first"
    );
    let sent = server.join().unwrap();
    assert_eq!(sent.len(), 1);
    let request = &sent[0];
    assert!(request.starts_with("POST /token HTTP/1.1\r\n"), "{request}");
    assert!(request.contains("content-type: application/x-www-form-urlencoded\r\n"));
    let body = &request[request.find("\r\n\r\n").unwrap() + 4..];
    let jwt = body
        .strip_prefix("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=")
        .unwrap_or_else(|| panic!("{body}"));
    assert_eq!(jwt.split('.').count(), 3);
    // Its audience is the key file's token server.
    let claims = part(jwt.split('.').nth(1).unwrap());
    assert!(
        claims.contains(&format!(r#""aud":"{url}/token""#)),
        "{claims}"
    );
}

#[test]
fn a_token_about_to_end_is_not_kept() {
    let replies = vec![token_reply("ya29.one", 60), token_reply("ya29.two", 60)];
    let (url, server) = fake_server(replies);
    let account = account_at(&url);
    let wait = Duration::from_secs(5);
    let scope = "https://www.googleapis.com/auth/short";
    assert_eq!(
        google::token(&account, scope, wait).ok().unwrap(),
        "ya29.one"
    );
    assert_eq!(
        google::token(&account, scope, wait).ok().unwrap(),
        "ya29.two"
    );
    assert_eq!(server.join().unwrap().len(), 2);
}

#[test]
fn a_scope_must_be_one_line_of_text() {
    for scope in ["", "  ", "a\nb"] {
        let call = format!(r#"{{"url":"x","auth":{{"GoogleToken":{{"scope":{scope:?}}}}}}}"#);
        assert!(
            matches!(wire::Call::read(&call), Err(Failure::Failed(0, m)) if m.contains("needs a scope")),
            "{scope:?}"
        );
    }
}
