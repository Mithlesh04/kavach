//! `std.time`'s `Date`, `Time` and `DateTime` in JSON (spec 2.12): ISO
//! 8601 text, as their `iso()` methods write it, the same in the
//! interpreter, native code and a type's schema. A `DateTime` in JSON is
//! in UTC: it is written with `Z`, and read with `Z`, an offset (turned
//! to UTC) or none.

/// Which of the three a record is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Date,
    Time,
    DateTime,
}

impl Kind {
    /// The kind of a record named `name` with `fields`, in order: the
    /// fields of `std.time`'s type of that name.
    pub fn of<'s>(name: &str, fields: impl IntoIterator<Item = &'s str>) -> Option<Kind> {
        let (kind, want): (Kind, &[&str]) = match name {
            "Date" => (Kind::Date, &["year", "month", "day"]),
            "Time" => (Kind::Time, &["hour", "minute", "second", "ms"]),
            "DateTime" => (Kind::DateTime, &["date", "time"]),
            _ => return None,
        };
        fields.into_iter().eq(want.iter().copied()).then_some(kind)
    }

    /// What decoding wants (a `JsonProblem`'s `expected`).
    pub fn expected(self) -> &'static str {
        match self {
            Kind::Date => "a date such as `2024-01-02`",
            Kind::Time => "a time of day such as `03:04:05`",
            Kind::DateTime => "a date and time such as `2024-01-02T03:04:05Z`",
        }
    }

    /// Its schema's part after `"type": "string"`: a `format` where JSON
    /// Schema has one that fits, a pattern for a time of day (whose
    /// `format` would need an offset).
    pub fn schema(self) -> (&'static str, &'static str) {
        match self {
            Kind::Date => ("format", "date"),
            Kind::Time => ("pattern", r"^[0-9]{2}:[0-9]{2}(:[0-9]{2}(\.[0-9]{1,9})?)?$"),
            Kind::DateTime => ("format", "date-time"),
        }
    }
}

fn padded(out: &mut String, n: i64, width: usize) {
    if n < 0 {
        out.push('-');
    }
    out.push_str(&format!("{:0width$}", n.unsigned_abs()));
}

fn time_text(out: &mut String, n: &[i64]) {
    padded(out, n[0], 2);
    out.push(':');
    padded(out, n[1], 2);
    out.push(':');
    padded(out, n[2], 2);
    if n[3] != 0 {
        out.push('.');
        padded(out, n[3], 3);
    }
}

/// The text for `n`, the kind's numbers in order: year, month and day,
/// then hour, minute, second and milliseconds.
pub fn write(kind: Kind, n: &[i64]) -> String {
    let mut out = String::new();
    if kind != Kind::Time {
        padded(&mut out, n[0], 4);
        out.push('-');
        padded(&mut out, n[1], 2);
        out.push('-');
        padded(&mut out, n[2], 2);
    }
    match kind {
        Kind::Date => {}
        Kind::Time => time_text(&mut out, n),
        Kind::DateTime => {
            out.push('T');
            time_text(&mut out, &n[3..]);
            out.push('Z');
        }
    }
    out
}

/// The whole number `text` writes in exactly `width` digits.
fn digits(text: &str, width: usize) -> Option<i64> {
    let ok = text.len() == width && text.bytes().all(|b| b.is_ascii_digit());
    if ok { text.parse().ok() } else { None }
}

fn is_leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        2 if is_leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `2026-11-04`, as `time.parse_date` reads it.
fn date(text: &str) -> Option<[i64; 3]> {
    let mut parts = text.split('-');
    let y = digits(parts.next()?, 4)?;
    let m = digits(parts.next()?, 2)?;
    let d = digits(parts.next()?, 2)?;
    let ok = parts.next().is_none() && (1..=12).contains(&m) && d >= 1;
    (ok && d <= days_in_month(y, m)).then_some([y, m, d])
}

/// `09:05`, `09:05:30` or `09:05:30.250`, as `time.parse_time` reads it:
/// a fraction of 1 to 9 digits, of which the first three count.
fn time(text: &str) -> Option<[i64; 4]> {
    let (clock, fraction) = match text.split_once('.') {
        Some((c, f)) => (c, Some(f)),
        None => (text, None),
    };
    let parts: Vec<&str> = clock.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let h = digits(parts[0], 2)?;
    let mi = digits(parts[1], 2)?;
    let s = match parts.get(2) {
        Some(p) => digits(p, 2)?,
        None => 0,
    };
    if h > 23 || mi > 59 || s > 59 {
        return None;
    }
    let ms = match fraction {
        None => 0,
        Some(f) if parts.len() == 3 && (1..=9).contains(&f.len()) => {
            digits(f, f.len())?;
            digits(&format!("{f}00")[..3], 3)?
        }
        Some(_) => return None,
    };
    Some([h, mi, s, ms])
}

