# 14.19 std.crypto

Hashes, message authentication, password hashing, signatures and encryption, from vetted libraries: `ring`, which TLS already uses, and `argon2` for passwords ([decision 0093](../../decisions/0093-std-crypto.md)). None of it is written by hand, and none of it needs a capability: it changes nothing outside the program (8.1).

| Function | Gives |
|---|---|
| `crypto.sha256(data) -> Bytes`, `crypto.sha384(data) -> Bytes`, `crypto.sha512(data) -> Bytes` | The SHA-256, SHA-384 or SHA-512 digest of `data` (`Bytes`). SHA-384 is what channel binding (RFC 5929) needs for a certificate signed with it |
| `crypto.hmac_sha256(key, data) -> Bytes` | HMAC-SHA-256 of `data` with `key` |
| `crypto.equal(a, b) -> Bool` | Whether two `Bytes` are the same, taking the same time wherever they differ, so a secret can't be guessed byte by byte from timing |
| `crypto.random_bytes(n) -> Bytes` | `n` random bytes from the system's secure generator |
| `crypto.token() -> String` | 32 random bytes as URL-safe base64 text: a session id, a reset link, an API key |
| `crypto.hash_password(password) -> String` | The password hashed with Argon2id, a new salt each time, as one text (`$argon2id$v=19$…`) to store |
| `crypto.check_password(password, hash) -> Bool` | Whether `password` is the one `hash` was made from; `false` for a `hash` that isn't one |
| `crypto.ed25519_secret() -> Bytes` | A new Ed25519 secret key |
| `crypto.ed25519_public(key) -> Bytes ! CryptoError` | The public key that goes with the secret key `key` |
| `crypto.ed25519_sign(key, data) -> Bytes ! CryptoError` | The signature of `data` |
| `crypto.ed25519_verify(public, data, signature) -> Bool` | Whether `signature` is `data` signed by the key `public` goes with |
| `crypto.p256_secret()`, `p256_public(key)`, `p256_sign(key, data)`, `p256_verify(public, data, signature)` | The same with ECDSA on P-256 and SHA-256, as JWTs (`ES256`) and WebAuthn use |
| `crypto.rsa_verify(n, e, data, signature, pss: false) -> Bool` | Whether `signature` is `data` signed with RSA and SHA-256 by the public key with modulus `n` and exponent `e` (big-endian `Bytes`, as a JWK's `n` and `e` decode): PKCS #1 v1.5 (`RS256`), or PSS (`PS256`) with `pss: true`. Keys of 2048 to 8192 bits. Only checking: OpenID Connect providers sign ID tokens this way, and a service signs its own with Ed25519 or P-256 |
| `crypto.aes_key() -> Bytes` | A new 256-bit AES key |
| `crypto.encrypt(key, data) -> Bytes ! CryptoError` | `data` sealed with AES-256-GCM: a new random nonce, then the ciphertext and its tag |
| `crypto.decrypt(key, sealed) -> Bytes ! CryptoError` | `data` back; fails with `Tampered` when `sealed` was changed or the key is another |

`CryptoError` is `BadKey(reason)` for a key of the wrong kind or size, or `Tampered`.

```kavach
use std.crypto

fn main() ! CryptoError {
    const stored = crypto.hash_password("correct horse")
    print(crypto.check_password("correct horse", stored))   // true
    print(crypto.check_password("wrong", stored))           // false

    const key = crypto.aes_key()
    const sealed = try crypto.encrypt(key, "card 4242".bytes())
    print(try crypto.decrypt(key, sealed) == "card 4242".bytes())   // true

    const signing = crypto.ed25519_secret()
    const signature = try crypto.ed25519_sign(signing, "order 7".bytes())
    print(crypto.ed25519_verify(try crypto.ed25519_public(signing), "order 7".bytes(), signature))   // true
}
```

Secret keys are `Bytes` to keep out of logs and prompts; store them as a program stores any secret (8.7).
