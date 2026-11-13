# 14.4 std.time

| Function | Gives |
|---|---|
| `time.now_ms() -> Int` | Milliseconds since 1970-01-01 UTC |
| `time.monotonic_ms() -> Int` | Milliseconds from a clock that never goes back; use it to measure durations |
| `time.sleep_ms(ms)` | Waits `ms` milliseconds |
| `time.sleep(d)` | Waits for the `Duration` `d` |
| `time.every_ms(ms)` | An interval: a `for` loop over it runs a round every `ms` milliseconds, counting `0, 1, 2, …`, until the loop ends with `break` or `return` |
| `time.every(d)` | The same, every `Duration` `d` |
| `time.after(d, f)` | A `Task` (9.1) that waits for `d` and then calls `f`; like any task, it belongs to the function that called `time.after`, and is cancelled when that function returns (9.2) |

```kavach
use std.time

for tick in time.every_ms(1_000) {
    print("tick {tick}")
    if tick == 4 {
        break
    }
}
```

The first round starts `ms` after the loop begins, and each later one `ms` after the one before started; if a round takes longer, the next one starts at once (rounds are never skipped or run twice to catch up).

A **`Duration`** is a length of time: a record with one field, `ms`, its milliseconds. A whole number followed by a unit makes one:

| Written | Is |
|---|---|
| `n.ms` | `n` milliseconds |
| `n.seconds` | `n * 1_000` ms |
| `n.minutes` | `n * 60_000` ms |
| `n.hours` | `n * 3_600_000` ms |
| `n.days` | `n * 86_400_000` ms |

```kavach
use std.time

time.sleep(200.ms)
const backup = 2.days
print(backup.ms)          // 172800000
const page = try fetch.wait().timeout(5.seconds)   // 9.6
```

`n` is any `Int` expression (`retries.seconds`); one too large for the unit overflows like any `Int` arithmetic. Any other name after a whole number is a compile error that lists these units: there is no `2.months` or `1.years`, since months and years have no fixed length. Every program has `Duration`, like `Json`.

## Dates and times of day

Reading the clock (`now_ms`, `monotonic_ms`, `today`, `now`) needs no capability: it changes nothing outside the program, and under `kavach test` the clock is faked (11.5). Waiting (`sleep`, `every`, `after`) needs `Clock` (8.1). A contract may not read the clock, since its answer would change from run to run ([decision 0058](../../decisions/0058-dates-in-std-time.md)).

