//! TLS servers and mutual TLS (spec 14.10): a Kavach server proves who it
//! is with its certificate and asks each client for one; a client trusts
//! a private certificate authority (`roots`) and shows its own. The
//! certificates are made for the test with `openssl`, so no private key
//! is kept in the repository. The server reads the names on the client's
//! certificate (decision 0099).

use std::path::Path;
use std::process::Command;

const PROGRAM: &str = "use std.cli
use std.fs
use std.net

fn serve(listener: Listener) ! NetError uses Net {
    while true {
        const conn = try listener.accept()
        const got = try conn.read()
        // Whose certificate it was (decision 0099).
        const who = if let c = conn.client_cert() {
            \"{c.common_name} {c.dns} {c.uris}\"
        } else {
            \"nobody\"
        }
        try conn.write(\"hello, {text_of(got)} from {who}\".bytes())
        conn.close()
    }
}

fn text_of(data: Bytes?) -> String {
    if let d = data { d.text() ?? \"?\" } else { \"-\" }
}

fn main() ! Error {
    const dir = cli.args()[0]
    const ca = try fs.read_text(\"{dir}/ca.pem\")
    const cert = try fs.read_text(\"{dir}/server.pem\")
    const key = try fs.read_text(\"{dir}/server.key\")
    const listener = try net.listen_tls(\"127.0.0.1:0\", cert, key, protocols: [\"kv\"], clients: ca)
    const server = spawn serve(listener)
    const address = listener.address()
    // A client that never starts its handshake holds up no one.
    const silent = try net.connect(address)

    // A client the server trusts, which trusts the server.
    const mine = try fs.read_text(\"{dir}/client.pem\")
    const my_key = try fs.read_text(\"{dir}/client.key\")
    const conn = try net.connect_tls(address, [\"kv\"], roots: ca, cert: mine, key: my_key)
    try conn.write(\"client\".bytes())
    const answer = try conn.read()
    print(text_of(answer))
    print(conn.protocol() ?? \"-\")
    // The server's certificate, for channel binding (spec 14.10).
    print((conn.peer_certificate() ?? Bytes([])).len() > 100)
    conn.close()

    // Without a certificate, the server refuses the client: while
    // connecting, or at the first read.
    const refused = match net.connect_tls(address, roots: ca) {
        ok(a) => {
            a.write(\"me\".bytes()) catch e => {}
            const reply = a.read() catch e => none
            reply.is_none()
        }
        err(e) => true
    }
    print(if refused { \"refused\" } else { \"served!\" })

    // Without the private authority, the client refuses the server.
    const trusted = match net.connect_tls(address, cert: mine, key: my_key) {
        ok(c) => true
        err(e) => false
    }
    print(if trusted { \"trusted!\" } else { \"server not trusted\" })
    silent.close()
}
";

/// The test's own `openssl` settings, so any `openssl` on the PATH works,
/// even one whose own settings file isn't where it looks (Anaconda's).
const OPENSSL_CONF: &str = "[req]
distinguished_name = dn
x509_extensions = ca
[dn]
[ca]
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer
basicConstraints = critical,CA:true
";

fn openssl(dir: &Path, args: &[&str]) {
    let conf = dir.join("openssl.cnf");
    std::fs::write(&conf, OPENSSL_CONF).unwrap();
    let out = Command::new("openssl")
        .args(args)
        .env("OPENSSL_CONF", &conf)
        .current_dir(dir)
        .output()
        .expect("openssl, to make the test's certificates");
    assert!(
        out.status.success(),
        "openssl {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A certificate authority, and a server and a client certificate it
/// signed.
fn certificates(dir: &Path) {
    let ec = [
        "-newkey",
        "ec",
        "-pkeyopt",
        "ec_paramgen_curve:P-256",
        "-nodes",
    ];
    let mut ca = vec!["req", "-x509", "-days", "1", "-subj", "/CN=Kavach Test CA"];
    ca.extend(ec);
    ca.extend(["-keyout", "ca.key", "-out", "ca.pem"]);
    openssl(dir, &ca);
    for (name, usage, alt) in [
        ("server", "serverAuth", "subjectAltName=IP:127.0.0.1\n"),
        (
            "client",
            "clientAuth",
            "subjectAltName=DNS:orders.shop,URI:spiffe://shop/orders
",
        ),
    ] {
        let ext = format!("basicConstraints=CA:FALSE\nextendedKeyUsage={usage}\n{alt}");
        std::fs::write(dir.join(format!("{name}.ext")), ext).unwrap();
        let (key, csr) = (format!("{name}.key"), format!("{name}.csr"));
        let subject = format!("/CN=kavach-{name}");
        let mut req = vec!["req", "-subj", subject.as_str()];
        req.extend(ec);
        req.extend(["-keyout", key.as_str(), "-out", csr.as_str()]);
        openssl(dir, &req);
        let (pem, ext) = (format!("{name}.pem"), format!("{name}.ext"));
        openssl(
            dir,
            &[
                "x509",
                "-req",
                "-in",
                &csr,
                "-CA",
                "ca.pem",
                "-CAkey",
                "ca.key",
                "-CAcreateserial",
                "-days",
                "1",
                "-out",
                &pem,
                "-extfile",
                &ext,
            ],
        );
    }
}

#[test]
fn servers_and_clients_prove_who_they_are() {
    let dir = std::env::temp_dir().join(format!("kavach-tls-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    certificates(&dir);
    let file = dir.join("tls.kav");
    std::fs::write(&file, PROGRAM).unwrap();
    let expected = "hello, client from kavach-client [\"orders.shop\"] [\"spiffe://shop/orders\"]\nkv\ntrue\nrefused\nserver not trusted\n";
    for native in [false, true] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let started = std::time::Instant::now();
        let out = cmd.arg(&file).arg(&dir).output().unwrap();
        // A handshake that blocked would wait for the silent client's
        // 30-second deadline.
        let took = started.elapsed();
        assert!(took.as_secs() < 15, "native: {native}: {took:?}");
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            expected,
            "native: {native}\n{said}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
