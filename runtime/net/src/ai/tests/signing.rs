// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! AWS Signature Version 4 (spec 8.4, *Signing AWS requests*), checked
//! against AWS's published test suite: the cases `get-vanilla`,
//! `post-vanilla`, `get-vanilla-query-order-key-case`,
//! `post-x-www-form-urlencoded` and `post-sts-header-before` of
//! `awslabs/aws-c-auth`, `tests/aws-signing-test-suite/v4/`, read on
//! 2026-11-22. Their credentials are AWS's documented example ones, not
//! real.

use super::super::http::Target;
use super::super::sigv4::{self, Keys, Scope};
use super::*;

const DATE: &str = "20150830T123600Z";
const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const TOKEN: &str = "AQoDYXdzEPT//////////wEXAMPLEtc764bNrC9SAPBSM22wDOk4x4HIZ8j4FZTwdQWLWsKWHGBuFqwAeMicRXmxfpSPfIeoIYRqTflfKD8YUuwthAx7mSEI/qkPpKPi/kMcGdQrmGdeehM4IC1NtBmUpp2wUE8phUZampKsburEDy0KPkyQDYwT7WZ0wq5VSXDvp75YU9HFvlRd8Tx6q6fE8YQcHNVXAkiY9q6d+xo0rKwT38xVqr7ZD0u0iPPkUL64lIZbqBAz+scqKmlzm8FDrypNC9Yjc8fPOLn9FX9KSYvKTr4rvx3iSIlTJabIQwj2ICCR/oLxBA==";

fn example_keys(token: Option<&str>) -> Keys {
    Keys {
        id: "AKIDEXAMPLE".into(),
        secret: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
        token: token.map(String::from),
    }
}

fn scope() -> Scope {
    Scope {
        service: "service".into(),
        region: "us-east-1".into(),
    }
}

/// One case of the suite: the request, and the canonical request, string
/// to sign and signature it expects.
struct Case {
    method: &'static str,
    target: &'static str,
    headers: &'static [(&'static str, &'static str)],
    body: &'static str,
    token: Option<&'static str>,
    canonical: String,
    to_sign: String,
    signature: &'static str,
}

fn host_only(method: &'static str, target: &'static str, query: &str) -> Case {
    Case {
        method,
        target,
        headers: &[("Host", "example.amazonaws.com")],
        body: "",
        token: None,
        canonical: format!(
            "{method}\n/\n{query}\nhost:example.amazonaws.com\nx-amz-date:{DATE}\n\nhost;x-amz-date\n{EMPTY_HASH}"
        ),
        to_sign: String::new(),
        signature: "",
    }
}

fn string_to_sign(hash: &str) -> String {
    format!("AWS4-HMAC-SHA256\n{DATE}\n20150830/us-east-1/service/aws4_request\n{hash}")
}