| Type | Is |
|---|---|
| `Date` | A day of the calendar, with no time and no zone: `year`, `month` (1 to 12), `day` (1 to the month's last) |
| `Time` | A time of day, with no date and no zone: `hour` (0 to 23), `minute`, `second`, `ms` |
| `DateTime` | A `date` and a `time`, with no zone: what a calendar and a wall clock show |
| `Weekday` | `Monday` to `Sunday` |

A **moment** is a number of milliseconds since 1970-01-01 UTC, as `time.now_ms()` gives it. A `DateTime` is not a moment: it has no zone, so the two can't be mixed by accident. `time.utc(ms)` gives the `DateTime` a moment is in UTC, and `dt.utc_ms()` the moment back.

| Function or method | Gives |
|---|---|
| `time.today()`, `time.now()` | Today's `Date`, and the `DateTime` now, in UTC |
| `time.utc(ms)`, `dt.utc_ms()` | A moment as a `DateTime` in UTC, and back |
| `time.parse_date(text)` | The `Date` of `2026-11-04`; fails with `Invalid(text)` for anything else, an impossible date (`2026-02-30`) too |
| `time.is_leap_year(year)`, `time.days_in_month(year, month)` | As their names say |
| `d.weekday()` | Its `Weekday` |
| `d.add_days(n)`, `d.add_months(n)` | The date `n` days or months later (earlier for a negative `n`); a month too short for the day gives its last day: January 31 and one month is February 28 or 29 |
| `d.days_until(other)` | Days from `d` to `other`, negative when `other` comes first |
| `d.iso()`, `t.iso()`, `dt.iso()` | `2026-11-04`, `09:05:30` (with `.250` when there are milliseconds), `2026-11-04T09:05:30` |

```kavach
use std.time

const due = time.Date(year: 2024, month: 1, day: 31)
print(due.add_months(1).iso())       // 2024-02-29
print(due.weekday())                 // Wednesday
print(time.utc(0).iso())             // 1970-01-01T00:00:00
const day = try time.parse_date("2026-11-04")
print(time.today().days_until(day))
```

The rules that keep the usual date mistakes out:

- Months are 1 to 12, never from 0.
- The calendar is the Gregorian one for every year, with no exceptions: 1900 is not a leap year, 2000 is.
- A `Date` that can't exist can't be made: `time.Date(year: 2026, month: 2, day: 30)` breaks its invariant (7.3), and `parse_date` fails.
- Durations are measured with `monotonic_ms`, never from two readings of the wall clock, which can jump.

Waits (`sleep`, `every`, `after`, timeouts) and `monotonic_ms` use the operating system's clock that never jumps, as Rust, Go, Java and Python do: changing the clock or the time zone doesn't move them. Whether time the machine slept counts is the system's own rule: it doesn't on Linux and macOS, and it does on Windows ([decision 0062](../../decisions/0062-crashes-restarts-and-clocks.md)). In a durable agent handler a wait carries on across a restart (8.5).

There is no wait until a time of day and no cron schedule, as in the standard libraries of most languages: a job at 09:00 every day belongs to the operating system's scheduler (cron, systemd timers, Task Scheduler), which starts the program.

## Time zones

A **`Zone`** is a named time zone of the IANA database (`Asia/Kolkata`, `Europe/Berlin`, `America/New_York`, `UTC`), or a fixed offset from UTC (`+05:30`, `-08:00`), which never changes: what the clocks there show at each moment, daylight saving time included. The rules of a named zone come from the operating system where it has them (Linux, macOS), and otherwise from a copy built into the program (Windows) ([decision 0064](../../decisions/0064-time-zones-through-jiff.md)). Like reading the clock, it needs no capability.

| Function or method | Gives |
|---|---|
| `time.zone(name)` | The `Zone` named `name`, or the fixed offset `+05:30`; fails with `UnknownZone(name)` for a name the database doesn't have and for an offset that can't be (`+24:00`, `+5:30`) |
| `z.at(ms)` | The `DateTime` the clocks in `z` show at the moment `ms` |
| `z.moment(dt)` | The moment the clocks in `z` show `dt`; a skipped or repeated time is resolved as below |
| `z.resolve(dt)` | Which moments show `dt`: `Exact(ms)`, `Skipped(earlier, later)` or `Repeated(earlier, later)`, a `Local` |
| `z.offset_seconds(ms)` | How far `z` is ahead of UTC at the moment `ms`, in seconds (negative west of Greenwich) |
| `z.iso(ms)` | The moment `ms` as the clocks in `z` show it, with the offset: `2026-11-04T14:35:30+05:30`, and `Z` for an offset of zero (see "Times as text") |
| `time.zone_offset(name, ms)` | The same for a named zone of the database, `none` for any other name: what the others are built on |

```kavach
use std.time

const india = try time.zone("Asia/Kolkata")
print(india.at(0).iso())                     // 1970-01-01T05:30:00
const new_york = try time.zone("America/New_York")
const meeting = time.DateTime(date: time.Date(year: 2026, month: 3, day: 8), time: time.Time(hour: 2, minute: 30, second: 0, ms: 0))
print(new_york.at(new_york.moment(meeting)).iso())   // 2026-03-08T03:30:00
```

When daylight saving time starts, the clocks skip an hour, so some times of day never happen there; when it ends, they go back, so some happen twice. `z.moment(dt)` resolves them as RFC 5545 (iCalendar), Java's `java.time`, JavaScript's `Temporal` and Rust's `jiff` do by default:

- A **skipped** time moves later by the length of the gap: 02:30 on the night clocks jump from 02:00 to 03:00 is 03:30.
- A **repeated** time is its earlier moment: 01:30 on the night clocks go back from 02:00 to 01:00 is the first 01:30, still on summer time.

A program that must choose otherwise matches on `z.resolve(dt)`: `Skipped(earlier, later)` gives the moments the time would be with the offset after and before the jump (`moment` gives `later`), and `Repeated(earlier, later)` both moments it happens.

Kavach never reads the machine's own time zone: a program names the zone it means, so it runs the same on every machine.


## Times as text

`time.parse_date` and three more read the formats `iso` writes. What each gives keeps its rule: a `Time` and a `DateTime` have no zone, so their text can't have one, and a moment needs a zone, so its text must.

| Function | Reads | Gives |
|---|---|---|
| `time.parse_time(text)` | `09:05`, `09:05:30`, `09:05:30.250` | A `Time` |
| `time.parse_datetime(text)` | `2026-11-04T09:05:30`, with a time as above | A `DateTime` |
| `time.parse_moment(text)` | `2026-11-04T09:05:30Z`, `2026-11-04T14:35:30+05:30` | A moment, in milliseconds |

Each fails with `Invalid(text)` for anything else.

```kavach
use std.time

const at = try time.parse_moment("2026-11-04T14:35:30+05:30")
print(at)                                    // 1793783130000
const berlin = try time.zone("Europe/Berlin")
print(berlin.iso(at))                        // 2026-11-04T10:05:30+01:00
print(try time.parse_time("09:05"))
const wall = try time.parse_datetime("2026-11-04T09:05:30")
print(wall.iso())                            // 2026-11-04T09:05:30
const none_of_it = time.parse_moment("2026-11-04T09:05:30") catch e => {
    print("{e}")                             // Invalid(text: "2026-11-04T09:05:30")
    return
}
```

- Numbers have exactly the digits shown: `9:05`, `+5:30` and `2026-11-4` fail, and so do signs where there should be digits (`2026-+1-04`). A year before 0 or after 9999 is written as `iso` writes it, with a sign or more digits, as ISO 8601's expanded years are: `-0005-01-01`, `12345-06-07` (or `+12345-06-07`).
- A fraction of a second has 1 to 9 digits, as other systems write it. Kavach keeps milliseconds, so digits past the third are dropped, not rounded: `.1239` is 123 ms.
- A leap second (`23:59:60`) fails: a `Time` has no such second, as in Java's `java.time`.
- An offset may have seconds (`+05:21:10`): some zones' oldest offsets do, and `z.iso` writes them, so `parse_moment` reads back any moment `z.iso` wrote.

In JSON, a `Date`, `Time` and `DateTime` are this text too, with a `DateTime` in UTC (spec [2.12](../02-types/12-json.md)).

Formats other than these are an open question.

## The clock in tests

A `test` block (11.8) reads the real clock, as a program does. To check code at a chosen moment, it freezes the clock, as Python's `freezegun`, Ruby's `Timecop` and Jest's fake timers do:

| Function | Does |
|---|---|
| `time.freeze(ms)` | From now on in this test, the clock is fake: `now_ms` gives `ms` (milliseconds since 1970-01-01 UTC), and `sleep` moves it on at once instead of waiting |

```kavach
use std.time

test "a trial ends after 30 days" {
    time.freeze(1_767_225_600_000)        // 2026-01-01 00:00 UTC
    const trial = start_trial()
    time.sleep(30.days)
    assert trial.ended()
}
```

Calling `time.freeze` again makes the clock jump, forward or back, as when a person changes the machine's clock. `monotonic_ms` doesn't jump: on a frozen clock it counts only what the test slept, as a real one counts only time that passed (decision 0062).

`time.freeze` is only for tests: anywhere but directly in a `test` block it is a compile error. Generated tests (11.5) need no call: their clock is already frozen, at 2026-01-01 00:00 UTC.
