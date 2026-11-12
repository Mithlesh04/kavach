//! The code of each op, which native code passes to the runtime.

use super::RtOp;

impl RtOp {
    /// A fingerprint of the codes (`RtOp::ALL`, made by `build.rs`): a
    /// compiler and a runtime library built from different lists of
    /// operations have different ones, and a program built by one and
    /// linked with the other stops at its start instead of calling the
    /// wrong operations (spec 10.5).
    pub const FINGERPRINT: i64 = {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut i = 0;
        while i < RtOp::ALL.len() {
            hash ^= RtOp::ALL[i] as u64;
            hash = hash.wrapping_mul(0x0100_0000_01b3);
            i += 1;
        }
        // A positive number, so it is one `Int` constant in the code.
        (hash >> 1) as i64
    };

    pub fn code(self) -> i64 {
        RtOp::ALL
            .iter()
            .position(|o| *o == self)
            .expect("a known op") as i64
    }
}
