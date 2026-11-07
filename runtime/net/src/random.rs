//! Random bits for `std.random` (spec 14.15): from the operating
//! system's cryptographically secure generator, through the `ring`
//! provider TLS already uses (decision 0021), so no new dependency. In
//! this crate so a program without random numbers or networking doesn't
//! link it (decision 0024).

/// 64 random bits.
pub fn bits() -> u64 {
    let mut b = [0u8; 8];
    // The system generator fails only when the system has no randomness
    // to give, and nothing random can be made then.
    rustls::crypto::ring::default_provider()
        .secure_random
        .fill(&mut b)
        .expect("the operating system gave no random bytes");
    u64::from_le_bytes(b)
}

#[cfg(test)]
mod tests {
    use super::bits;

    #[test]
    fn draws_differ() {
        // 64 equal bits twice in a row would happen once in 2^64 runs.
        assert_ne!(bits(), bits());
    }
}
