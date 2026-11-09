# 0006. Name: Kavach; extension: .kav

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

The name must be short, meaningful, pronounceable worldwide, available as a domain, and free of conflicts with other languages.

## Decision

- **Name:** Kavach (कवच, "armor"), which reflects the core idea: safety and protection.
- **Domain:** kavach.io
- **File extension:** `.kav`
- **CLI:** `kavach`, with the short alias `kv`

## Consequences

- A clear story: a language from India, built for safe software everywhere.
- `.kav` avoids `.kv`, which is already used by Kivy (Python UI) files.

## Before going public

- Trademark search for software (classes 9 and 42) in India, the US and the EU. "Kavach" is also the name of Indian Railways' train-protection system, which is a different class but should be checked.
- Reserve the GitHub organization and package names (crates.io, npm, PyPI) for the interop packages.

## Alternatives considered

Vow, Sutra and Keel were shortlisted. Kavach was chosen for its strong safety meaning and an available domain.
