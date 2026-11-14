//! Traces (spec 14.12, decision 0107): the ids of spans, and the W3C
//! `traceparent` header that carries them between services
//! (`00-<trace>-<span>-<flags>`). Ids must be unique, not secret, so
//! they come from a fast generator seeded from the process's random hash
//! keys.

use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// Where a span is: its trace, and its own id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanId {
    pub trace: u128,
    pub span: u64,
}

/// A new id, never 0 (W3C counts all zeros as no id).
fn new_u64() -> u64 {
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut seed = STATE.load(Ordering::Relaxed);
    if seed == 0 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(1, |d| d.as_nanos()),
        );
        seed = h.finish() | 1;
        STATE.store(seed, Ordering::Relaxed);
    }
    let mut state = STATE.fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed);
    crate::seeded::split_mix(&mut state).max(1)
}

/// A span under `parent`: in its trace, or in a new trace for none.
pub fn child_of(parent: Option<SpanId>) -> SpanId {
    let trace = match parent {
        Some(p) => p.trace,
        None => (u128::from(new_u64()) << 64) | u128::from(new_u64()),
    };
    SpanId {
        trace,
        span: new_u64(),
    }
}

/// The `traceparent` header for `id`, sampled.
pub fn traceparent(id: SpanId) -> String {
    format!("00-{:032x}-{:016x}-01", id.trace, id.span)
}

/// The span a `traceparent` header names, or `None` when it isn't one.
pub fn parse_traceparent(header: &str) -> Option<SpanId> {
    let parts: Vec<&str> = header.trim().split('-').collect();
    let [version, trace, span, flags] = parts.as_slice() else {
        return None;
    };
    let hex = |s: &str, len: usize| s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit());
    if !hex(version, 2) || *version == "ff" || !hex(trace, 32) || !hex(span, 16) || !hex(flags, 2) {
        return None;
    }
    let id = SpanId {
        trace: u128::from_str_radix(trace, 16).ok()?,
        span: u64::from_str_radix(span, 16).ok()?,
    };
    (id.trace != 0 && id.span != 0).then_some(id)
}

/// A span that ended, as the collector gets it.
#[derive(Clone, Debug)]
pub struct Finished {
    pub name: String,
    pub id: SpanId,
    pub parent: Option<u64>,
    pub start_nanos: u128,
    pub end_nanos: u128,
    /// The error's printed form, for a block that failed or panicked.
    pub error: Option<String>,
}

/// Nanoseconds since 1970, as OTLP counts time.
pub fn now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traceparent_goes_out_and_comes_back() {
        let root = child_of(None);
        let child = child_of(Some(root));
        assert_eq!(child.trace, root.trace);
        assert_ne!(child.span, root.span);
        let header = traceparent(child);
        assert_eq!(header.len(), 55);
        assert_eq!(parse_traceparent(&header), Some(child));
        let w3c = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let id = parse_traceparent(w3c).unwrap();
        assert_eq!(traceparent(id), w3c);
        for bad in [
            "",
            "00-00000000000000000000000000000000-00f067aa0ba902b7-01",
            "ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            "00-xyz-00f067aa0ba902b7-01",
        ] {
            assert_eq!(parse_traceparent(bad), None, "{bad}");
        }
    }
}
