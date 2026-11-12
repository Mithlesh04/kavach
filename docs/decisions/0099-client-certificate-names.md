# 0099. A client certificate's names, from `x509-cert`

- **Status:** Accepted
- **Date:** 2026-11-13

## Context

With mutual TLS (`net.listen_tls(…, clients: ca)`, spec 14.10), `rustls` checks a client certificate's chain, but a server can't see whose certificate it was. Rath, a web framework written in Kavach, needs that name for allow-lists between services: `callers: ["orders"]`, often a SPIFFE id such as `spiffe://shop/orders`, which is a URI subject-alternative name. Reading a certificate's names takes an X.509 parser, which Kavach doesn't have: `rustls-webpki` checks certificates but doesn't hand out their common name or URI names. A maintainer approved adding one (2026-11-13). The number 0098 is kept for the record about moving the modules written in Kavach out.

## Decision

- **`x509-cert`** (RustCrypto, pure Rust), with the `der`, `spki` and `const-oid` crates it brings. It only reads the certificate the TLS handshake already checked; it decides nothing about trust.
- `conn.client_cert() -> ClientCert?` (spec 14.10): `none` without mutual TLS. `ClientCert { common_name: String, dns: [String], uris: [String] }`, from the leaf certificate's subject and subject-alternative names.
- A served `Request` carries it as `r.client_cert: ClientCert?` (spec 14.11), `none` for a request made by hand.

## Consequences

- A service can allow callers by name with no other library.
- The parser is in `runtime/net`, so a program that doesn't use the network links none of it (decision 0024).

## Alternatives considered

- **`x509-parser`** (rusticata): more complete, but brings `nom`, `asn1-rs`, `der-parser` and `oid-registry`.
- **Only DNS names, from `rustls-webpki`:** no new crate, but no URI names, so no SPIFFE ids.
