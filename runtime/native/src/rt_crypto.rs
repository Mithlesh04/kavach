//! `std.crypto` in native code (spec 14.19, decision 0093), from
//! `kavach_runtime_net::crypto`, as the interpreter does. An entry of its
//! own, so only a program that uses it links `ring`'s and `argon2`'s code
//! (decision 0024).

use crate::heap::release;
use crate::rt::type_of;
use crate::rt_bytes::{new_bytes, with_bytes};
use crate::rt_errors::set_error;
use crate::shapes::{new_str, str_of};
use kavach_kir::{RtOp, Shape};
use kavach_runtime_net::crypto::{self, CryptoFail};

/// Leaves `e` as a `CryptoError` for `kv_take_error`.
fn fail(e: CryptoFail) -> i64 {
    let Some(ty) = crate::shapes::find_union("CryptoError", "BadKey") else {
        return 0;
    };
    let error = match e {
        CryptoFail::BadKey(why) => {
            let text = crate::shapes::new_text(&why, crate::shapes::first_of(&Shape::Str));
            crate::rt_json::new_variant(ty, "BadKey", &[text as u64])
        }
        CryptoFail::Tampered => crate::rt_json::new_variant(ty, "Tampered", &[]),
    };
    set_error(error);
    0
}

fn done(r: Result<i64, CryptoFail>) -> i64 {
    r.unwrap_or_else(fail)
}

/// How many of each function's operands are objects (`Bytes` or text),
/// by its number.
const OBJECTS: [usize; 18] = [1, 1, 2, 2, 0, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 2, 2, 1];

crate::rt_data::entry!(kv_rt_crypto, |op, a, b, c, code, _line| {
    let ty = type_of(code);
    let bytes = |p: i64| with_bytes(p, <[u8]>::to_vec);
    let text = |p: i64| str_of(p);
    // An RSA check: a list of four `Bytes`, and whether PSS.
    if op == RtOp::CryptoRsaVerify {
        let parts: Vec<Vec<u8>> = crate::heap::slots(a)
            .iter()
            .map(|p| bytes(*p as i64))
            .collect();
        release(a);
        let [n, e, data, sig] = parts.as_slice() else {
            return 0;
        };
        return i64::from(crypto::rsa_verify(n, e, data, sig, b != 0));
    }
    // A signature check: three objects, nothing selected.
    if matches!(op, RtOp::CryptoEd25519Verify | RtOp::CryptoP256Verify) {
        let (public, data, sig) = (bytes(a), bytes(b), bytes(c));
        for p in [a, b, c] {
            release(p);
        }
        let ok = match op {
            RtOp::CryptoEd25519Verify => crypto::ed25519_verify(&public, &data, &sig),
            _ => crypto::p256_verify(&public, &data, &sig),
        };
        return i64::from(ok);
    }
    let out = match c {
        0 => new_bytes(crypto::sha256(&bytes(a)), ty),
        1 => new_bytes(crypto::sha512(&bytes(a)), ty),
        2 => new_bytes(crypto::hmac_sha256(&bytes(a), &bytes(b)), ty),
        3 => i64::from(crypto::equal(&bytes(a), &bytes(b))),
        4 => new_bytes(crypto::random_bytes(usize::try_from(a).unwrap_or(0)), ty),
        5 => new_str(crypto::token(), ty),
        6 => new_str(crypto::hash_password(&text(a)), ty),
        7 => i64::from(crypto::check_password(&text(a), &text(b))),
        8 => new_bytes(crypto::ed25519_secret(), ty),
        9 => done(crypto::ed25519_public(&bytes(a)).map(|v| new_bytes(v, ty))),
        10 => done(crypto::ed25519_sign(&bytes(a), &bytes(b)).map(|v| new_bytes(v, ty))),
        11 => new_bytes(crypto::p256_secret(), ty),
        12 => done(crypto::p256_public(&bytes(a)).map(|v| new_bytes(v, ty))),
        13 => done(crypto::p256_sign(&bytes(a), &bytes(b)).map(|v| new_bytes(v, ty))),
        14 => new_bytes(crypto::aes_key(), ty),
        15 => done(crypto::encrypt(&bytes(a), &bytes(b)).map(|v| new_bytes(v, ty))),
        16 => done(crypto::decrypt(&bytes(a), &bytes(b)).map(|v| new_bytes(v, ty))),
        17 => new_bytes(crypto::sha384(&bytes(a)), ty),
        _ => unreachable!("not a crypto function: {c}"),
    };
    // The operands are owned.
    for p in [a, b]
        .into_iter()
        .take(OBJECTS.get(c as usize).copied().unwrap_or(0))
    {
        release(p);
    }
    out
});
