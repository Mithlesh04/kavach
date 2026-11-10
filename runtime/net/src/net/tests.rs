use std::net::TcpStream;
use std::time::{Duration, Instant};

/// Connections that come faster than a program accepts them wait in the
/// listener's queue (decision 0079). With Rust's 128 places, the system
/// drops those past it, and each tries again only after a second or more;
/// Windows refuses them. 180 fit in the system's longest queue (about 200
/// on a desktop Windows, thousands elsewhere).
#[test]
fn a_listener_queues_many_connections_before_accepting() {
    let (_listener, address) = super::bind_listener("127.0.0.1:0").expect("listens");
    let to = address.parse().expect("an address");
    let started = Instant::now();
    let open: Vec<TcpStream> = (0..180)
        .map(|i| {
            TcpStream::connect_timeout(&to, Duration::from_secs(5))
                .unwrap_or_else(|e| panic!("connection {i}: {e}"))
        })
        .collect();
    let took = started.elapsed();
    assert_eq!(open.len(), 180);
    assert!(
        took < Duration::from_millis(900),
        "took {took:?}: some tried again"
    );
}
