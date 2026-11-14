//! Spans as OTLP over HTTP with JSON (spec 14.12, decision 0107): the
//! body of `POST <endpoint>/v1/traces`, which `otlp.rs` sends from its
//! queue. Ids are hexadecimal, as OTLP's JSON form writes them.

use crate::otlp::{resource, text};
use kavach_runtime_core::json::{self, Tree};
use kavach_runtime_core::trace::Finished;

/// One span: ids, name, times, and an error status when its block failed.
fn span(s: &Finished) -> Tree {
    let mut fields = vec![
        ("traceId".to_string(), text(&format!("{:032x}", s.id.trace))),
        ("spanId".to_string(), text(&format!("{:016x}", s.id.span))),
    ];
    if let Some(parent) = s.parent {
        fields.push(("parentSpanId".into(), text(&format!("{parent:016x}"))));
    }
    // OTLP's status codes: 1 ok, 2 error.
    let status = match &s.error {
        Some(why) => Tree::Object(vec![
            ("code".into(), Tree::Int(2)),
            ("message".into(), text(why)),
        ]),
        None => Tree::Object(vec![("code".into(), Tree::Int(1))]),
    };
    fields.extend([
        ("name".to_string(), text(&s.name)),
        // 1: a span inside the program (`SPAN_KIND_INTERNAL`).
        ("kind".to_string(), Tree::Int(1)),
        (
            "startTimeUnixNano".to_string(),
            text(&s.start_nanos.to_string()),
        ),
        (
            "endTimeUnixNano".to_string(),
            text(&s.end_nanos.to_string()),
        ),
        ("status".to_string(), status),
    ]);
    Tree::Object(fields)
}

/// The request body for `spans`, as `service`.
pub(crate) fn request(service: &str, spans: &[Finished]) -> String {
    let scope = Tree::Object(vec![
        (
            "scope".into(),
            Tree::Object(vec![("name".into(), text("kavach"))]),
        ),
        (
            "spans".into(),
            Tree::Array(spans.iter().map(span).collect()),
        ),
    ]);
    let traces = Tree::Object(vec![
        ("resource".into(), resource(service)),
        ("scopeSpans".into(), Tree::Array(vec![scope])),
    ]);
    json::write(
        &Tree::Object(vec![("resourceSpans".into(), Tree::Array(vec![traces]))]),
        0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kavach_runtime_core::trace::SpanId;

    #[test]
    fn spans_are_otlp_json() {
        let s = Finished {
            name: "checkout".into(),
            id: SpanId {
                trace: 0xab,
                span: 0xcd,
            },
            parent: Some(0xef),
            start_nanos: 10,
            end_nanos: 20,
            error: Some("failed".into()),
        };
        let w = request("shop", &[s]);
        assert!(
            w.contains("\"traceId\":\"000000000000000000000000000000ab\""),
            "{w}"
        );
        assert!(w.contains("\"parentSpanId\":\"00000000000000ef\""), "{w}");
        assert!(w.contains("\"name\":\"checkout\""), "{w}");
        assert!(w.contains("\"code\":2"), "{w}");
        assert!(w.contains("\"service.name\""), "{w}");
    }
}
