//! A client certificate's names, for mutual TLS (spec 14.10, decision
//! 0099): the common name and the DNS and URI subject-alternative names of
//! the certificate the handshake already checked. Nothing here decides
//! trust; `rustls` did that.

use x509_cert::Certificate;
use x509_cert::der::asn1::ObjectIdentifier;
use x509_cert::der::{Decode, Encode};
use x509_cert::ext::pkix::SubjectAltName;
use x509_cert::ext::pkix::name::GeneralName;

/// What `conn.client_cert()` gives.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CertNames {
    pub common_name: String,
    pub dns: Vec<String>,
    pub uris: Vec<String>,
}

const COMMON_NAME: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.3");
const ALT_NAMES: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.17");

/// The names in certificate `der`, or `None` when it can't be read.
pub fn names(der: &[u8]) -> Option<CertNames> {
    let cert = Certificate::from_der(der).ok()?;
    let tbs = &cert.tbs_certificate;
    let mut out = CertNames::default();
    for rdn in tbs.subject.0.iter() {
        for atv in rdn.0.iter().filter(|a| a.oid == COMMON_NAME) {
            out.common_name = text_of(&atv.value)?;
        }
    }
    let extensions = tbs.extensions.iter().flatten();
    for ext in extensions.filter(|e| e.extn_id == ALT_NAMES) {
        let alt = SubjectAltName::from_der(ext.extn_value.as_bytes()).ok()?;
        for name in alt.0 {
            match name {
                GeneralName::DnsName(d) => out.dns.push(d.to_string()),
                GeneralName::UniformResourceIdentifier(u) => out.uris.push(u.to_string()),
                _ => {}
            }
        }
    }
    Some(out)
}

/// A name's text, whichever string type the certificate wrote it in.
fn text_of(value: &x509_cert::der::Any) -> Option<String> {
    use x509_cert::der::asn1::{Ia5StringRef, PrintableStringRef, Utf8StringRef};
    let bytes = value.to_der().ok()?;
    if let Ok(s) = Utf8StringRef::from_der(&bytes) {
        return Some(s.as_str().to_string());
    }
    if let Ok(s) = PrintableStringRef::from_der(&bytes) {
        return Some(s.as_str().to_string());
    }
    Ia5StringRef::from_der(&bytes)
        .ok()
        .map(|s| s.as_str().to_string())
}