/// How many seconds ahead of UTC `+05:30` (or `-08:00:00`) is.
fn offset(text: &str) -> Option<i64> {
    let sign = match text.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let parts: Vec<&str> = text[1..].split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let h = digits(parts[0], 2)?;
    let m = digits(parts[1], 2)?;
    let s = match parts.get(2) {
        Some(p) => digits(p, 2)?,
        None => 0,
    };
    (h <= 23 && m <= 59 && s <= 59).then_some(sign * (h * 3_600 + m * 60 + s))
}

/// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// The date `days` after 1970-01-01.
fn civil_from_days(days: i64) -> [i64; 3] {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    [yoe + era * 400 + i64::from(m <= 2), m, d]
}

/// The kind's numbers in order, from the text JSON holds; `None` for
/// anything else (`infinity` too).
pub fn read(kind: Kind, text: &str) -> Option<Vec<i64>> {
    match kind {
        Kind::Date => date(text).map(|d| d.to_vec()),
        Kind::Time => time(text).map(|t| t.to_vec()),
        Kind::DateTime => {
            let (day, clock) = text.split_once('T')?;
            let d = date(day)?;
            let (clock, east) = match clock.strip_suffix('Z') {
                Some(c) => (c, 0),
                None => match clock.rfind(['+', '-']) {
                    Some(i) => (&clock[..i], offset(&clock[i..])?),
                    None => (clock, 0),
                },
            };
            let t = time(clock)?;
            let seconds =
                days_from_civil(d[0], d[1], d[2]) * 86_400 + t[0] * 3_600 + t[1] * 60 + t[2] - east;
            let [y, m, dd] = civil_from_days(seconds.div_euclid(86_400));
            let s = seconds.rem_euclid(86_400);
            Some(vec![y, m, dd, s / 3_600, s / 60 % 60, s % 60, t[3]])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_read_and_write() {
        let cases: &[(Kind, &str, Option<&[i64]>)] = &[
            (Kind::Date, "2024-02-29", Some(&[2024, 2, 29])),
            (Kind::Date, "2023-02-29", None),
            (Kind::Date, "infinity", None),
            (Kind::Time, "03:04:05.123456", Some(&[3, 4, 5, 123])),
            (Kind::Time, "03:04", Some(&[3, 4, 0, 0])),
            (Kind::Time, "24:00:00", None),
            (
                Kind::DateTime,
                "2024-01-02T03:04:05.250Z",
                Some(&[2024, 1, 2, 3, 4, 5, 250]),
            ),
            (
                Kind::DateTime,
                "2024-01-02T03:04:05",
                Some(&[2024, 1, 2, 3, 4, 5, 0]),
            ),
            (
                Kind::DateTime,
                "2024-01-01T02:00:00+05:30",
                Some(&[2023, 12, 31, 20, 30, 0, 0]),
            ),
            (
                Kind::DateTime,
                "2024-12-31T20:00:00-08:00",
                Some(&[2025, 1, 1, 4, 0, 0, 0]),
            ),
            (Kind::DateTime, "2024-01-02", None),
            (Kind::DateTime, "-infinity", None),
        ];
        for (kind, text, want) in cases {
            assert_eq!(read(*kind, text).as_deref(), *want, "{text}");
        }
        assert_eq!(write(Kind::Date, &[2026, 11, 4]), "2026-11-04");
        assert_eq!(write(Kind::Time, &[9, 5, 30, 0]), "09:05:30");
        assert_eq!(write(Kind::Time, &[23, 0, 0, 250]), "23:00:00.250");
        let dt = [2026, 1, 2, 3, 4, 5, 6];
        assert_eq!(write(Kind::DateTime, &dt), "2026-01-02T03:04:05.006Z");
        assert_eq!(
            read(Kind::DateTime, &write(Kind::DateTime, &dt)).unwrap(),
            dt
        );
    }

    #[test]
    fn only_the_fields_of_std_time_count() {
        assert_eq!(Kind::of("Date", ["year", "month", "day"]), Some(Kind::Date));
        assert_eq!(Kind::of("Date", ["year", "month"]), None);
        assert_eq!(Kind::of("Point", ["x", "y"]), None);
    }
}