fn cases() -> Vec<Case> {
    let body_hash = "9095672bbd1f56dfc5b65f3e153adc8731a4a654192329106275f4c7b24d0b6e";
    vec![
        Case {
            to_sign: string_to_sign(
                "bb579772317eb040ac9ed261061d46c1f17a8133879d6129b6e1c25292927e63",
            ),
            signature: "5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31",
            ..host_only("GET", "/", "")
        },
        Case {
            to_sign: string_to_sign(
                "553f88c9e4d10fc9e109e2aeb65f030801b70c2f6468faca261d401ae622fc87",
            ),
            signature: "5da7c1a2acd57cee7505fc6676e4e544621c30862966e37dddb68e92efbe5d6b",
            ..host_only("POST", "/", "")
        },
        Case {
            to_sign: string_to_sign(
                "816cd5b414d056048ba4f7c5386d6e0533120fb1fcfa93762cf0fc39e2cf19e0",
            ),
            signature: "b97d918cfa904a5beff61c982a1b6f458b799221646efd99d3219ec94cdf2500",
            ..host_only(
                "GET",
                "/?Param2=value2&Param1=value1",
                "Param1=value1&Param2=value2",
            )
        },
        Case {
            method: "POST",
            target: "/",
            headers: &[
                ("Content-Type", "application/x-www-form-urlencoded"),
                ("Host", "example.amazonaws.com"),
                ("Content-Length", "13"),
                (
                    "x-amz-content-sha256",
                    "9095672bbd1f56dfc5b65f3e153adc8731a4a654192329106275f4c7b24d0b6e",
                ),
            ],
            body: "Param1=value1",
            token: None,
            canonical: format!(
                "POST\n/\n\ncontent-length:13\ncontent-type:application/x-www-form-urlencoded\nhost:example.amazonaws.com\nx-amz-content-sha256:{body_hash}\nx-amz-date:{DATE}\n\ncontent-length;content-type;host;x-amz-content-sha256;x-amz-date\n{body_hash}"
            ),
            to_sign: string_to_sign(
                "b1edd1d03544c25390e32085d55b57acc9a3961bb59415ff86c45c3d89d16cfb",
            ),
            signature: "d3875051da38690788ef43de4db0d8f280229d82040bfac253562e56c3f20e0b",
        },
        Case {
            token: Some(TOKEN),
            canonical: format!(
                "POST\n/\n\nhost:example.amazonaws.com\nx-amz-date:{DATE}\nx-amz-security-token:{TOKEN}\n\nhost;x-amz-date;x-amz-security-token\n{EMPTY_HASH}"
            ),
            to_sign: string_to_sign(
                "c237e1b440d4c63c32ca95b5b99481081cb7b13c7e40434868e71567c1a882f6",
            ),
            signature: "85d96828115b5dc0cfc3bd16ad9e210dd772bbebba041836c64533a82be05ead",
            ..host_only("POST", "/", "")
        },
    ]
}

#[test]
fn signatures_match_aws_test_suite() {
    for case in cases() {
        let headers: Vec<(String, String)> = (case.headers.iter())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let keys = example_keys(case.token);
        let signed = sigv4::sign(
            case.method,
            case.target,
            &headers,
            case.body.as_bytes(),
            &keys,
            &scope(),
            DATE,
        );
        let name = format!("{} {}", case.method, case.target);
        assert_eq!(signed.canonical, case.canonical, "{name}");
        assert_eq!(signed.to_sign, case.to_sign, "{name}");
        assert_eq!(signed.signature, case.signature, "{name}");
        let authorization = &signed.headers.last().unwrap().1;
        assert!(
            authorization.starts_with(
                "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, SignedHeaders="
            ),
            "{authorization}"
        );
        assert!(authorization.ends_with(&format!(", Signature={}", case.signature)));
        let token = signed
            .headers
            .iter()
            .find(|(k, _)| k == "x-amz-security-token");
        assert_eq!(token.map(|(_, v)| v.as_str()), case.token, "{name}");
    }
}

#[test]
fn the_path_is_encoded_once_more_and_the_query_sorted() {
    let signed = sigv4::sign(
        "POST",
        "/model/anthropic.claude-v1%3A0/converse?b=2&a=x%20y&a=1",
        &[("host".into(), "h".into())],
        b"",
        &example_keys(None),
        &scope(),
        DATE,
    );
    let mut lines = signed.canonical.lines();
    assert_eq!(
        lines.nth(1),
        Some("/model/anthropic.claude-v1%253A0/converse")
    );
    assert_eq!(lines.next(), Some("a=1&a=x%20y&b=2"));
}

#[test]
fn the_date_is_written_as_aws_writes_it() {
    assert_eq!(sigv4::amz_date(1_440_938_160_000), DATE);
}

#[test]
fn credentials_need_an_id_and_a_secret() {
    let read = |vars: &'static [(&'static str, &'static str)]| {
        Keys::read(|name| {
            (vars.iter())
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
        })
    };
    assert_eq!(read(&[]).err(), Some("AWS_ACCESS_KEY_ID"));
    // Set to empty text: not set.
    let only_id = read(&[
        ("AWS_ACCESS_KEY_ID", "FAKEKEY"),
        ("AWS_SECRET_ACCESS_KEY", " "),
    ]);
    assert_eq!(only_id.err(), Some("AWS_SECRET_ACCESS_KEY"));
    const BOTH: [(&str, &str); 3] = [
        ("AWS_ACCESS_KEY_ID", "FAKEKEY"),
        ("AWS_SECRET_ACCESS_KEY", "fake-secret"),
        ("AWS_SESSION_TOKEN", ""),
    ];
    let keys = read(&BOTH).ok().unwrap();
    assert_eq!((keys.id.as_str(), keys.token), ("FAKEKEY", None));
}

