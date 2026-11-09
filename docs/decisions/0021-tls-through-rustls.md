# 0021. TLS through rustls, with root certificates built in

- **Status:** Accepted
- **Date:** 2026-10-31

## Context

`std.net` (spec 14.10) has plain TCP, and `std.http` (14.11) can only fetch `http://` URLs. Most of the web, and every API worth calling, needs `https://`: TLS. The platform plan asks for TLS on by default, certificates always checked, and no "accept any certificate" switch outside tests.

TLS is too large and too security-sensitive to write ourselves. The choice is which library, and where the trusted root certificates come from. It has to build on Windows, Linux and macOS with the toolchain we already use, and work with tasks that take turns (decision 0020): a task waiting for data must let others run.

## Decision

- **rustls** (Rust, no OpenSSL), with the **ring** crypto backend, in `runtime/core`, so the interpreter and native code use the same code. ring builds with the C compiler Rust already needs; it was tried on Windows and Linux before this decision.
- **Root certificates built in:** Mozilla's list, from `webpki-roots`. A program checks certificates the same way on every machine, and doesn't depend on what an operating system has installed. A new Kavach release brings an updated list.
- **TLS 1.3, and 1.2** for servers that don't have 1.3 yet (rustls' safe defaults). Certificates are always checked against the host name; there is no switch to turn that off.
- **`net.connect_tls(address)`** gives a `Connection` like `net.connect`; `std.http` uses it for `https://`.
- The handshake runs on the helper thread that already connects (spec 14.10). After it, a read waits at most 2 ms at a time, so it fits the same try-and-wait loop as plain TCP.

## Consequences

- `https://` works, checked, with nothing to configure.
- Every program that links the runtime carries rustls and ring: a larger executable (about a megabyte) even without TLS. The linker drops what a program doesn't use where it can.
- Root certificates age with the Kavach release a program was built with.
- Still to come: TLS servers (a certificate and key), client certificates (mutual authentication), and an option to use the operating system's certificates.

## Alternatives considered

- **OpenSSL, or the system's TLS (SChannel, Secure Transport):** three different libraries and behaviours, and OpenSSL must be installed to build. Rejected: one behaviour everywhere matters more.
- **aws-lc-rs as the crypto backend:** faster and FIPS-capable, but needs CMake (and NASM on Windows) to build. Possible later, behind the same interface.
- **The operating system's root certificates (`rustls-native-certs`):** follows the machine's trust settings, but a program then checks differently on each machine, and containers often have none. Kept as a later option.
- **Writing TLS ourselves:** no.
