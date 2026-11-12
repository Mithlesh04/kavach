//! What the runtime gives a WebSocket handshake (spec 14.11, decision
//! 0092): the server's accept value, SHA-1 of the client's key and RFC
//! 6455's fixed text, in base64; a client's random key; and the random
//! mask a client puts on each frame. SHA-1 comes from `ring`, which TLS
//! already uses (decision 0021); randomness from the system, as for
//! `std.random`. The framing itself is Kavach, in `std/http/websocket.kav`.

/// The fixed text RFC 6455 joins to the client's key (section 1.3).
const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// `sec-websocket-accept` for a client's `sec-websocket-key`.
pub fn accept(key: &str) -> String {
    let mut ctx = ring::digest::Context::new(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY);
    ctx.update(key.trim().as_bytes());
    ctx.update(GUID.as_bytes());
    base64(ctx.finish().as_ref())
}

fn random(out: &mut [u8]) {
    rustls::crypto::ring::default_provider()
        .secure_random
        .fill(out)
        .expect("the operating system gave no random bytes");
}

/// A client's `sec-websocket-key`: 16 random bytes, in base64.
pub fn key() -> String {
    let mut b = [0u8; 16];
    random(&mut b);
    base64(&b)
}

/// The four random bytes a client masks a frame with.
pub fn mask() -> [u8; 4] {
    let mut b = [0u8; 4];
    random(&mut b);
    b
}

/// `data` in standard base64, with `=` padding.
fn base64(data: &[u8]) -> String {
    const DIGITS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(DIGITS[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_matches_rfc_6455() {
        // The example of RFC 6455, section 1.3.
        assert_eq!(
            accept("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(key().len(), 24);
        assert_ne!(mask(), mask());
    }
}
