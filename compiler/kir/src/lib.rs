//! KIR, the Kavach IR: a small, typed, structured form of a checked
//! program. Code generators read KIR, not the syntax tree.
//!
//! Lowering supports a growing subset of the language. Anything outside it
//! is reported as [`Unsupported`], never compiled wrongly; the interpreter
//! (decision 0014) still runs such programs.

pub mod borrows;
mod cancel;
mod coalesce;
mod collect;
mod counters;
mod devirt;
mod drop_scopes;
mod encode;
mod exits;
mod fuse;
mod gpu_later;
mod gpu_stay;
mod inline;
mod ir;
mod jam;
mod library;
mod lower;
mod lower_agents;
mod lower_ai;
mod lower_base64;
mod lower_c;
mod lower_calls;
mod lower_chain;
mod lower_cli;
mod lower_closure;
mod lower_convert;
mod lower_data;
mod lower_destructure;
mod lower_dispatch;
mod lower_drops;
mod lower_durable;
mod lower_enums;
mod lower_errors;
mod lower_events;
mod lower_every;
mod lower_expr;
mod lower_fn;
mod lower_fn_errors;
mod lower_generic;
mod lower_gpu;
mod lower_guard;
mod lower_iface;
mod lower_inout;
mod lower_json;
mod lower_json_checks;
mod lower_labels;
mod lower_lambda;
mod lower_log;
mod lower_match;
mod lower_methods;
mod lower_optional;
mod lower_power;
mod lower_process;
mod lower_ptr;
mod lower_ptr_syntax;
mod lower_replay;
mod lower_schema;
mod lower_selected;
mod lower_std;
mod lower_stmt;
mod lower_tasks;
mod lower_tls;
mod moves;
mod par;
mod passes;
mod prefetch;
mod prune;
mod ranges;
mod rt_op;
mod sroa;
mod stop_points;
mod types;
mod unrecurse;
mod unused;
mod version;
pub mod walk;
mod whole_lines;

pub use encode::{decode_shapes, encode_shapes};
pub use ir::*;
pub use library::{Library, library};
pub use lower::{lower, lower_with};
pub use rt_op::RtOp;

/// A language feature the native compiler doesn't handle yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsupported {
    pub feature: String,
    pub line: u32,
}

/// The functions of `std.crypto` (spec 14.19), in the order their number
/// (`RtOp::Crypto`'s operand `c`) gives.
pub const CRYPTO: [&str; 18] = [
    "sha256",
    "sha512",
    "hmac_sha256",
    "equal",
    "random_bytes",
    "token",
    "hash_password",
    "check_password",
    "ed25519_secret",
    "ed25519_public",
    "ed25519_sign",
    "p256_secret",
    "p256_public",
    "p256_sign",
    "aes_key",
    "encrypt",
    "decrypt",
    "sha384",
];
