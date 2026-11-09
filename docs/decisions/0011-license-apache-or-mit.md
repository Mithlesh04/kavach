# 0011. License: Apache-2.0 OR MIT

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

The repository is public. Without a license, nobody may legally use, copy or change the code, so no company can adopt Kavach and no one can contribute safely. A language also needs a license that allows programs written in it, and its standard library, to be used in closed-source products.

## Decision

Kavach can be used under **Apache-2.0 or MIT**, and each user chooses which. This covers the whole repository: compiler, runtime, standard library, tools, docs and examples.

- License texts: `LICENSE-APACHE` and `LICENSE-MIT` at the repository root.
- Contributions are accepted under the same terms (see `CONTRIBUTING.md`). No separate contributor agreement is needed.

## Consequences

- Companies can use Kavach in commercial and closed-source products.
- Apache-2.0 gives users an explicit patent grant, which legal teams look for.
- MIT keeps Kavach compatible with GPLv2 projects, which can't use Apache-2.0 code.
- Anyone can fork Kavach, including commercially. Protection of the name comes from a trademark policy, not the license (still open).

## Alternatives considered

- **MIT only:** simple, but no patent grant.
- **Apache-2.0 only:** patent grant, but incompatible with GPLv2.
- **GPL or another copyleft license:** would stop many companies from adopting the language and its standard library.
- **Source-available licenses (BSL, SSPL):** not open source; would block adoption and outside contributions.
