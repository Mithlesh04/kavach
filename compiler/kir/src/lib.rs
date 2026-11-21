// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! KIR, the Kavach IR: a small, typed, structured form of a checked
//! program. Code generators read KIR, not the syntax tree.
//!
//! Lowering supports a growing subset of the language. Anything outside it
//! is reported as [`Unsupported`], never compiled wrongly; the interpreter
//! (decision 0014) still runs such programs.

pub mod borrows;
pub mod c_abi;
mod cancel;
mod checked_fits;
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
mod inout_stash;
mod invariant_checks;
mod ir;
mod jam;
mod library;
mod list_layout;
mod loop_fields;
mod loop_lens;
mod lower;
mod lower_agents;
mod lower_ai;
mod lower_arena;
mod lower_assert;
mod lower_base64;
mod lower_body;
mod lower_bytes;
mod lower_bytes_text;
mod lower_c;
mod lower_c_records;
mod lower_calls;
mod lower_chain;
mod lower_cli;
mod lower_closure;
mod lower_concat;
mod lower_consts;
mod lower_convert;
mod lower_data;
mod lower_destructure;
mod lower_dispatch;
mod lower_display;
mod lower_draw;
mod lower_drops;
mod lower_durable;
mod lower_enums;
mod lower_errors;
mod lower_events;
mod lower_every;
mod lower_expr;
mod lower_ext;
mod lower_fn;
mod lower_fn_errors;
mod lower_frozen;
mod lower_generic;
mod lower_gpu;
mod lower_gpu_choice;
mod lower_guard;
mod lower_if;
mod lower_iface;
mod lower_inout;
mod lower_json;
mod lower_json_checks;
mod lower_labels;
mod lower_lambda;
mod lower_loaded;
mod lower_log;
mod lower_match;
mod lower_methods;
mod lower_numbers;
mod lower_ops;
mod lower_optional;
mod lower_power;
mod lower_process;
mod lower_ptr;
mod lower_ptr_syntax;
mod lower_range;
mod lower_reflect;
mod lower_replay;
mod lower_round;
mod lower_schema;
mod lower_scoped;
mod lower_selected;
mod lower_set_loops;
mod lower_sizes;
mod lower_slices;
mod lower_statics;
mod lower_std;
mod lower_stmt;
mod lower_tagged;
mod lower_tasks;
mod lower_text;
mod lower_threads;
mod lower_tls;
mod lower_tuple_assign;
mod lower_views;
mod lower_whole;
mod moves;
mod par;
mod passes;
mod prefetch;
mod prune;
mod ranges;
mod rt_op;
mod sroa;
mod std_ops;
mod stop_points;
mod tuple_lets;
mod types;
mod unrecurse;
mod unused;
mod version;
pub mod walk;
mod whole_lines;
mod written_types;

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

/// The system libraries a program links for the runtime parts it uses,
/// beyond those every program links: on Linux, `libudev` for gamepads
/// (decision 0113) and `libasound` for sound (decision 0114), named so the
/// linker takes the file every system with them has, not the development
/// package's `.so`.
pub fn system_libraries(program: &Program) -> Vec<String> {
    // Tracy's client is C++ (decision 0123): its standard library, which
    // Windows' linker finds by itself.
    let cxx = match () {
        _ if !program.tracy || cfg!(windows) => None,
        _ if cfg!(target_os = "macos") => Some("c++".to_string()),
        _ => Some("stdc++".to_string()),
    };
    if !cfg!(target_os = "linux") {
        return cxx.into_iter().collect();
    }
    let uses = |prefix: &str| {
        (program.functions.iter()).any(|f| {
            walk::any_in_block(&f.body, &mut |e| {
                matches!(&e.kind, ExprKind::Rt(op, _) if format!("{op:?}").starts_with(prefix))
            })
        })
    };
    let mut libs: Vec<String> = cxx.into_iter().collect();
    if uses("Input") {
        libs.push(":libudev.so.1".to_string());
    }
    if uses("Audio") {
        libs.push(":libasound.so.2".to_string());
    }
    libs
}
