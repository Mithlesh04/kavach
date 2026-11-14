//! Metrics as OTLP over HTTP with JSON (spec 14.12, decision 0107):
//! `log.count`, `log.gauge` and `log.histogram` are kept here, and a
//! thread of their own sends them every 10 seconds
//! (`POST <endpoint>/v1/metrics`). Sums and histograms count from the
//! first value (cumulative, as OpenTelemetry's default is).

use crate::otlp::{post, resource, text};
use kavach_runtime_core::json::{self, Tree};
use kavach_runtime_core::trace::now_nanos;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

/// How often metrics go out.
const EVERY: Duration = Duration::from_secs(10);

/// OpenTelemetry's default histogram bounds.
const BOUNDS: [f64; 15] = [
    0.0, 5.0, 10.0, 25.0, 50.0, 75.0, 100.0, 250.0, 500.0, 750.0, 1000.0, 2500.0, 5000.0, 7500.0,
    10000.0,
];

#[derive(Default)]
struct Histogram {
    count: u64,
    sum: f64,
    min: f64,
    max: f64,
    buckets: [u64; 16],
}

struct Metrics {
    url: String,
    service: String,
    start: u128,
    sums: BTreeMap<String, f64>,
    gauges: BTreeMap<String, (f64, u128)>,
    histograms: BTreeMap<String, Histogram>,
}

static METRICS: Mutex<Option<Metrics>> = Mutex::new(None);

/// Records a value for the collector at `endpoint`: kind 0 adds to a sum,
/// 1 sets a gauge, 2 adds to a histogram. The first one starts the
/// sending thread.
pub fn record(endpoint: &str, service: &str, kind: i64, name: &str, value: f64) {
    let mut all = METRICS.lock().unwrap_or_else(|e| e.into_inner());
    let m = all.get_or_insert_with(|| {
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(EVERY);
                send_now();
            }
        });
        Metrics {
            url: format!("{}/v1/metrics", endpoint.trim_end_matches('/')),
            service: service.to_string(),
            start: now_nanos(),
            sums: BTreeMap::new(),
            gauges: BTreeMap::new(),
            histograms: BTreeMap::new(),
        }
    });
    match kind {
        0 => *m.sums.entry(name.to_string()).or_default() += value,
        1 => {
            m.gauges.insert(name.to_string(), (value, now_nanos()));
        }
        _ => {
            let h = m.histograms.entry(name.to_string()).or_default();
            if h.count == 0 || value < h.min {
                h.min = value;
            }
            if h.count == 0 || value > h.max {
                h.max = value;
            }
            h.count += 1;
            h.sum += value;
            let bucket = BOUNDS.iter().take_while(|b| value > **b).count();
            h.buckets[bucket] += 1;
        }
    }
}

/// Sends the metrics now, if there are any (also at the program's end).
pub fn send_now() {
    let body = {
        let all = METRICS.lock().unwrap_or_else(|e| e.into_inner());
        let Some(m) = all.as_ref() else {
            return;
        };
        (m.url.clone(), request(m))
    };
    post(&body.0, &body.1);
}

fn point(m: &Metrics, now: u128, extra: Vec<(String, Tree)>) -> Tree {
    let mut fields = vec![
        ("startTimeUnixNano".to_string(), text(&m.start.to_string())),
        ("timeUnixNano".to_string(), text(&now.to_string())),
    ];
    fields.extend(extra);
    Tree::Object(fields)
}

fn metric(name: &str, kind: &str, body: Vec<(String, Tree)>) -> Tree {
    Tree::Object(vec![
        ("name".into(), text(name)),
        (kind.into(), Tree::Object(body)),
    ])
}

/// The request body for everything kept so far.
fn request(m: &Metrics) -> String {
    let now = now_nanos();
    let cumulative = || ("aggregationTemporality".to_string(), Tree::Int(2));
    let mut metrics = Vec::new();
    for (name, v) in &m.sums {
        let p = point(m, now, vec![("asDouble".into(), Tree::Float(*v))]);
        metrics.push(metric(
            name,
            "sum",
            vec![
                ("dataPoints".into(), Tree::Array(vec![p])),
                cumulative(),
                ("isMonotonic".into(), Tree::Bool(true)),
            ],
        ));
    }
    for (name, (v, at)) in &m.gauges {
        let p = Tree::Object(vec![
            ("timeUnixNano".into(), text(&at.to_string())),
            ("asDouble".into(), Tree::Float(*v)),
        ]);
        metrics.push(metric(
            name,
            "gauge",
            vec![("dataPoints".into(), Tree::Array(vec![p]))],
        ));
    }
    for (name, h) in &m.histograms {
        let p = point(
            m,
            now,
            vec![
                ("count".into(), text(&h.count.to_string())),
                ("sum".into(), Tree::Float(h.sum)),
                ("min".into(), Tree::Float(h.min)),
                ("max".into(), Tree::Float(h.max)),
                (
                    "bucketCounts".into(),
                    Tree::Array(h.buckets.iter().map(|c| text(&c.to_string())).collect()),
                ),
                (
                    "explicitBounds".into(),
                    Tree::Array(BOUNDS.iter().map(|b| Tree::Float(*b)).collect()),
                ),
            ],
        );
        metrics.push(metric(
            name,
            "histogram",
            vec![("dataPoints".into(), Tree::Array(vec![p])), cumulative()],
        ));
    }
    let scope = Tree::Object(vec![
        (
            "scope".into(),
            Tree::Object(vec![("name".into(), text("kavach"))]),
        ),
        ("metrics".into(), Tree::Array(metrics)),
    ]);
    let all = Tree::Object(vec![
        ("resource".into(), resource(&m.service)),
        ("scopeMetrics".into(), Tree::Array(vec![scope])),
    ]);
    json::write(
        &Tree::Object(vec![("resourceMetrics".into(), Tree::Array(vec![all]))]),
        0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_are_otlp_json() {
        let mut m = Metrics {
            url: String::new(),
            service: "shop".into(),
            start: 1,
            sums: BTreeMap::new(),
            gauges: BTreeMap::new(),
            histograms: BTreeMap::new(),
        };
        m.sums.insert("orders".into(), 3.0);
        m.gauges.insert("queue".into(), (7.0, 2));
        let mut h = Histogram::default();
        (h.count, h.sum, h.min, h.max) = (2, 30.0, 10.0, 20.0);
        h.buckets[2] = 1;
        h.buckets[3] = 1;
        m.histograms.insert("checkout_ms".into(), h);
        let w = request(&m);
        assert!(w.contains("\"name\":\"orders\",\"sum\""), "{w}");
        assert!(w.contains("\"isMonotonic\":true"), "{w}");
        assert!(w.contains("\"name\":\"queue\",\"gauge\""), "{w}");
        assert!(w.contains("\"count\":\"2\""), "{w}");
        assert!(w.contains("\"explicitBounds\":[0"), "{w}");
    }
}
