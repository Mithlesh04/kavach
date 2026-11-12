# The Bug Classes Kavach Must Prevent

Research done 2026-11-03 from fifty years of incident data: CWE/CVE rankings, vendor root-cause studies (Microsoft, Google, Uber), academic bug studies, and the famous failures.

Each class is graded against the current spec and decision records:

- **(a)** Kavach already rules it out by design.
- **(b)** Partly covered, with the one rule that closes the gap.
- **(c)** No answer yet.

Companions: [language-design-research.md](language-design-research.md) (what a language must pass) and [developer-needs.md](developer-needs.md) (what developers want).

**Summary:** Kavach already rules out these bug classes by construction:

- Memory unsafety, which causes roughly 70% of severe vulnerabilities in C/C++.
- Null dereference, the #1 production crash in Java and JavaScript.
- Silent integer wrap, the quietest latent corruption bug.
- Deserialization that executes code, the defining RCE class of Java and Python.
- Data races that can be prevented at compile time.

No shipping language covers this whole set.

The evidence says this is the right choice. Removing classes with a mechanical definition works at scale. Android's memory-safety share fell from 76% to under 20% when new code moved to safe languages ([Google Security Blog](https://security.googleblog.com/2025/11/rust-in-android-move-fast-fix-things.html)). But no language measurably reduces defects *in general* ([TOPLAS 2019 reproduction](https://arxiv.org/pdf/1901.10220)).

What remains are three kinds of gap.

**Partly covered (seven classes):**

- network capability scoping (SSRF)
- decoded data skipping invariants
- swallowed-error handlers
- negative durations
- locale-sensitive case mapping
- supply-chain policy not yet written down
- compound updates on `Shared`

**No answer yet (three classes):**

- Injection through future stdlib APIs (SQL, process spawning, HTML). This is the most important rule to commit to *now*, before those modules exist.
- An untrusted-data marker, the inbound twin of `secret`. It is the only language-level tool anyone has against prompt injection.
- Unit-of-measure confusion, which Kavach can reasonably declare out of scope.

## Three datasets rank the worst bugs: memory corruption, injection, mishandled errors

Bug classes have been measured well, and three independent bodies of evidence agree on the ranking.

First, **memory unsafety is ~65–76% of severe vulnerabilities in C/C++ codebases**:

