# 0064. Time zones through `jiff`, with the operating system's rules

- **Status:** Accepted (Mithlesh's choice, 2026-11-04)
- **Date:** 2026-11-04
- **Changes:** spec 14.4; `runtime/core` gains a dependency

## Context

`std.time` had dates and times of day in UTC only (decision 0058). Named zones (`Asia/Kolkata`) need the IANA time zone database: when each zone's offset changes, daylight saving time included. Its data changes several times a year, as governments change their rules.

## Decision

- **The zone rules come through the `jiff` crate.** It reads the operating system's copy of the database where there is one (Linux and macOS keep it current). On Windows, whose own zones have other names and rules, it uses an IANA copy built into the program (`jiff`'s `tzdb-bundle-platform`). Mithlesh chose this over always bundling the database and over the operating system's data only.
- **Only one primitive comes from the runtime**: `time.zone_offset(name, ms)`, a zone's offset from UTC at a moment. `Zone`, `at`, `moment` and `resolve` are written in Kavach in `std.time`, as dates are. In native code the primitive is a runtime entry of its own (decision 0024), so only a program that uses zones links `jiff` and the zone data.
- **Skipped and repeated times are resolved as RFC 5545 does**, which is the default of `java.time`, `Temporal` and `jiff`: a skipped time moves later by the gap, and a repeated one is its earlier moment. `z.resolve(dt)` gives both moments for a program that must choose otherwise. An earlier plan made every program handle these cases; Mithlesh asked to do as other languages do.
- **Kavach never reads the machine's own zone.** A program names the zone it means.

## Consequences

- A zone's rules on Linux and macOS are as current as the system's. On Windows they are as current as the Kavach that built the program.
- Two machines can disagree about a zone whose rules changed recently, if one of them isn't updated.
- A program that uses zones on Windows is about 200 KB larger.

## Alternatives considered

- **Always bundle the database.** Every machine would then give the same answer, but the rules would be only as current as the build.
- **The operating system's data only.** That is the smallest choice, but Windows names and rules differ and some machines have no data.
- **`chrono-tz`.** It always bundles its own copy, and has no reading of the system's data.
