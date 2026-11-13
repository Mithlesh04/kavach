//! `std.crypto` in the interpreter (spec 14.19, decision 0093), from
//! `kavach_runtime_net::crypto`, as native code does.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_runtime_net::crypto::{self, CryptoFail};

impl<'a> Interp<'a> {
    pub(crate) fn crypto_call(&mut self, name: &str, args: &[Value<'a>], span: Span) -> Eval<'a> {
        let bytes = |i: usize| match args.get(i) {
            Some(Value::Bytes(b)) => b.to_vec(),
            _ => Vec::new(),
        };
        let text = |i: usize| match args.get(i) {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let b = |v: Vec<u8>| Value::Bytes(v.as_slice().into());
        let done: Result<Value<'a>, CryptoFail> = match name {
            "sha256" => Ok(b(crypto::sha256(&bytes(0)))),
            "sha384" => Ok(b(crypto::sha384(&bytes(0)))),
            "sha512" => Ok(b(crypto::sha512(&bytes(0)))),
            "hmac_sha256" => Ok(b(crypto::hmac_sha256(&bytes(0), &bytes(1)))),
            "equal" => Ok(Value::Bool(crypto::equal(&bytes(0), &bytes(1)))),
            "random_bytes" => {
                let n = match args.first() {
                    Some(Value::Int(n, _)) => usize::try_from(*n).unwrap_or(0),
                    _ => 0,
                };
                Ok(b(crypto::random_bytes(n)))
            }
            "token" => Ok(self.str_value(&crypto::token())),
            "hash_password" => Ok(self.str_value(&crypto::hash_password(&text(0)))),
            "check_password" => Ok(Value::Bool(crypto::check_password(&text(0), &text(1)))),
            "ed25519_secret" => Ok(b(crypto::ed25519_secret())),
            "ed25519_public" => crypto::ed25519_public(&bytes(0)).map(b),
            "ed25519_sign" => crypto::ed25519_sign(&bytes(0), &bytes(1)).map(b),
            "ed25519_verify" => Ok(Value::Bool(crypto::ed25519_verify(
                &bytes(0),
                &bytes(1),
                &bytes(2),
            ))),
            "p256_secret" => Ok(b(crypto::p256_secret())),
            "p256_public" => crypto::p256_public(&bytes(0)).map(b),
            "p256_sign" => crypto::p256_sign(&bytes(0), &bytes(1)).map(b),
            "p256_verify" => Ok(Value::Bool(crypto::p256_verify(
                &bytes(0),
                &bytes(1),
                &bytes(2),
            ))),
            "rsa_verify" => Ok(Value::Bool(crypto::rsa_verify(
                &bytes(0),
                &bytes(1),
                &bytes(2),
                &bytes(3),
                matches!(args.get(4), Some(Value::Bool(true))),
            ))),
            "aes_key" => Ok(b(crypto::aes_key())),
            "encrypt" => crypto::encrypt(&bytes(0), &bytes(1)).map(b),
            "decrypt" => crypto::decrypt(&bytes(0), &bytes(1)).map(b),
            _ => return Err(self.panic(format!("std.crypto has no function `{name}`"), span)),
        };
        done.map_err(|e| {
            Flow::Fail(match e {
                CryptoFail::BadKey(why) => {
                    let reason = vec![("reason", self.str_value(&why))];
                    self.builtin_variant("CryptoError", "BadKey", reason)
                }
                CryptoFail::Tampered => self.builtin_variant("CryptoError", "Tampered", Vec::new()),
            })
        })
    }
}
