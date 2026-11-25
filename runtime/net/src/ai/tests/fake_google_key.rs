// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A FAKE Google service account's RSA key, made with
//! `openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048` for the
//! tests of `Auth.GoogleToken` (spec 8.4). It belongs to no account. The
//! runtime's own tests and `tools/cli/tests/ai_google.rs` read it. Only
//! its base64 body is kept here, so tools that look for leaked keys in
//! the repository don't stop at a test's fake one; `pem` adds the lines
//! around it.

#![allow(dead_code)]

/// The key's PKCS #8 bytes in base64, as a PEM file's body has them.
const BODY: &str = "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDmPQYCSJGPMc1j
NQ2AK3jHOFOq9gCye6gPqVr0wNIz43t0tKHUo/kQpH7wnwDw3WaYk/qBVgc9VRVf
sE9uyIUNaRTsfoScKe+Zt60u/iGnNgSqqLCOM5ggegPGUB+G0egDwTcEqOx2lO/O
A313MK+9+1xwoMdrynKYcTZNaON9Jh4CNX8t0WPT/2++1r+R8Y7lhL6zrEM5OBPM
DX61LnuoFvxHQbB511HlTfBdiaBWCudF7uugWLEQtlhvqYp/rGLBh01nHdGqwQ0E
uBdcILmAB7ognZt3BqaXGPgVCidQAOVpI3tWlofejjEbpRNerbV02mqn6TQVGFUL
+ULxTCVpAgMBAAECggEAQ3iPmby1d16zX0bNUYmrx8oAbuHPm350Zj6Nips1HoKk
C7PhluSEmTVzg8U869kd90mqSBuzXGxiKpwK8x6tKxxnxtRevTrm3ce/Zz2PFCbW
4EI2Fz4UbzQChiu1yXcbBHqrSmDMhwWnbtqD+plYbxbJ5LGhLzvxMC5BKRc7T1F9
fbcO6SXE6pTLCTUfTfCC4dRUqnOIyIRyF84zVZqsUVgmTocAdevaRZmDHFouzAO2
jzi840Fz+lpdezFVDMyJZMS72UNbfC2/GoD0/3gLKCOHPNyOH+1GRJEmEKWcPpbG
8/nII0VtCfq/DDxyd5qxlIiaEr4knoM/314tbNfOjwKBgQD1VScqRbzAgULhBSoo
TjDR82Hv5r+qdw7QQZuuvLlZrX2+udXGGQR8Gqz6VbTwJH6vpnclxKeFeO3O8MVN
1ICdboOecd5Ktsm1JF++sN7nyl9xqaHz5yY2YzqooE11GiEkmJUJkAzWnWDwB3+R
MW3Be/jTSWoexnf+L+ktTwt0VwKBgQDwP9pwLQ73Jdi8C66R9WyoafIKtoBUzct4
H1TTU1TX0DDxDwN+BgufeXEJ5LQhAVkasSwSt1uhtVSPJz04ul9ETcfmJGlx9xbA
ITt86bSQi7vK3m+norS8ITAtse/t+H3H2x+kLi4/b2Sf6WmDUol4MPgp1ElVdA6u
u549aKccPwKBgQCxJ1gsskbmS3DbxDr4991Q/gqOjXWBxo8v2vBscWL2Ws0eWFsB
vTb71tdpvVH2xqOkevQ2lZbSTsyKTlprJTPqes8YLulx0iQ9ZZKtxFJx1jStguyr
jxKM24KUGY1VLhUYtAWK7jYXE3XYXdHEKoky2i6Hg8/ZgT9MkZQzX4zJ6QKBgQCm
hYuxbvnNgnUMOgig8HyE6xIyLLcckJv0Qbs40qye/ZW+sApaonQR8EnScqwLqHSL
VLxYqLChkB7xLmB33m5D2KmyshBIRtyFBUPHW/GkssXSYoT/q34cW+CZ+5hy2GbO
TOpMkRnLLDmRwWY5+M6B+Wc5DwRZb5zxreeLZutVXwKBgC78Ki9oRUZMl8JS6+/f
OdQu+V5kK4hCBtw0c8VwTUyXjqNSBdzqVedfLIdeyP7U9a2+LkntGvZWuFd/9t1c
tTPsrDHL7lQWfh6U+X7ElFZJ89fis5UhpwbREJYeOvMX7EVtRXp9iKcJ7tpggw3I
qh/MbQEIsaxZrQhGPLqvXIM0";

/// Its public key's modulus, in hex, as `openssl rsa -modulus` gives it;
/// the exponent is 65537.
pub const MODULUS: &str = "E63D060248918F31CD63350D802B78C73853AAF600B27BA80FA95AF4C0D233E37B74B4A1D4A3F910A47EF09F00F0DD669893FA8156073D55155FB04F6EC8850D6914EC7E849C29EF99B7AD2EFE21A73604AAA8B08E3398207A03C6501F86D1E803C13704A8EC7694EFCE037D7730AFBDFB5C70A0C76BCA729871364D68E37D261E02357F2DD163D3FF6FBED6BF91F18EE584BEB3AC43393813CC0D7EB52E7BA816FC4741B079D751E54DF05D89A0560AE745EEEBA058B110B6586FA98A7FAC62C1874D671DD1AAC10D04B8175C20B98007BA209D9B7706A69718F8150A275000E569237B569687DE8E311BA5135EADB574DA6AA7E9341518550BF942F14C2569";

/// The client email of the fake account.
pub const EMAIL: &str = "robot@fake-project.iam.gserviceaccount.com";

/// The key in PEM, as a key file's `private_key` has it.
pub fn pem() -> String {
    let kind = "PRIVATE KEY";
    format!("-----BEGIN {kind}-----\n{BODY}\n-----END {kind}-----\n")
}

/// A fake key file for the account, whose token server is `token_uri`.
pub fn key_file(token_uri: &str) -> String {
    let key = pem().replace('\n', "\\n");
    format!(
        "{{\"type\":\"service_account\",\"project_id\":\"fake-project\",\"private_key_id\":\"fake-key-id\",\"private_key\":\"{key}\",\"client_email\":\"{EMAIL}\",\"token_uri\":\"{token_uri}\"}}"
    )
}

/// The modulus as bytes, for checking a signature.
pub fn modulus() -> Vec<u8> {
    (0..MODULUS.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&MODULUS[i..i + 2], 16).unwrap())
        .collect()
}
