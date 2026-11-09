//! Time zones for `std.time` (spec 14.4, decision 0064): a zone's offset
//! from UTC at a moment, from the IANA database through `jiff`. The rest
//! of zones is written in Kavach (`stdlib/time/time.kav`).

/// How many seconds zone `name` is ahead of UTC at the moment `ms`
/// (milliseconds since 1970-01-01 UTC), or `None` for a name the database
/// doesn't have. A moment past the years jiff knows (about ±9999) is
/// taken at the nearest one it does.
pub fn offset_seconds(name: &str, ms: i64) -> Option<i64> {
    let zone = jiff::tz::TimeZone::get(name).ok()?;
    let at = jiff::Timestamp::from_millisecond(ms).unwrap_or(match ms < 0 {
        true => jiff::Timestamp::MIN,
        false => jiff::Timestamp::MAX,
    });
    Some(i64::from(zone.to_offset(at).seconds()))
}

#[cfg(test)]
mod tests {
    use super::offset_seconds;

    #[test]
    fn offsets_follow_the_zone_and_daylight_saving_time() {
        assert_eq!(offset_seconds("UTC", 0), Some(0));
        assert_eq!(offset_seconds("Asia/Kolkata", 0), Some(19_800));
        // New York: winter (-5 h), then summer (-4 h), in 2026.
        assert_eq!(
            offset_seconds("America/New_York", 1_767_225_600_000),
            Some(-18_000)
        );
        assert_eq!(
            offset_seconds("America/New_York", 1_783_000_000_000),
            Some(-14_400)
        );
        assert_eq!(offset_seconds("Not/A_Zone", 0), None);
        assert!(offset_seconds("Europe/Berlin", i64::MAX).is_some());
    }
}
