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

/// Whether `sig` is an RSA signature of `data` with SHA-256 by the public
/// key with modulus `n` and exponent `e` (big-endian, as a JWK has them):
/// RSASSA-PSS when `pss`, else PKCS #1 v1.5. Keys of 2048 to 8192 bits.
pub fn rsa_verify(n: &[u8], e: &[u8], data: &[u8], sig: &[u8], pss: bool) -> bool {
    let key = signature::RsaPublicKeyComponents { n, e };
    let params = if pss {
        &signature::RSA_PSS_2048_8192_SHA256
    } else {
        &signature::RSA_PKCS1_2048_8192_SHA256
    };
    key.verify(params, data, sig).is_ok()
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

    /// A 2048-bit key made with OpenSSL, and its two signatures of
    /// "order 7".
    const RSA_N: &str = "9ff5faaa63eb1ad63fbca04f1c53ab1efa28e998526d635bd09eef38e92ae7e8c60f0d12ae19b75351d4ca70c3419c4f45a55ba89e97047b96180adb0e234eff89d93e46efdd0b27de2c08621a7368149d587eda0c778dd92a928f420fb364a9155d80bddd9440af40b0768cb221819fa0c7f05c1181719cd2324914ef9752ffa63ef763a7e04988f5cd35895de8edb7943e37069baac0e20647a929572b87b84a4d37ef1b51bdb0fd791d6735f3520e2eca884c945377474515a9645da6091fd0744ec74efb1e5a22ad0f669b89eb784bfbd82d1924d989e79ce868f5499e0686f21bf96d277b8eee7784bfc5190086fb3e09ca2f3fed5a6928a129b18c03b1";
    const RSA_PKCS1: &str = "6c46a87062d9c1208828ad3704826b79f8a71d38881ee16cffc8ef732deda2c77528f6d34a081b1495c77801b425160e677a7a2a4a8b70ec5190f86b0f448edafd87dbadedb8174a79fa57dc0e57480f75f8d845d27750aa332cff43a251af277d1076fa61cac5d264324f47ef148eacfb370830c3e2009dd392056a778f6c772510f96c3eeb1ceb1f4e805d0ae4a070bb90c784031072a0926d1bd9fcaa38c4e6950e076e90116acd5b4981ef5b76dcb1950ec428615283b6294e914c8438ecb4b83ebd33b206552227ef14196b3dde30d8cfdbbd1384ba8a20134b996711de8ec9490267e6d24c52c73bc6976923a4e4cb40e4e077e548f265fbfb2cccbf31";
    const RSA_PSS: &str = "6f40a54e2c8d3da674eaba7a44cf97b97062a0087c719352ba2373d4eaa6ca9c3ec82151afa5256cdd99bce7fba47f55d3e2ae88109f7114f912259e875b06a4c4a4a51340252c0cf9aeec2aed46dcc1828805f8586635d3985f7212baaa53038b32dce1bcac5463aba8133d58b6f4c34e0a2144ceda3b17b79b1f5ecd6f3f4442b9efd2a7a980b6e072913a10c02616c493e0156acdd6af65cf74ed2f4f75474dc7c1a822e300bacb6919d07f636f65257db34b510f771dd51b683c75ba649ba1383a3fdefc454e2bd4b5934d3cdefbe39c906c789d93e40ef0f7f457e23a6ef75ab25dce598f88649900614992c5ebfdd25847be4ab32859c386c9a0dc1d6e";

    fn unhex(h: &str) -> Vec<u8> {
        (0..h.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn rsa_signatures_are_checked() {
        let (n, e) = (unhex(RSA_N), [1, 0, 1]);
        assert!(rsa_verify(&n, &e, b"order 7", &unhex(RSA_PKCS1), false));
        assert!(rsa_verify(&n, &e, b"order 7", &unhex(RSA_PSS), true));
        assert!(!rsa_verify(&n, &e, b"order 8", &unhex(RSA_PKCS1), false));
        assert!(!rsa_verify(&n, &e, b"order 7", &unhex(RSA_PKCS1), true));
        assert!(!rsa_verify(
            &n[1..],
            &e,
            b"order 7",
            &unhex(RSA_PKCS1),
            false
        ));
    }

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
