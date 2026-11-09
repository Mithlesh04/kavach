# Design Criteria Every New Language Must Pass

No body licenses or approves new programming languages. There are no mandatory official guidelines. ISO/Ecma standardization is something successful modern languages deliberately skip ([JTC1 SC22 overview](https://jtc1info.org/jtc1-subcommittees/sc-22/); [Ecma ECMA-408](https://ecma-international.org/publications-and-standards/standards/ecma-408/)).

What a language must "pass" instead is a bar set in practice. It comes from 50 years of design literature and from what shipped languages of the 2010s–20s proved. The bar is:

- **Design criteria:** simplicity, regularity and compile-time error detection (Hoare, Wirth, MacLennan, Sebesta).
- **A spec:** a versioned prose-plus-grammar spec with a conformance suite tied to it (Go, test262, WebAssembly).
- **Tooling every language must now have:** one binary with compiler, build tool, package manager, formatter and language server (Gleam v1's definition of "done").
- **Above all, an ecosystem story.** The strongest empirical result in the field is that social factors (existing code, expertise, libraries, interop) outweigh technical merit in adoption ([Meyerovich & Rabkin via ILNumerics](https://ilnumerics.net/blog/social-aspects-of-programming-languages/)).

Measured against this full bar, Kavach is stronger than usual on design discipline. It has a 14-chapter spec as source of truth, 38 decision records and 324 one-rule-per-file conformance tests. It also has a working LSP, dual interpreter/native backends, and self-published benchmarks beating C++ on its own suites.

It is weakest exactly where languages die: distribution and ecosystem. There is no deployed website, no playground, no Marketplace listing, no package manager and no macOS/ARM. There is a single maintainer, and an unverified name-clearance risk around "Kavach."

The right next moves do not include adding language features. They are:

- Shipping what exists (website, playground, VS Code extension).
- Adding stable spec rule IDs with a rules-without-tests checker.
- Writing a pre-1.0 stability policy and governance note.
- Clearing the trademark landscape.

## Design criteria

The design criteria have been stable for fifty years, and Kavach's priority order matches them.

The classic references agree on one main criterion: **simplicity backed by regularity and orthogonality**.

Hoare called simplicity the central requirement. Without it, "even the language designer himself cannot evaluate the consequences of his design decisions," he said. He also advised consolidating proven features rather than inventing new ones ([Hoare 1973 summary](https://softwareas.com/hints-on-programming-language-design-by-car-hoare-quick-summary/)).

Wirth described design as "the art of finding a compromise between many, often conflicting, criteria". He warned that "the combination of two seemingly harmless and well understood features may have disastrous effects". So every feature must be audited against all existing ones ([Wirth 1974](https://research.cs.queensu.ca/home/cordy/cisc860/Biblio/drb/DC/wirth74.pdf); [MIT reading summary](https://people.csail.mit.edu/feser/pld-s23/reading1.html)).

The second main idea is **compile-time error detection**:

- MacLennan's Security principle ("no program that violates the definition of the language... should escape detection"), with his Impossible Error and Defense in Depth principles ([MacLennan's principles, UTK](https://web.eecs.utk.edu/~bmaclenn/Classes/365/principles.html)).
- Sebesta's standard textbook rubric of **readability, writability, reliability and cost** as explicit trade-offs ([Sebesta ch.1](https://web.cs.ndsu.nodak.edu/~slator/html/CS372/sebesta-pdf/01.pdf)).

Steele adds the balancing point on growth. Design a small language planned to grow through its users, where user-defined additions look native ([Steele 1998](https://www.cs.virginia.edu/~evans/cs655/readings/steele.pdf)).

Designers working today state the same criteria in practical terms:

- Optimize for reading over writing. Zig: "no hidden control flow, no hidden memory allocations" ([Ziggit](https://ziggit.dev/t/what-does-zig-mean-by-no-hidden-control-flow/3618)).
- Keep the grammar simple enough for humans and tools: "better restrict your grammar to LL(1)" ([C3 lessons](https://c3-lang.org/blog/some-language-design-lessons-learned/)).
- Treat **error-message quality as part of the design**, not an afterthought. Elm's compiler errors were its main learning UI ([Packt on Elm 0.19.1](https://hub.packtpub.com/?p=30476)).
- Iterate semantics on paper and in real code before implementing, because "changing semantics after implementation forces painful compiler-wide changes" ([C3 lessons](https://c3-lang.org/blog/some-language-design-lessons-learned/)).

Some hard decisions are known to need settling early. The meaning of the language rests on them, and they are nearly irreversible. They are the memory model, the error model, the hidden-machinery budget, grammar class, C FFI, extension policy, and the compatibility promise ([C3](https://c3-lang.org/blog/some-language-design-lessons-learned/); [Graydon Hoare retrospective coverage](https://notes.billmill.org/link_blog/2023/06/The_rust_I_wanted_had_no_future.html); [pointersgonewild](https://pointersgonewild.com/2012/06/07/why-do-most-new-languages-fail/)).

**Kavach against these criteria: largely satisfied, by deliberate design.**

Its stated priority order is Safe, Readable, Simple, Fast, Familiar, and the higher one wins on conflict (docs/vision/vision.md). This is a Sebesta-style trade-off ranking made explicit. Few languages bother to write one down.

Every hard early decision is already settled and recorded in an ADR:

- Memory: arenas, no GC (dec 0003).
- Errors: errors as values with no exceptions (dec 0007), matching the Zig/Odin/Elm consensus.
- C interop: working in native code today.
- Extension: a small stdlib plus packages.
- Grammar: braces, no semicolons (dec 0001).

The spec-first rule ("if code and spec disagree, the code is wrong") makes C3's iterate-before-implementing lesson a standing rule.

Kavach also mostly consolidates rather than invents: unions, contracts, JS-style destructuring, Go-flavored tooling. Contracts come from Eiffel's lineage. Meyer designed Eiffel "to increase the reliability of commercial software", with contracts as its signature mechanism. Eiffel was ACM-awarded and ISO-standardized ([Wikipedia: Eiffel](https://en.wikipedia.org/wiki/Eiffel_(programming_language))).

Kavach keeps genuine invention for its niche: capabilities in signatures, agent guardrails, token-efficient AI tooling. That is exactly where Hoare's "consolidate, don't innovate" rule permits an exception, because the niche is the point.

Two things to watch:

- **Feature-interaction testing.** With inout (dec 0038) landing on top of contracts, capabilities, destructuring and interface values, Wirth's warning applies. The conformance suite should grow deliberate cross-feature tests (inout × contracts, inout × capabilities), not just per-rule ones.
- **Syntax decisions still rest on designer intuition.** Stefik's empirical work showed that intuition is unreliable: novices found Perl no easier than a language with randomly generated syntax ([Never Work in Theory](https://neverworkintheory.org/2011/10/24/an-empirical-comparison-of-the-accuracy-rates-of-novices-using-the-quorum-perl-and-randomo-programming-languages.html)). Quorum's measured gains came largely from word-based over symbol-heavy syntax ([Quorum/DO-IT](https://www.washington.edu/doit/node/6533)). Kavach's plain-keyword style (`require`, `uses`, `approve ... by`) points the right way. Cheap informal accuracy tests on non-users, before freezing agent-block syntax, would turn taste into evidence.

## Standards, specs and conformance tests

There is no official standard to pass. The real bar is spec rigor plus a conformance suite, and Kavach already clears most of it.

The official bodies exist: ISO/IEC JTC1 SC22 (working groups per language: C WG14, C++ WG21) and Ecma. They standardize *existing* multi-stakeholder languages and do not approve new ones. SC22 has been disbanding working groups, not adding them ([jtc1info.org](https://jtc1info.org/jtc1-subcommittees/sc-22/)).

Dart is the warning. Ecma created TC52 for it and published ECMA-408 in 2014. The standard was quietly abandoned after four editions, while the language moved on ([TechCrunch](https://techcrunch.com/2014/07/08/googles-dart-language-is-now-an-official-ecma-standard/); [Ecma](https://ecma-international.org/publications-and-standards/standards/ecma-408/)).

Standardization becomes relevant only when independent implementations, procurement requirements, or safety certification demand a frozen normative document. Kavach should not go near a standards body for years.

What the field actually expects is the **Go model** ([Go spec](https://go.dev/ref/spec)):

- One versioned reference document in the language's repo.
- EBNF grammar.
- Prose semantics with examples.
- Explicit "Implementation restriction:" markers.
- Per-feature version tags.

The upgrade path, when it matters, is what Ferrocene did for Rust. It restructured the reference with **stable paragraph-level IDs** to support safety qualification traceability. The FLS was the explicit prerequisite for qualifying Rust under ISO 26262 ASIL D and IEC 61508 SIL 4 ([Rust Foundation](https://rustfoundation.org/?p=6809); [Ferrocene](https://www.github.com/ferrocene/ferrocene)).

On the testing side, the strongest ecosystems make the suite part of the standard:

- ECMAScript's test262 holds 50,000+ test files, and Stage 4 standardization requires two implementations passing it ([tc39/test262](https://github.com/tc39/test262); [TC39 process](https://tc39.es/process-document/)).
- WebAssembly keeps spec, reference interpreter and test suite in one repo, and requires all three for every feature ([WebAssembly/spec](https://github.com/WebAssembly/spec)).

**Kavach is ahead of what a project of its size usually has here.**

- A 14-chapter spec marked source-of-truth, with must/must-not conventions and per-chapter Open questions, exceeds what Go had at launch.
- `tests/conformance/<NN-chapter>/<rule>.kav` with 324 files is test262's pattern at small scale. It is written "against the spec not the compiler", so any future implementation can prove itself.
- The development interpreter serves as the reference implementation, cross-checked against native output on every supported program (tests/native.rs). This is the same discipline as Wasm.

Three cheap gaps remain:

- **Stable rule IDs.** Filenames encode rules informally. Assigning explicit IDs in the spec (e.g. `3.4-R2`) and naming test files after them is the single step that does most to protect the future. It is the thing FLS added to Rust ([spec-fls-publish goal](https://rust-lang.github.io/rust-project-goals/2025h1/spec-fls-publish.html)).
- **A coverage checker that lists spec rules with no matching test.** Kavach checks links and line counts in scripts/check-repo.sh, but nothing closes the rule→test loop mechanically.
- **An explicit list of undefined and unspecified behavior.** The spec should enumerate this surface and keep it minimal. That list is what subset standards like MISRA exist to police, and a short one is a selling point.

Relatedly, one regulatory point costs nothing to claim. CISA/NSA now name memory-safe languages as what is, in practice, a list for market access ([NSA, June 2025](https://www.nsa.gov/Press-Room/Press-Releases-Statements/Press-Release-View/Article/3608324); [CISA roadmaps guide](https://cisa.gov/news-events/alerts/2023/12/06/cisa-releases-joint-guide-software-manufacturers-case-memory-safe-roadmaps)).

Kavach's workspace-wide `unsafe` denial with one audited runtime crate maps directly onto CISA's framing (spatial/temporal safety, bounded unsafe escape hatches). It deserves a page in docs/vision stating the memory-safety story in those terms.

## Tooling and performance

Tooling and performance are the pass/fail test. Kavach passes most of it but hasn't shipped any of it.

The documented failure modes of new languages are overwhelmingly ecosystem failures, not design failures. They are impractical tools (one platform, build-from-source), no C interop, too slow, and abandoned reference implementation ([Chevalier-Boisvert](https://pointersgonewild.com/2012/06/07/why-do-most-new-languages-fail/)).

Gleam v1 defined the 2020s tooling bar: a single binary containing compiler, build tool, package manager, formatter and language server. Gleam's one stated regret was under-investing in the LSP, "immature compared to the rest of the Gleam tooling," which was its top post-1.0 priority ([Gleam v1](https://gleam.run/news/gleam-version-1/)).

Zero-config formatting is the settled norm for new languages. From gofmt: "if configuration were possible, people would argue over the configuration too" ([opinionated-tooling post](https://www.youngju.dev/blog/compiler/2026-06-25-go-formatters-opinionated-tooling.en)).

Playgrounds demonstrably onboard users. Go built its playground so people could try the language before installing it ([Go blog](https://go.googlesource.com/website/+/master/_content/blog/playground-intro.md)).

Compile speed is now a headline feature. It was Go's founding requirement, and it is Zig's entire current flagship work ([LWN](https://lwn.net/Articles/959915)).

The evidenced architecture is exactly a fast debug backend plus LLVM release backend. Cranelift compiles ~10x faster while producing ~2x slower code ([LWN](https://lwn.net/Articles/965369)).

**Kavach already has almost all of this**, which is unusual for pre-alpha:

- One `kavach` binary with run/build/test/check/fmt/outline/query/context/mcp/lsp.
- A working LSP with VS Code client (errors as you type, hover, go-to-def, completion).
- A no-options formatter.
- The Cranelift-debug/LLVM-release split that rustc and Zig both arrived at.
- A dev interpreter retained permanently as reference implementation.
- C interop in native code.
- Windows as a first-class platform. It develops *on* Windows, which is rarer and more credible than porting later.
- `kavach test` generating property tests from types and contracts. This is a genuine differentiator, descended from Meyer's Design by Contract plus MacLennan's Automation principle.

The AI-tooling layer (`outline`, `query`, `context`, MCP server, llms.txt) has **no published prior art**. The research found no comparable token-efficient compiler CLI for agents. That makes it a new idea Kavach can rightly claim, and one worth writing up publicly.

On performance, Kavach's self-published suites beat C++ (closures 12×, loops 30×, sort 10×), largely via auto-parallelization onto all cores. They follow the credible format: idiomatic code per language, raw numbers, losses reported. This matters more than Benchmarks Game placement, given that site's known flaws ([Are We Fast Yet?](https://stefan-marr.de/papers/dls-marr-et-al-cross-language-compiler-benchmarking-are-we-fast-yet/)).

The honest caveat stays attached. These are self-measured micro-benchmarks, and several pit all cores against single-threaded C++ (stated openly in todo.md). Publishing the planned benchmarks against C#/Go/Rust/Python, with reproduction scripts, is what upgrades the claim from interesting to credible.

The gaps are concrete:

- **Nothing is distributed.** The website is built but not deployed. The VS Code extension is not on the Marketplace. There are no precompiled installers. Only Windows/Linux x86-64 is covered (no macOS/ARM64, which excludes a large share of developers).
- **There is no browser playground.** The Gleam precedent, compiling the Rust compiler to WASM with a JS API, serves both the playground and docs embedding.
- **A tree-sitter grammar is missing** beside the TextMate one. It is a single artifact that unlocks GitHub, Neovim, Zed and Helix highlighting ([CodeEditLanguages](https://github.com/HanweeeeLee/CodeEditLanguages)).
- **The package manager** (tools/pkg) is an empty placeholder. The Gleam lesson is to build resolution and lockfiles, but consider piggybacking an existing registry backend rather than running one from day one ([Gleam v1.1](https://gleam.run/news/gleam-v1.1/)).
- **The stdlib sits on polling sockets.** The stdlib is more complete than usual (HTTP/1.1–3, TLS, gRPC written in Kavach), but its sockets poll with 1–8 ms sleeps. The async event loop in "Noted for later" becomes essential the moment an outside user benchmarks a server.

The fuller-stdlib choice itself is well supported. A small std produces "an explosion of transitive dependencies" ([Phil Eaton](https://notes.eatonphil.com/2025-01-25-an-explosion-of-transitive-dependencies.html)). For an AI-agent language, fewer third-party packages means less untrusted code for guardrails to reason about.

But stdlib modules need a versioning story (Go's `math/rand/v2` pattern), so that batteries-included never means mistakes-frozen-forever.

## Adoption

Adoption is won socially: through a niche, interop, a stability promise, and a clear governance story.

The empirical core: "social factors usually outweigh intrinsic technical ones. In fact, the larger the organization, the more important social factors become". Existing code, expertise and libraries dominate, and "performance, reliability and simple semantics appear not to matter much at all" ([Meyerovich & Rabkin via ILNumerics](https://ilnumerics.net/blog/social-aspects-of-programming-languages/)).

Every clear success of 2007–2025 combined three things: a sponsor or platform that guaranteed survival, a niche where it was obviously better, and interop with an incumbent (Kotlin/Java+Android, TypeScript/JS, Go/cloud). Every stall missed at least two (D, Ceylon, Nim, Crystal).

Launching as general-purpose first is the common cause of death. Dart nearly died as a "JavaScript killer" and lives as "the Flutter language" ([Dice](https://www.dice.com/career-advice/fall-rise-dart-google-javascript-killer)).

Stability promises have a documented link to growth inflections. Kotlin's team says real growth started when backward compatibility got serious ([InfoQ](https://www.infoq.com/articles/kotlin-ten-years-qa/)). In contrast, "D never had a stable version" is cited as its main blocker ([D forum](https://forum.dlang.org/post/hzcoboztuevaucfrauem@forum.dlang.org)).

Timelines are long even with funding: 4–9 years to 1.0, double that solo (Nim: 11 years). Zig shows a donation-funded project thriving deliberately pre-1.0 for 11+ years, by stating clearly what is stable and showing visible momentum ([byteiota](https://byteiota.com/?p=16204); [ZSF](https://ziglang.org/zsf/)).

Governance failures that measurably hurt languages come in two shapes:

- Staying opaque after contributors arrive: Elm, "your hope of being useful depends on relationships" ([lukeplant.me.uk](https://lukeplant.me.uk/blog/posts/why-im-leaving-elm/)).
- Factional wars with no arbitration: Scala.

A BDFL phase is normal. The cheap insurance is public written decisions, a permissive license, and a stated succession story.

**Applied to Kavach:** the niche has the right shape and is rare. "the language AI agents write and operate safely" has no incumbent. It gives business gatekeepers a one-sentence statement of its value. It can be demonstrated by one person with public repos and benchmarks, unlike "better for large teams."

What adoption is missing is **interop**. Python/JS interop sits in Phase 3. Meyerovich's result argues it is the highest-value non-feature work on the roadmap, because it turns the incumbent ecosystems from a barrier into an asset. The TypeScript move here is "call your existing Python from a guarded Kavach agent". Kavach's trust ladder (observe → suggest → enforce) is the per-company analogue of incremental interop adoption.

Governance assets already in place:

- Dual MIT/Apache-2.0 licensing, the post-Rust norm, with Apache's patent grant ([Rust internals](https://internals.rust-lang.org/t/rationale-of-apache-dual-licensing/8952)).
- 38 public ADRs that work as an RFC-lite record.
- CONTRIBUTING.md and SECURITY.md.

Gaps:

- **CODE_OF_CONDUCT.md is absent.**
- The security@kavach.io mailbox is not live.
- There is no written who-decides statement. One sentence is insurance against what happened to Elm: "Kavach has a BDFL; decisions land as ADRs; here is what happens to the IP if the BDFL stops".
- There is no DCO/CLA line in CONTRIBUTING. One would avoid Rust's relicensing pain later.
- **There is no pre-1.0 stability policy.** Use the Gleam framing in reverse. Say loudly that everything may break until 1.0, keep a disciplined changelog, and put an editions-style mechanism in the spec before 1.0 ([Gleam v1](https://gleam.run/news/gleam-version-1/); [Go 1 compat](https://stuff.mit.edu/afs/sipb/project/golang/arch/amd64_deb60/doc/go1compat.html)).

The single maintainer is the built-in risk the vision document itself names (success metric: outside maintainers, currently none). The final verdict on D is that it "will fade once the major contributors retire or lose interest" ([D forum](https://forum.dlang.org/post/hzcoboztuevaucfrauem@forum.dlang.org)). The governance note exists to counter that scenario.

Finally, the **name**. "Kavach" collides with India's KAVACH train-protection system and NIC's Kavach government 2FA app. No clearance search has been done.

The Rust lesson is that the danger is less a squatter than a bad policy ([SFC on Rust trademark](https://sfconservancy.org/blog/2023/jul/27/trademark-history-and-rust/)). But a basic clearance search in relevant classes belongs on the near-term list, before the name accumulates more sunk cost. Registration can wait. A permissive-by-default policy intent can be one paragraph now.

## What to do next

Ship, connect, promise, in that order.

The gap list below is ranked by how much each step gains for its effort, weighted by the evidence:

| Priority | Action | Why (evidence) |
|---|---|---|
| 1 | Deploy kavach.io + version tags; publish VS Code extension to Marketplace | Zero-distribution is failure mode #2 ([pointersgonewild](https://pointersgonewild.com/2012/06/07/why-do-most-new-languages-fail/)); both are already built |
| 2 | Browser playground via compiler-to-WASM | Go/Gleam precedent; the 30-minute adoption plan depends on it ([Go playground](https://go.googlesource.com/website/+/master/_content/blog/playground-intro.md); [Gleam v1](https://gleam.run/news/gleam-version-1/)) |
| 3 | Pre-1.0 stability policy + editions mechanism in the spec; one-paragraph governance/succession note; CODE_OF_CONDUCT; live security@ mailbox; DCO line | Kotlin/D stability evidence; Elm/D governance evidence. Each is under a day of work |
| 4 | Stable spec rule IDs + a check that lists rules without conformance tests | The FLS lesson: cheapest now, of very high value if qualification or a second implementation ever matters ([Rust Foundation](https://rustfoundation.org/?p=6809)) |
| 5 | Name clearance search for "Kavach" (software classes, India + US/EU) | KAVACH rail / NIC Kavach collisions unexamined; pre-registration search is cheap |
| 6 | Tree-sitter grammar with highlight queries | One artifact → GitHub/Neovim/Zed/Helix ([CodeEditLanguages](https://github.com/HanweeeeLee/CodeEditLanguages)) |
| 7 | User guide (docs/guide; the repo's own note says it is now due) + published cross-language benchmarks with reproduction scripts | Docs/onboarding convert individual enthusiasts, the first audience ([ICFP 2020, Czaplicki](https://icfp20.sigplan.org/details/icfp-2020-papers/39/On-Language-Adoption)) |
| 8 | Async I/O event loop (to lower the risk in the HTTP stack); approvals across restarts | Becomes essential at first outside benchmark; completes the agent-durability story |
| 9 | Package manager (lockfiles first, piggyback a registry); then Python interop earlier than Phase 3 if possible | Gleam/Hex precedent; interop is the top social factor in adoption ([Meyerovich](https://ilnumerics.net/blog/social-aspects-of-programming-languages/)) |
| 10 | Write up the token-efficient AI tooling publicly; state the memory-safety story in CISA terms | Both are novel or give credibility for free; no prior art found for outline/query-style agent CLIs |

Deliberately *not* on the list:

- Standards bodies (Dart's wasted ECMA-408).
- Self-hosting. Go took 6 years post-launch. Rust implementation is no credibility problem: Gleam reached v1 in Rust.
- A foundation. Defer until there is money or trademark worth holding.
- macOS/ARM64 ahead of the above. It is important, but distribution of what exists comes first.

## Conclusion

The research changes the question. A new language does not pass an exam. It survives a market, and the market grades ecosystems, not semantics.

The odd thing about Kavach is this. It has already done the hard, slow, usually-skipped part: a real spec, conformance discipline, contracts-generate-tests, dual backends, an LSP, a coherent niche with no incumbent. It has left undone the easy part that actually kills languages: nobody outside one machine can try it.

Everything in the top five next steps is shipping and promising, not building. The project's move from strong private work to "language" is measured in deploys, not features.

Two deeper points carry forward.

First, Kavach's agent niche turns the field's hardest constraint around. Every prior language had to recruit human developers away from incumbent ecosystems. Kavach's primary "developers" are AI models, whose switching cost is an llms.txt and an MCP server. So its token-efficiency tooling is not a convenience feature. It plays the role that a platform owner's order plays for other languages. It is also the one asset worth publicizing before anyone else names the category.

Second, the discipline already in the repo (rule-per-test, spec-first, ADRs, honest benchmark rules) is the early form of exactly what safety qualification and future standardization will someday demand. Spending a few days making that traceability machine-checkable (rule IDs, coverage checker) turns today's good habits into tomorrow's lasting advantage, at near-zero cost.
