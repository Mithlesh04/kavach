//! The signatures of `std.crypto` (spec 14.19, decision 0093): hashes,
//! passwords, signatures and encryption, none needing a capability.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};

type Signature = (Vec<(String, Ty, bool)>, Ty);

impl Checker<'_> {
    /// The parameters and result of `crypto.<name>`, or `None` when there
    /// is no such function.
    pub(crate) fn crypto_signature(&mut self, name: &str) -> Option<Signature> {
        let (int, string, bytes) = (
            Ty::Prim(Prim::Int),
            Ty::Prim(Prim::String),
            Ty::Prim(Prim::Bytes),
        );
        let boolean = Ty::Prim(Prim::Bool);
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        let fails = Ty::Result(
            Box::new(bytes.clone()),
            Box::new(self.builtin_ty("CryptoError")),
        );
        let b = |names: &[&str]| -> Vec<(String, Ty, bool)> {
            names.iter().map(|n| p(n, &bytes)).collect()
        };
        Some(match name {
            "sha256" | "sha384" | "sha512" => (b(&["data"]), bytes.clone()),
            "hmac_sha256" => (b(&["key", "data"]), bytes.clone()),
            "equal" => (b(&["a", "b"]), boolean),
            "random_bytes" => (vec![p("n", &int)], bytes.clone()),
            "token" => (vec![], string),
            "hash_password" => (vec![p("password", &string)], string),
            "check_password" => (vec![p("password", &string), p("hash", &string)], boolean),
            "ed25519_secret" | "p256_secret" | "aes_key" => (vec![], bytes.clone()),
            "ed25519_public" | "p256_public" => (b(&["key"]), fails),
            "ed25519_sign" | "p256_sign" => (b(&["key", "data"]), fails),
            "ed25519_verify" | "p256_verify" => {
                (b(&["public", "data", "signature"]), boolean.clone())
            }
            "rsa_verify" => {
                let mut params = b(&["n", "e", "data", "signature"]);
                params.push(("pss".to_string(), boolean.clone(), true));
                (params, boolean)
            }
            "encrypt" => (b(&["key", "data"]), fails),
            "decrypt" => (b(&["key", "sealed"]), fails),
            _ => return None,
        })
    }
}
