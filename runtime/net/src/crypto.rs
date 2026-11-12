//! `std.crypto` (spec 14.19, decision 0093): hashes, HMAC, constant-time
//! comparison, random bytes, Ed25519, ECDSA P-256 and AES-256-GCM from
//! `ring`, which TLS already uses; Argon2id passwords from `argon2`. Not a
//! line of cryptography is written here: each function only calls them.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{self, KeyPair};

/// Why a function of `std.crypto` failed (`CryptoError`).
#[derive(Debug, PartialEq)]
pub enum CryptoFail {
    BadKey(String),
    Tampered,
}

type Done<T> = Result<T, CryptoFail>;

fn bad_key(what: &str) -> CryptoFail {
    CryptoFail::BadKey(what.to_string())
}

pub fn sha256(data: &[u8]) -> Vec<u8> {
    ring::digest::digest(&ring::digest::SHA256, data)
        .as_ref()
        .to_vec()
}

pub fn sha512(data: &[u8]) -> Vec<u8> {
    ring::digest::digest(&ring::digest::SHA512, data)
        .as_ref()
        .to_vec()
}

pub fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, key);
    ring::hmac::sign(&key, data).as_ref().to_vec()
}

/// Whether `a` and `b` are the same, in time that doesn't depend on where
/// they differ.
pub fn equal(a: &[u8], b: &[u8]) -> bool {
    #[allow(deprecated)]
    ring::constant_time::verify_slices_are_equal(a, b).is_ok()
}

pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut out = vec![0u8; n];
    SystemRandom::new()
        .fill(&mut out)
        .expect("the operating system gave no random bytes");
    out
}

