# 0093. std.crypto from ring and argon2

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

Web services hash passwords, sign tokens, compare secrets and encrypt data at rest. Rath, a web framework written in Kavach, asked (2026-11-12) for `std.crypto`: SHA-2, HMAC, constant-time compare, random tokens, Argon2id, Ed25519, ECDSA and AES-GCM, through a vetted library, never hand-written. Hand-written cryptography fails in ways no test shows.

A maintainer chose (2026-11-12), from the options put to them, the libraries and the capability rule below.

## Decision

- **`ring`** for everything it has: SHA-256 and SHA-512, HMAC, constant-time comparison, the system's secure random generator, Ed25519, ECDSA P-256 and AES-256-GCM. It is already in every build that speaks TLS ([decision 0021](0021-tls-through-rustls.md)).
- **`argon2`** (RustCrypto, pure Rust) for Argon2id password hashing, which `ring` doesn't have: the one new outside dependency, with its default settings (19 MiB, 2 passes, 1 lane, as OWASP recommends). It brings four small RustCrypto crates with it: `password-hash`, `blake2`, `base64ct` and `rand_core`; its salt comes from `ring`'s generator, so none of `rand`'s randomness is used.
- **No capability.** Hashing, signing, encrypting and making random keys change nothing outside the program, as Go, Rust and Python treat them; `Random` stays for `std.random`'s numbers.
- **One module, `std.crypto`** (spec 14.19), in the runtime crate that has the network (`runtime/net`), so a program without it links none of it (decision 0024).

## Consequences

- A service stores passwords, signs JWTs (`ES256`, `EdDSA`) and seals data with no other library.
- Keys are plain `Bytes` for now; making them secrets that can't be printed (8.7) is left open.

## Alternatives considered

- **RustCrypto throughout** (`sha2`, `hmac`, `ed25519-dalek`, `p256`, `aes-gcm`): about seven new crates where `ring` already covers them.
- **`aws-lc-rs`:** a large C build that would also replace `ring` under TLS.
- **`Random` for new keys and tokens:** keys would need a capability that says nothing about what the code can touch.