#[test]
fn a_scope_is_letters_digits_and_dashes() {
    let scope = |service: &str, region: &str| Scope {
        service: service.into(),
        region: region.into(),
    };
    assert!(scope("bedrock", "us-east-1").valid());
    assert!(!scope("", "us-east-1").valid());
    assert!(!scope("bedrock", "us/east").valid());
    assert!(
        matches!(wire::Call::read(r#"{"url":"x","auth":{"AwsSignature":{"service":"bedrock","region":""}}}"#), Err(Failure::Failed(0, m)) if m.contains("letters, digits and `-`"))
    );
}

#[test]
fn the_host_header_has_the_port_only_when_it_isnt_the_schemes() {
    let host = |url| Target::of(url).ok().map(|t| t.host.to_string());
    assert_eq!(
        host("http://127.0.0.1:8080/x"),
        Some("127.0.0.1:8080".into())
    );
    assert_eq!(host("https://a.example:443/x"), Some("a.example".into()));
    assert_eq!(host("https://a.example/x"), Some("a.example".into()));
    let t = Target::of("https://a.example").ok().unwrap();
    assert_eq!((t.address.as_str(), t.path), ("a.example:443", "/"));
}

/// A request as the fake server got it: its path, headers and body.
fn taken_apart(request: &str) -> (String, Vec<(String, String)>, String) {
    let (head, body) = request.split_once("\r\n\r\n").unwrap();
    let mut lines = head.split("\r\n");
    let path = lines.next().unwrap().split(' ').nth(1).unwrap().to_string();
    let headers = lines
        .filter_map(|l| l.split_once(": "))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    (path, headers, body.to_string())
}

#[test]
fn each_try_is_signed_for_the_address_it_is_sent_to() {
    let (url, server) = fake_server(vec![BUSY, OK]);
    let mut f = fake(&url);
    f.auth = r#"{"AwsSignature":{"service":"bedrock","region":"us-east-1"}}"#;
    let asked = ask_fake(&mut f, 2);
    assert!(asked.result.is_ok(), "{:?}", asked.result.err());
    let sent = server.join().unwrap();
    assert_eq!(sent.len(), 2);
    let keys = Keys {
        id: "FAKEKEY".into(),
        secret: "fake-secret".into(),
        token: Some("fake-token".into()),
    };
    let scope = Scope {
        service: "bedrock".into(),
        region: "us-east-1".into(),
    };
    for request in &sent {
        let (path, headers, body) = taken_apart(request);
        let get = |name: &str| (headers.iter().find(|(k, _)| k == name)).map(|(_, v)| v.clone());
        let date = get("x-amz-date").unwrap();
        assert_eq!(get("x-amz-security-token").as_deref(), Some("fake-token"));
        // The host has the fake server's port: what was sent is signed.
        assert_eq!(get("host"), Some(url["http://".len()..].to_string()));
        // What the server works out from what it got matches.
        let covered: Vec<(String, String)> = (headers.iter())
            .filter(|(k, _)| {
                !["authorization", "content-length", "connection"].contains(&k.as_str())
            })
            .filter(|(k, _)| k != "x-amz-date" && k != "x-amz-security-token")
            .cloned()
            .collect();
        let want = sigv4::sign(
            "POST",
            &path,
            &covered,
            body.as_bytes(),
            &keys,
            &scope,
            &date,
        );
        let authorization = get("authorization").unwrap();
        assert_eq!(authorization, want.headers.last().unwrap().1, "{request}");
        assert!(
            authorization.starts_with(&format!(
                "AWS4-HMAC-SHA256 Credential=FAKEKEY/{}/us-east-1/bedrock/aws4_request, SignedHeaders=content-type;host;x-amz-date;x-amz-security-token;x-model, Signature=",
                &date[..8]
            )),
            "{authorization}"
        );
        // Neither the secret nor the key of another scheme is sent.
        assert!(!request.contains("fake-secret") && !request.contains("x-api-key"));
    }
}
