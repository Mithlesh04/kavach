//! Networking for every way of running Kavach: TCP and UDP (spec 14.10),
//! TLS clients (decision 0021), the model call behind `ask<T>` (spec
//! 8.4) and the OpenTelemetry export of log events (14.12). A crate of its own, apart from `kavach-runtime-core`, so the
//! code a program uses without networking never shares a compiled unit
//! with TLS, and the linker can leave TLS out (decision 0024).

pub mod ai;
pub mod crypto;
pub mod net;
pub mod otlp;
pub mod random;
pub mod websocket;
