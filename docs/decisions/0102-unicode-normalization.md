# 0102. Unicode normalisation in `std.text`, from `unicode-normalization`

- **Status:** Accepted
- **Date:** 2026-11-13

## Context

The same text can be written with different code points: `é` is one code point (U+00E9) or `e` followed by a combining accent (U+0065 U+0301), and `ﬁ` is a ligature of `f` and `i`. Unicode's normal forms (UAX #15) turn such texts into one spelling, so they compare equal. The PostgreSQL package needs NFKC for SASLprep (RFC 4013), which SCRAM authentication applies to passwords; without it, a password with such characters fails to authenticate. User names, search and identifiers need it too. Normalising takes Unicode's tables, which Kavach doesn't have. A maintainer approved adding a crate for it (2026-11-13).

## Decision

- **`unicode-normalization`** (pure Rust, no `unsafe` in its API, tables for the Unicode version it names), in `runtime/core`, so the interpreter and native code give the same text.
- Four functions in `std.text` (spec 14.9), one per normal form: `text.nfc(s)`, `text.nfd(s)`, `text.nfkc(s)`, `text.nfkd(s)`, each `-> String`. The runtime provides them, as it provides `math.sqrt`; the rest of `std.text` is written in Kavach.

## Consequences

- SCRAM drivers and any code comparing what people type can normalise first.
- A program that doesn't use them links none of the tables (decision 0024).
- The Unicode version follows the crate's; updating it can change a normalised text for newly added characters, as in every language.

## Alternatives considered

- **One function with the form as text, `text.normalize(s, "NFKC")`**, as Python and JavaScript have: a misspelled form would be found only when the program runs.
- **A union for the form (`Normalization.Nfkc`):** checked, but longer to write than the four names.
- **Tables written in Kavach:** large, and they would have to follow each Unicode version by hand.