/// 32 random bytes as URL-safe base64 without padding.
pub fn token() -> String {
    const DIGITS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let data = random_bytes(32);
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let n =
            (chunk.iter().enumerate()).fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(DIGITS[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    out
}

/// `password` hashed with Argon2id, as a PHC text with its salt.
pub fn hash_password(password: &str) -> String {
    let salt = SaltString::encode_b64(&random_bytes(16)).expect("16 bytes make a salt");
    argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .expect("Argon2id hashes any password")
}

pub fn check_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|h| {
        argon2::Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}

pub fn ed25519_secret() -> Vec<u8> {
    signature::Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
        .expect("a new Ed25519 key")
        .as_ref()
        .to_vec()
}

fn ed25519(secret: &[u8]) -> Done<signature::Ed25519KeyPair> {
    signature::Ed25519KeyPair::from_pkcs8(secret).map_err(|_| bad_key("not an Ed25519 secret key"))
}

pub fn ed25519_public(secret: &[u8]) -> Done<Vec<u8>> {
    Ok(ed25519(secret)?.public_key().as_ref().to_vec())
}

pub fn ed25519_sign(secret: &[u8], data: &[u8]) -> Done<Vec<u8>> {
    Ok(ed25519(secret)?.sign(data).as_ref().to_vec())
}

pub fn ed25519_verify(public: &[u8], data: &[u8], sig: &[u8]) -> bool {
    signature::UnparsedPublicKey::new(&signature::ED25519, public)
        .verify(data, sig)
        .is_ok()
}

pub fn p256_secret() -> Vec<u8> {
    let alg = &signature::ECDSA_P256_SHA256_FIXED_SIGNING;
    signature::EcdsaKeyPair::generate_pkcs8(alg, &SystemRandom::new())
        .expect("a new P-256 key")
        .as_ref()
        .to_vec()
}

fn p256(secret: &[u8]) -> Done<signature::EcdsaKeyPair> {
    let alg = &signature::ECDSA_P256_SHA256_FIXED_SIGNING;
    signature::EcdsaKeyPair::from_pkcs8(alg, secret, &SystemRandom::new())
        .map_err(|_| bad_key("not a P-256 secret key"))
}

pub fn p256_public(secret: &[u8]) -> Done<Vec<u8>> {
    Ok(p256(secret)?.public_key().as_ref().to_vec())
}

pub fn p256_sign(secret: &[u8], data: &[u8]) -> Done<Vec<u8>> {
    let sig = p256(secret)?
        .sign(&SystemRandom::new(), data)
        .map_err(|_| bad_key("signing failed"))?;
    Ok(sig.as_ref().to_vec())
}

pub fn p256_verify(public: &[u8], data: &[u8], sig: &[u8]) -> bool {
    signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_FIXED, public)
        .verify(data, sig)
        .is_ok()
}

pub fn aes_key() -> Vec<u8> {
    random_bytes(32)
}

fn aes(key: &[u8]) -> Done<ring::aead::LessSafeKey> {
    let key = ring::aead::UnboundKey::new(&ring::aead::AES_256_GCM, key)
        .map_err(|_| bad_key("an AES key is 32 bytes"))?;
    Ok(ring::aead::LessSafeKey::new(key))
}

/// `data` sealed: a new random 12-byte nonce, then the ciphertext and tag.
pub fn encrypt(key: &[u8], data: &[u8]) -> Done<Vec<u8>> {
    let key = aes(key)?;
    let nonce: [u8; 12] = random_bytes(12).try_into().expect("12 bytes");
    let mut sealed = data.to_vec();
    key.seal_in_place_append_tag(
        ring::aead::Nonce::assume_unique_for_key(nonce),
        ring::aead::Aad::empty(),
        &mut sealed,
    )
    .map_err(|_| CryptoFail::Tampered)?;
    let mut out = nonce.to_vec();
    out.extend(sealed);
    Ok(out)
}

pub fn decrypt(key: &[u8], sealed: &[u8]) -> Done<Vec<u8>> {
    let key = aes(key)?;
    if sealed.len() < 12 + 16 {
        return Err(CryptoFail::Tampered);
    }
    let nonce: [u8; 12] = sealed[..12].try_into().expect("12 bytes");
    let mut data = sealed[12..].to_vec();
    let plain = key
        .open_in_place(
            ring::aead::Nonce::assume_unique_for_key(nonce),
            ring::aead::Aad::empty(),
            &mut data,
        )
        .map_err(|_| CryptoFail::Tampered)?;
    Ok(plain.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_function_does_what_its_library_does() {
        // FIPS 180-2's example and RFC 4231's first HMAC case.
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&hmac_sha256(&[0x0b; 20], b"Hi There")),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        assert!(equal(b"same", b"same") && !equal(b"same", b"sane"));
        assert_eq!(token().len(), 43);
        let stored = hash_password("correct horse");
        assert!(stored.starts_with("$argon2id$"));
        assert!(check_password("correct horse", &stored));
        assert!(!check_password("wrong", &stored) && !check_password("x", "not a hash"));
        let secret = ed25519_secret();
        let sig = ed25519_sign(&secret, b"order 7").unwrap();
        assert!(ed25519_verify(
            &ed25519_public(&secret).unwrap(),
            b"order 7",
            &sig
        ));
        assert!(!ed25519_verify(
            &ed25519_public(&secret).unwrap(),
            b"order 8",
            &sig
        ));
        let secret = p256_secret();
        let sig = p256_sign(&secret, b"order 7").unwrap();
        assert!(p256_verify(
            &p256_public(&secret).unwrap(),
            b"order 7",
            &sig
        ));
        let key = aes_key();
        let sealed = encrypt(&key, b"card 4242").unwrap();
        assert_eq!(decrypt(&key, &sealed).unwrap(), b"card 4242");
        let mut changed = sealed.clone();
        changed[20] ^= 1;
        assert_eq!(decrypt(&key, &changed), Err(CryptoFail::Tampered));
        assert!(matches!(
            encrypt(b"short", b"x"),
            Err(CryptoFail::BadKey(_))
        ));
    }
}
