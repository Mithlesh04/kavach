# 0058. Dates in `std.time`, and reading the clock needs no capability

- **Status:** Accepted (Mithlesh's answer on the clock, 2026-11-04; the date types are for review)
- **Date:** 2026-11-04
- **Changes:** spec 8.1 (`Clock`), 14.1, 14.4

## Context

`std.time` had clocks and durations, but no dates (to-do item 6). Other languages' date libraries are known for the same traps ([classic-mistakes.md](../vision/classic-mistakes.md#time)): months from 0, the 1900 leap year, "local time" by default, and wall-clock differences used as durations. Mithlesh decided (2026-11-04) that every device needs a capability except the clock, which is free to read, and that tests and replayed agents can set a fixed or recorded time.

## Decision

1. **Reading the clock needs no capability.** `time.now_ms()`, `time.monotonic_ms()`, `time.today()` and `time.now()` work in any function. Waiting (`sleep`, `every`, `after`) still needs `Clock`. A contract may not read the clock, since its answer would change from run to run.
2. **Three types with no zone, and moments as milliseconds.** `Date` (year, month 1 to 12, day), `Time` (a time of day) and `DateTime` (both) are what a calendar and a wall clock show. A moment stays a number of milliseconds since 1970-01-01 UTC. `time.utc(ms)` and `dt.utc_ms()` go between them, so a time with no zone is never taken for a moment.
3. **The Gregorian calendar for every year, and impossible dates refused.** A `Date`'s invariant holds its month and day in range; `parse_date` fails on `2026-02-30`.
4. **Written in Kavach** (`stdlib/time/time.kav`), on the runtime's clock: no compiler or runtime change, so native code and the interpreter agree.

```kavach
use std.time

const due = time.Date(year: 2024, month: 1, day: 31)
print(due.add_months(1).iso())    // 2024-02-29
```

## Consequences

- Programs can read the date without a `uses` clause; the clock is faked under `kavach test`, so tests stay repeatable.
- Named time zones wait: they need the zone database, and their daylight saving gaps and overlaps must be values the program handles (an open question in spec 14).
- `add_months` clamps to the month's last day, as most libraries do; a program that wants an error instead checks `days_in_month` first.

## Alternatives considered

- **Keep `Clock` for reading too.** Every function that shows a date would need `uses Clock`, for a read that changes nothing. Mithlesh chose against it.
- **One `DateTime` with an optional zone.** It invites mixing a zone-less time with a moment, the mistake classic-mistakes.md lists.
- **Months from 0, or a `Month` union.** Numbers 1 to 12 read as people write dates; a union would make arithmetic on months longer.
