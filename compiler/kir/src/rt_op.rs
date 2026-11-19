// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Built-in operations, carried out by the run-time library (`kv_rt`).

mod codes;
mod groups;

// `RtOp` and `RtOp::ALL`, made by `build.rs` from the op lists in
// `rt_op/ops/`, one file per family.
include!(concat!(env!("OUT_DIR"), "/rt_op.rs"));