- ~70% of Microsoft's patched CVEs over a decade ([MSRC](https://msrc.microsoft.com/blog/2019/07/a-proactive-approach-to-more-secure-code)).
- ~70% of Chrome's 912 high/critical bugs, and about half of those are use-after-free ([Chromium](https://www.chromium.org/Home/chromium-security/memory-safety/)).
- 76% of Android vulnerabilities in 2019 ([Google](https://security.googleblog.com/2024/09/eliminating-memory-safety-vulnerabilities-Android.html)).
- 68% of in-the-wild exploited 0-days ([Project Zero](https://projectzero.google/0day.html)).

Among *actively exploited* weaknesses in 2025, use-after-free and out-of-bounds write rank #2 and #3 in the world. #1 is OS command injection ([MITRE KEV insights](https://cwe.mitre.org/top25/archive/2025/2025_kev_insights.html)).

Second, once memory is safe, **injection and "data interpreted as code" take over**. The 2024 CWE Top 25 ranks XSS #1, SQL injection #3, path traversal #5, OS command injection #7 and deserialization #16. Of the top 25, nine-plus are fully present in memory-safe languages ([MITRE](https://cwe.mitre.org/top25/archive/2024/2024_top25_list.html)).

The two most expensive incidents in Java's history were both in this class, and neither was a memory bug:

- Equifax: ~$1.4B, OGNL expression injection in Struts ([GovInfoSecurity](https://www.govinfosecurity.com/equifaxs-data-breach-costs-hit-14-billion-a-12473)).
- Log4Shell: CVSS 10.0. A logging library evaluated `${jndi:…}` in logged *data*. It was present in ~88% of organizations ([Veracode](https://veracode.com/blog/research/state-log4j-vulnerabilities-how-much-did-log4shell-change)).

Microsoft spent four years trying to mitigate .NET's BinaryFormatter. It concluded that the mechanism is unfixable: a deserializer that instantiates wire-chosen types and runs their code. It was deleted from .NET 9 ([Microsoft DevBlogs](https://devblogs.microsoft.com/dotnet/binaryformatter-removed-from-dotnet-9)).

Third, in production *reliability* data the dominant classes are nulls, mishandled errors and silent arithmetic.

NullPointerException is the #1 logged production exception across 1,000+ Java apps ([OverOps/DZone](https://dzone.com/articles/we-crunched-1-billion-java-logged-errors-heres-wha)). The message "cannot read property of undefined" is the #1 JavaScript error ([Rollbar](https://rollbar.com/blog/top-10-javascript-errors/)).

The strongest single result in the literature: **92% of catastrophic distributed-system failures were triggered by wrong handling of errors the software had already detected**. In 35% of them the handler was trivially wrong: empty, log-only, or marked TODO ([Yuan et al., OSDI 2014](https://blog.acolyer.org/2016/10/06/simple-testing-can-prevent-most-critical-failures/)).

Silent integer wrap broke the JDK's own binary search for nine years ([Google Research](https://research.google/blog/extra-extra-read-all-about-it-nearly-all-binary-searches-and-mergesorts-are-broken/)). It also forced the FAA to mandate rebooting Boeing 787s every 120 days ([Federal Register AD 2015-09-07](https://www.federalregister.gov/documents/2015/05/01/2015-10066/airworthiness-directives-the-boeing-company-airplanes)).

On concurrency, Uber's race detector found 2,000+ data races in six months in Go, a language that *ships* a race detector ([PLDI 2022](https://arxiv.org/pdf/2204.00764)). Message passing proved as error-prone as shared memory when channels carry no ownership semantics ([Tu et al., ASPLOS 2019](https://blog.acolyer.org/2019/05/17/understanding-real-world-concurrency-bugs-in-go/)).

One limit applies to everything below. Language design reliably removes bug classes *with an exact mechanical definition*. It does not move overall defect density. Both sides of the FSE-2014 replication dispute agree that density is dominated by logic and requirements errors ([Berger et al.](https://arxiv.org/pdf/1901.10220)). Kavach's pitch must stay "class elimination," never "fewer bugs overall."

## Verdict (a): fourteen classes Kavach already rules out by design

**Spatial memory unsafety: out-of-bounds write/read, buffer overflow** (CWE-787/125, Heartbleed, CrowdStrike's kernel OOB read). `list[i]` and `Bytes[i]` panic out of range, and `.get(i)` returns an optional ([spec 2](../spec/02-types/README.md)). There is no pointer arithmetic outside `unsafe`. `unsafe` is opt-in per package via `kavach.toml` and surfaced by `kavach audit` ([spec 6](../spec/06-memory.md)).

Kavach checks *reads* too. A language that only prevented writes would still have allowed Heartbleed and Cloudbleed, two of the costliest incidents. Performance is no longer an excuse: Google's fleet-wide bounds checks cost **0.30% CPU** ([Google](https://security.googleblog.com/2024/11/retrofitting-spatial-safety-to-hundreds.html)).

**Temporal unsafety: use-after-free, double free** (the #2 actively exploited weakness). Safe Kavach has no `free`. Memory is automatic and values are freed at last use. Arena values provably cannot escape their block: trying is a compile error ([spec 6](../spec/06-memory.md)). The class exists only inside `unsafe`.

**Type confusion and uninitialized use** (the remaining classes Microsoft's root-cause data names). Kavach has static nominal types and no downcasts anywhere. An interface value deliberately has *no* cast back to its concrete type. A closed set is a union with compiler-enforced exhaustive `match` ([spec 2](../spec/02-types/README.md)). Every variable is initialized at declaration, and even `unsafe` `alloc` zeroes.

This also removes the erasure/ClassCastException class: 8.7% of Java methods contain casts, half unguarded ([OOPSLA 2019](https://2019.splashcon.org/details/splash-2019-oopsla/42/Casting-about-in-the-Dark-An-Empirical-Study-of-Cast-Operations-in-Java-Programs)). Kavach has no cast to guard.

**Undefined behavior.** The MIT STACK study found ~160 confirmed bugs where optimizers silently deleted security checks through UB reasoning. These include an exploitable Linux kernel privilege escalation ([Wang et al., SOSP 2013](https://pdos.csail.mit.edu/papers/stack:sosp13.pdf)).

Kavach's safe subset defines every behavior: overflow panics, shifts panic out of range, null doesn't exist, and data races can't compile. UB is confined to `unsafe`, where the spec says so explicitly.

One sentence in the spec is worthwhile: "safe Kavach has no undefined behavior" as a named guarantee, so the promise can be cited.

**Null/nil dereference** (CWE-476). NPE is #1 in ~70% of Java production environments ([OverOps/DZone](https://dzone.com/articles/we-crunched-1-billion-java-logged-errors-heres-wha)). The measured fix, Kotlin's null safety on new code, cut NPE crashes 33% ([Google Home](https://developer.android.com/stories/apps/google-home)). Kavach has no null at all: `T?`, `?.`, `??`, `if let`, and maps returning optionals ([spec 2](../spec/02-types/README.md)).

C# shows the lesson of adding this later. Its nullable reference types are opt-in warnings with escape hatches and no measured impact ([Microsoft](https://devblogs.microsoft.com/dotnet/embracing-nullable-reference-types/)). This confirms that a day-one default was the only correct call.

What remains: `.unwrap()` panics. `kavach audit --panics` lists every site ([spec 11](../spec/11-testing.md)), which keeps them in view.

**Silent integer wrap** (JDK binary search, 787 generator unit, Stagefright/BadAlloc where overflow *feeds* an allocation). Overflow is checked in debug *and* release, with `wrapping_add`/`checked_add` as explicit opt-ins ([spec 5](../spec/05-errors.md)). Big constants infer `BigInt` ([decision 0041](../decisions/0041-bigint-is-built-in.md)). `Decimal` exists for money. There are **no implicit numeric conversions** ([spec 2](../spec/02-types/README.md)).

That last rule also removes the Ariane 5 class, where a 64-bit float silently narrowed to a 16-bit int destroyed the rocket ([Inquiry Board report](https://www.inf.ed.ac.uk/teaching/courses/seoc2/1999_2000/ariane5rep.html)).

BadAlloc has 25+ CVEs: the same `size * count` check, hand-written wrong across a dozen professional RTOSes. Google's own Stagefright patch was wrong once. This proves the check cannot be left to discipline ([The Hacker News](https://thehackernews.com/2021/04/microsoft-finds-badalloc-flaws.html)).

**Deserialization as execution** (CWE-502; ysoserial, pickle, Rails YAML CVE-2013-0156). Kavach's `Json` is a closed union of seven data variants. `json.decode<T>` is strict and data-only. It never instantiates wire-chosen types and never runs user code during decode. It ships hostile-input limits by default: depth, size, field count, duplicate keys and out-of-range numbers all fail cleanly ([spec 2](../spec/02-types/README.md)).

Python's picklescan arms race shows why only data-only-by-construction works. It had five bypass CVEs in a year, trying to denylist code execution out of a code-executing format ([SentinelOne](https://www.sentinelone.com/vulnerability-database/cve-2025-71369/)).

The Log4Shell half of the class is also closed. Kavach has no `eval`, and string interpolation produces strings, never lookups with side effects.

**Ignored errors** (the "silently dropped" slice of the OSDI 92%). `! E` is in the signature. The caller must `try`, `catch` or `match`, and there are no exceptions to forget ([spec 5](../spec/05-errors.md), [decision 0007](../decisions/0007-errors-as-values.md)).

Go's remaining holes (bare `f()` dropping the error, `err` shadowing) don't exist here. An error-returning call won't compile unconsumed, and redeclaring a name in the same scope is an error ([spec 1](../spec/01-basics.md)).

Kavach goes further: `kavach test` runs every caller once per declared error variant of every capability it uses ([spec 11](../spec/11-testing.md)). The "simple testing of error paths" that would have caught 58% of catastrophic failures ([OSDI 2014](https://blog.acolyer.org/2016/10/06/simple-testing-can-prevent-most-critical-failures/)) is generated, not hand-written.

**Data races.** Values are copied into tasks. `Shared<T>` with automatic locking is the *only* shared mutable state, and the compiler rejects racy code ([spec 9](../spec/09-concurrency.md), [spec 6](../spec/06-memory.md)).

This is the Rust-grade guarantee, achieved without ownership annotations. It is the design with real-world evidence: Rust's non-blocking concurrency bugs required `unsafe` ([PLDI 2020](https://pldi20.sigplan.org/details/pldi-2020-papers/76/Understanding-Memory-and-Thread-Safety-Practices-and-Issues-in-Real-World-Rust-Progra)).

Deterministic turn-taking ([decision 0020](../decisions/0020-tasks-take-turns.md)) goes further than any mainstream language. The interleavings that made Dirty COW latent for 11 years and made races unreproducible are *the same on every run*. So a concurrency bug that exists is a bug tests can replay.

**Task leaks and unobserved async failure.** A task cannot outlive its spawning function, and siblings are cancelled on failure ([spec 9](../spec/09-concurrency.md)). This removes the goroutine-leak class ([ASPLOS 2019](https://blog.acolyer.org/2019/05/17/understanding-real-world-concurrency-bugs-in-go/)). It also removes the silently-swallowed-rejection class, which Node tolerated for five years before making it fatal in Node 15 ([nodejs/node #35232](https://github.com/nodejs/node/issues/35232)).

**ReDoS.** `regex.new` is linear-time by specification. The backtracking engine is a separate, explicitly-named opt-in with a bounded work budget ([spec 14](../spec/14-standard-library/README.md)). Go proved linear-time-only is shippable. Backtracking-by-default took down Stack Overflow and Cloudflare ([Cloudflare postmortem](https://blog.cloudflare.com/details-of-the-cloudflare-outage-on-july-2-2019/)).

**Coercion and equality traps** (PHP's `0e…` magic-hash authentication bypass, [PentesterLab](https://pentesterlab.com/glossary/php-type-juggling); JS `==`). Kavach has no implicit conversions and one strict deep structural equality ([decision 0039](../decisions/0039-equality-is-deep-and-fast.md)). PHP needed ~18 years to fix `==` because equality semantics are unfixable post-hoc. Kavach got them right on day one.

**Prototype pollution** (lodash CVE-2019-10744, CVSS 9.1, [Kodem](https://www.kodemsecurity.com/cve-archive/cve-2019-10744)) is structurally impossible. Records are closed nominal types, and no shared mutable prototype exists.

**Resource cleanup**: deterministic `Drop` at last use plus structured tasks is the RAII end of the design ranking. The GC-finalizer end was formally abandoned by Java itself (JEP 421).

## Verdict (b): seven classes partly covered, and one rule each closes the gap

**SSRF and network scoping** (CWE-918, #19). `Fs` grants narrow very well. `Fs.read("./data")` is runtime-enforced and blocks `..` and symlink escapes ([spec 8](../spec/08-capabilities-and-agents/README.md)). This covers most of path traversal (#5) *when used*. `Net` has no equivalent. It is all-or-nothing, and `Net("api.stripe.com")` sits in the open questions.

For an AI-agent language this is the most valuable capability refinement. SSRF and data exfiltration by a confused or hijacked agent are both "code that can open any URL" problems. String validation cannot fix them.

**Rule needed: host-scoped `Net`, symmetrical with folder-scoped `Fs`, before agents ship to production users.** Also make whole-disk `uses Fs` inside an `agent` a lint.

**Decoded data bypasses invariants.** Spec 2's own open question: `json.decode<T>` does not check the type's `invariant`s. This is the "parse, don't validate" boundary. The type system promises an `Account` with `balance >= 0` cannot exist ([spec 7](../spec/07-contracts.md)), but a decoded one can.

It is also what remains in Kavach of the mass-assignment class (GitHub 2012, [The Hacker News](https://thehackernews.com/2012/03/github-hacked-with-ruby-on-rails-public.html)): wire data constructing records directly.

**Done (2026-11-12, [decision 0086](../decisions/0086-decoding-reports-every-problem.md)): decoding enforces invariants; a violation is a problem in the decode's `Invalid` error, with the record's path.** It was one of the single decisions on the board that paid back the most.

**Swallowed error handlers** (the OSDI 35% slice: empty or log-only handlers). Errors can't be *ignored*, but they can be explicitly discarded. `catch _ => {}` and `catch default` are legal, and the spec itself uses one in `Drop`. Yuan's Aspirator checker found 143 real bugs just by flagging these patterns. Kavach's agent findings report already does the deep version ("Email.send fails after a refund succeeded in 61 scenarios", [spec 11](../spec/11-testing.md)).

**Rule needed: `kavach audit` lists every `catch` that discards the error value and yields a constant, like it lists `unsafe` and `reveal`.** This is cheap, proven, and matches Kavach's audit philosophy.

**Panics as a denial-of-service vector.** Bounds checks and checked overflow turn corruption into panics. That is correct, but a panic reachable from attacker input is an availability bug. CrowdStrike took down 8.5M hosts with the crash, not an exploit ([root-cause analysis](https://crowdstrike.com/wp-content/uploads/2024/08/Channel-File-291-Incident-Root-Cause-Analysis-08.06.2024.pdf)).

Kavach's containment is already good. A panic stops one task, and an agent keeps serving ([spec 5](../spec/05-errors.md)). `kavach audit --panics` plus generated tests hunt reachable panics.

**What's missing is only a default: make "zero unproven panic sites" the shipped-release norm (`--release` refuses `✗` lines unless configured), not an optional team policy.**

**Compound operations on `Shared`.** Single `update` calls are atomic. Cooperative turn-taking means a task is never preempted between waits. That silently removes most of Lu et al.'s atomicity-violation category (~97% of non-deadlock concurrency bugs are atomicity or order violations, [ASPLOS 2008](https://www.cs.columbia.edu/~junfeng/08fa-e6998/sched/readings/concurrency-bugs.pdf)).

But check-then-act *across an I/O wait*, or invariants spanning two `Shared` values, can still be written. Deadlock detection fires only when *every* task waits ([spec 9](../spec/09-concurrency.md)).

**Needed: a short spec section naming the hazard ("a `Shared` read is stale after any wait") plus an idiom: pass a closure spanning the whole transaction to `update`, or a `Shared.transaction` over several values.**

Deterministic replay already makes these testable. Lu et al. showed ~92% of such bugs trigger by ordering ≤4 accesses, which seeded exploration can do.

**Supply chain.** The capability system is the strongest supply-chain design any language has proposed. A dependency that never declares `Net` cannot exfiltrate, and `kavach audit` shows every package's capabilities and `unsafe`.

The attack data says design decides outcomes. npm, with default-executing install scripts, hosts 98.5% of observed open-source malware ([Sonatype](https://www.sonatype.com/en/press-releases/open-source-malware-reaches-778500-packages)). Go, with no hooks, hosts almost none.

But three things are not yet written down:

- **(1) An explicit spec rule that dependencies are compiled only: no install, build or codegen hooks, ever.** This is currently true by omission, which is not a promise.
- **(2) The per-dependency capability allowlist in `kavach.toml`** (an open question in [spec 10](../spec/10-modules-and-packages/README.md)). This turns audit into enforcement: `stripe = { version = "1.2", can = ["Net"] }`.
- **(3) The registry decisions**: signing, immutable versions, 2FA, no reuse of deleted names. PyPI's deleted-name hijack and the event-stream handoff are the cautionary tales ([devclass](https://devclass.com/2024/09/04/pypi-repository-vulnerable-to-deleted-package-hijack-say-researchers/)).

All three are policy, not research. They just need to be committed before an ecosystem exists.

**Time, locale and TLS edges.** Kavach already separates `time.monotonic_ms` from the wall clock. The test fake's jumping `Clock` means 248-day-class counters actually get exercised ([spec 11](../spec/11-testing.md)). Three small holes remain:

- A `Duration` can hold negative milliseconds. This is the exact Cloudflare leap-second failure, where a negative duration "should always have been, at worst, zero" propagated into panics ([Cloudflare](https://blog.cloudflare.com/how-and-why-the-leap-second-affected-cloudflare-dns/)). So **make `Duration` non-negative by invariant, with a signed difference type if ever needed**.
- `String.upper()`/`lower()` semantics are unspecified. So **define them as locale-independent (ordinal) with locale operations explicit**, closing the Turkish-i class.
- Spec the TLS rule the CCS 2012 studies demand: certificate validation in `std.net` **cannot be disabled by any one-liner**. If a dev escape hatch ever exists, it is `reveal()`-loud and `kavach audit`-listed. In "the most dangerous code in the world", 1,074 Android apps had trust-all validators, caused by API design, not crypto ([Georgiev et al.](https://www.cs.umd.edu/class/fall2022/cmsc614/papers/most-dangerous-code.pdf)).

## Verdict (c): three classes with no answer yet, and what to commit to now

**Injection through stdlib APIs that don't exist yet (SQL #3, OS command #7, XSS #1).** Kavach currently has no SQL module, no process-spawning API and no HTML templating. So the class is absent only because the attack surface is.

This is the best time to act. 25 years of "educate developers to escape strings" has failed ([MITRE](https://cwe.mitre.org/top25/archive/2024/2024_top25_list.html)). The languages that killed instances of the class did it by making the unsafe form unrepresentable.

**The rule to adopt now, as a decision record that applies to every future module: no stdlib or blessed-package API ever accepts a string where that string will be parsed as code or commands.** Concretely:

- Database APIs take parameterized queries only, with no string-concatenation entry point.
- Process spawning takes a program plus an argv list, with no shell-string form. Shellshock and ImageTragick were both "data reaches a shell" ([Ubuntu](https://ubuntu.com/security/CVE-2014-6271), [Cloudflare](https://blog.cloudflare.com/inside-imagetragick-the-real-payloads-being-used-to-hack-websites-2)).
- HTML/template output auto-escapes by type.

Written today, this costs nothing. Retrofitted later, it is PHP's 18-year `==` saga.

**Untrusted-data taint: the inbound twin of `secret`.** Kavach's `secret` tracking is outbound taint: sensitive values cannot *leave* through prints, prompts, logs or JSON ([spec 8](../spec/08-capabilities-and-agents/README.md)). It is compiler-enforced, which is rare and valuable.

There is no inbound equivalent. Nothing marks a string that *arrived* from the network, a file or a model answer. So nothing stops it flowing into a prompt, a path or (someday) a query. This is the only language-level handle anyone has proposed for **prompt injection**, the signature attack on AI agents.

Kavach's position today is containment rather than prevention, and that position is honest and strong:

- A hijacked model still cannot exceed `can`, `limit` or `approve`.
- Its output is schema-checked by `ask<T>` rather than parsed as actions.
- Agent exploration tests every answer the model could give, including malicious-shaped ones ([spec 8](../spec/08-capabilities-and-agents/README.md), [spec 11](../spec/11-testing.md)).

That containment-first stance is correct and should be stated as the official security model.

But evaluate a `tainted` (or `outside`) marker as a Phase-2+ decision record. Values from `Net`/`Fs`/`ask`/`Env` carry it. Sinks like `Fs` paths, prompts and future query APIs either refuse tainted values or require an explicit, auditable parse/sanitize step. Even a lint-strength version would make Kavach the first language where "user input reached the prompt unsanitized" is machine-visible.

**Unit-of-measure confusion** (Mars Climate Orbiter's lbf·s vs N·s; the Patriot's tick-to-seconds drift killed 28 people, [GAO IMTEC-92-26](https://www.gao.gov/assets/imtec-92-26.pdf)). Kavach has no units in the type system and should not add them to the core. F# demonstrates it as a niche feature.

Kavach's nominal wrapper types plus invariants (`type Newtons { value: Float }`) already express the defense at library level. `Duration` with unit-suffix constructors (`5.seconds`) covers the most common instance.

**Record it as a conscious non-goal with the library idiom documented**, so the gap is a choice, not an oversight.

Deploy-process errors are similarly out of language reach (Knight Capital's eighth server, [SEC](https://www.sec.gov/news/press-release/2013-222)). Even so, Kavach's compile-time agent config, `observe`/`enforce` modes and `--dry-run` already push further into that territory than any peer.

## The scorecard: twenty-two classes, ranked and graded

Rank weighs exploitation data (KEV/CWE), production frequency, and incident cost. Verdicts: **(a)** ruled out by design, **(b)** partly covered, **(c)** no answer yet.

| # | Bug class | Evidence anchor | Verdict | Action for Kavach |
|---|---|---|---|---|
| 1 | Out-of-bounds write/read | #1 CWE 2023; Heartbleed; CrowdStrike | **(a)** | None; keep `unsafe` audited |
| 2 | Use-after-free / double free | #2 exploited weakness 2025 | **(a)** | None |
| 3 | SQL/command/HTML injection | SQLi #3, cmd inj #7, XSS #1; Equifax | **(c)** | Decision record now: no string-as-code APIs, ever |
| 4 | Deserialization executes code | CWE-502; Log4Shell; pickle | **(a)** | None; keep `Json` closed and data-only |
| 5 | Null/nil dereference | #1 Java/JS production error | **(a)** | None; `audit --panics` covers `.unwrap()` |
| 6 | Mishandled errors | 92% of catastrophic outages (OSDI'14) | **(b)** | Audit `catch`-and-discard handlers |
| 7 | Silent integer overflow / narrowing | 787, Ariane 5, BadAlloc | **(a)** | None; decision 0041 + checked-always |
| 8 | Data races | 2,000+ at Uber in 6 months | **(a)** | None; deterministic turns are a lasting advantage |
| 9 | Supply-chain attacks | npm = 98.5% of OSS malware | **(b)** | Spec "no install hooks"; dependency capability allowlist; registry rules |
| 10 | Prompt injection (AI agents) | emerging; no existing answer | **(b/c)** | State containment model; evaluate `tainted` marker |
| 11 | Type confusion / uninitialized use | Microsoft root-cause data | **(a)** | None |
| 12 | Path traversal | #5 CWE 2024 | **(b)** | Lint whole-`Fs` grants in agents |
| 13 | SSRF / unscoped network | #19 CWE 2024 | **(b)** | Ship `Net("host")` before production agents |
| 14 | Undefined behavior / deleted checks | ~160 bugs (STACK, SOSP'13) | **(a)** | Add one-line "no UB in safe code" guarantee |
| 15 | Mass assignment / unvalidated decode | GitHub 2012; Rails | **(b)** | Decode must enforce `invariant`s |
| 16 | Atomicity/order violations | ~97% of non-deadlock concurrency bugs | **(b)** | Spec the stale-after-wait hazard + transaction idiom |
| 17 | Attacker-reachable panics (DoS) | CrowdStrike availability lesson | **(b)** | Zero-unproven-panics default for release |
| 18 | ReDoS | Stack Overflow, Cloudflare outages | **(a)** | None; linear-time default |
| 19 | Resource/task leaks | JEP 421; goroutine leaks | **(a)** | None; `Shared` cycles debug-detected |
| 20 | Time/duration/locale | Cloudflare leap second; Turkish-i | **(b)** | Non-negative `Duration`; ordinal case default |
| 21 | Coercion/equality; prototype pollution; cast bugs | PHP `0e…`; lodash 9.1; ClassCastException | **(a)** | None |
| 22 | Unit confusion; deploy/config semantics | Mars Climate Orbiter; Knight Capital | **(c)** | Declare non-goals; document wrapper-type idiom |

## Conclusion

Grading Kavach against fifty years of incident data shows that the hard part is done. Every class that dominates *exploitation* statistics (1, 2, 4, 5, 7, 8, 11, 14) is already closed by decisions on the books. They are closed the way the evidence demands: by construction, in defaults, not by opt-in discipline.

The Android data says this translates directly. Class elimination at the point where new code is written captures the benefit without anyone rewriting old code.

What remains is almost entirely *writing rules down before surfaces exist*:

- the injection rule before `std.sql`
- the no-install-hooks promise before a registry
- host-scoped `Net` before production agents
- invariant-checked decode before anyone depends on the loophole

Each gap above is a one-decision-record fix, and none requires new research.

The one new obligation is inbound taint for untrusted data. No existing language has solved it, and only Kavach is placed to own it. The defining attack on an AI-agent language is data persuading the interpreter of intent. Kavach already has the compiler machinery (`secret` flow tracking) that the feature would mirror.

The data shows one more thing. Kavach's generated error-path tests, deterministic scheduling, fake clock that jumps limit windows, and agent scenario exploration attack the *residue*: wrong-but-present handler logic, latent time bombs, order-dependent bugs. Type systems cannot reach these, and they caused the OSDI 92%.

No other language couples class elimination with generated adversarial testing of what's left. That combination is the claim Kavach can stand behind. No single feature is. The bugs Kavach cannot make impossible, it makes reproducible and tested by default.
