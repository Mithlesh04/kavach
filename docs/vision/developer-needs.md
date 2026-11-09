# What Developers Want and Need in a Language

Research done 2026-11-03 from the 2023–2026 developer surveys and studies. The companion document is [language-design-research.md](language-design-research.md). That document covers what a new language must *pass*. This one covers what developers *want*: the demand side. Every figure carries its year and source.

**The short answer:** developers reward languages that remove whole categories of tedious work. Every major survey shows this.

- **They admire languages that guarantee correctness.** Rust has been Stack Overflow's most admired language every year, at 84.66% in 2023, 83% in 2024 and 72% in 2025 ([Stack Overflow 2023](https://survey.stackoverflow.co/2023/), [SO blog 2024](https://stackoverflow.blog/2024/07/24/developers-want-more-more-more-the-2024-results-from-stack-overflow-s-annual-developer-survey/), [SO 2025](https://survey.stackoverflow.co/2025/technology/)).
- **They name technical debt their top frustration.** It stands at ~63%, roughly double the next issue ([SO 2024 via RDEL](https://rdel.substack.com/p/rdel-54-analyzing-the-2024-stack)).
- **Their most persistent complaints about each language are all feedback-loop failures:**
  - Slow incremental rebuilds are Rust's #1 open-answer complaint ([Rust compiler-performance survey 2025](https://blog.rust-lang.org/2025/09/10/rust-compiler-performance-survey-2025-results)).
  - Dependency management has been C++'s top frustration for five straight survey years ([ISO C++ 2024 summary](https://isocpp.org/files/papers/CppDevSurvey-2024-summary.pdf)).
  - Error-handling verbosity is Go's oldest complaint. The Go team formally gave up on it in 2025 ([go.dev/blog/error-syntax](https://go.dev/blog/error-syntax)).

The AI era makes this sharper, but does not change it. Now 84–90% of developers use AI tools, but only ~33% trust their output. Code that is "almost right, but not quite" is the #1 frustration, at 66% ([tech.co on SO 2025](https://tech.co/news/survey-developers-using-ai-more-trust-less), [Google DORA 2025](https://blog.google/technology/developers/dora-report-2025/)).

So the main limit is now **verification cost**. Developers are measurably moving toward typed languages that machines can check. TypeScript contributors grew +66.63% year over year, and it became GitHub's #1 language ([Octoverse 2025](https://github.blog/news-insights/octoverse/octoverse-a-new-developer-joins-github-every-second-as-ai-leads-typescript-to-1)).

For Kavach the meaning is direct. Its chosen ground is correctness guarantees, contracts, one toolchain, fast feedback and AI-checkable code. The evidence says developers reward exactly this area. That holds only if Kavach also meets the two demands the data makes of every new language: near-instant rebuilds and restraint against complexity creep.

## What developers admire

The main pattern in the cross-language surveys is the gap between what developers use and what they admire. JavaScript has been the most-used language for over a decade (63.61% in 2023, 66% in 2025). Yet only 58% of its users admired it in 2023. Rust sat at 13% usage but ~85% admiration ([SO 2023](https://survey.stackoverflow.co/2023/), [InfoWorld 2023](https://www.infoworld.com/article/3699788/rust-most-admired-language-stack-overflow-survey-says.html)).

In 2025 the entire top of the admired chart was **Rust 72%, Gleam 70%, Elixir 66%, Zig 64%**. All four are recently designed, strongly opinionated languages. In each one, the type system or runtime eliminates a class of errors ([SO 2025](https://survey.stackoverflow.co/2025/technology/)).

None is there because of job-market size. Languages people are forced to use score low on the wish to keep using them. Languages people choose score high.

The salary data shows the same thing. The top-paying technologies are the same niche languages, which people choose and are not assigned. Zig was #1 in 2023 at a $103,611 median, and Erlang was #1 in 2024 at $100,636 ([InfoWorld 2023](https://www.infoworld.com/article/3699788/rust-most-admired-language-stack-overflow-survey-says.html), [Analytics India Magazine](https://analyticsindiamag.com/ai-trends/how-on-earth-is-erlang-the-top-paid-programming-language)). These medians rest on small samples.

What developers say and what they do point the same way. In JetBrains' 2025 Developer Ecosystem survey (24,534 developers), the languages people most want to adopt next are **Go (11%) and Rust (10%)**. Its Language Promise Index names TypeScript, Rust and Go the most promising ([heise on JetBrains 2025](https://heise.de/-10771317)). On GitHub, developers actually moved: TypeScript passed both Python and JavaScript in August 2025 ([Octoverse 2025](https://github.blog/news-insights/octoverse/octoverse-a-new-developer-joins-github-every-second-as-ai-leads-typescript-to-1)).

All of this adds up to one demand: static types, native-class performance and first-class tooling. Admiration also covers the whole toolchain, not just the syntax. The two most admired tools of 2025 were both opinionated package managers: **uv at 74.2% and Cargo at 70.8%** ([Weekly Rust on SO 2025](https://weeklyrust.substack.com/p/rust-tops-the-charts-again)).

Gleam is the best sign for a new language. **Gleam debuted at 70% admiration in its first survey appearance despite tiny usage** ([SO 2025](https://survey.stackoverflow.co/2025/technology/)). Design properties earn admiration: type safety plus a friendly toolchain. Age and market share do not.

**Lesson for Kavach:** a language can reach the admired group fast, and Kavach's safety-plus-speed position aims straight at it. But people judge the toolchain as much as the language. So the package and build story of the `kavach` binary needs the same design care as the type system.

## Five pains found in every ecosystem

Ranked by the weight of the evidence, the pains every ecosystem shares are:

1. **Technical debt and maintenance drag.** It was the top frustration in SO 2024, at ~63%. An older estimate from Stripe put ~42% of the work week into maintenance ([RDEL #54](https://rdel.substack.com/p/rdel-54-analyzing-the-2024-stack), [Lightspeed on Stripe 2018](https://lsvp.com/stories/the-developer-productivity-manifesto-part-3-leaving-software-on-the-table/)).
2. **Slow feedback loops.** This is the one pain a language design directly controls. Rust is the warning. In 2021, 61% were unsatisfied with compile times. In 2025, "waiting too long for an incremental rebuild after making a small source code change was by far the most common complaint" ([Rust 2021](https://blog.rust-lang.org/2022/02/15/Rust-Survey-2021/), [Rust perf survey 2025](https://blog.rust-lang.org/2025/09/10/rust-compiler-performance-survey-2025-results)).
3. **Dependency management.** C++'s top two frustrations have been library management and build times in every survey since at least 2021 ([ISO C++ 2021](https://isocpp.org/files/papers/CppDevSurvey-2021-04-summary.pdf)). Python's packaging crisis is "too many tools and users are not sure which ones to use" ([PSF strategy discussion](https://discuss.python.org/t/python-packaging-strategy-discussion-part-1/22420)). Dependency management was a top-five JS pain at 29% ([InfoWorld on State of JS 2024](https://www.infoworld.com/article/3804318/state-of-javascript-highlights-from-the-latest-javascript-developer-survey.html/amp/)).
4. **Error-handling design.** Verbosity is "one of the oldest and most persistent complaints about Go," and it topped Go's surveys for years ([go.dev/blog/error-syntax](https://go.dev/blog/error-syntax)).
5. **Missing static types.** This is the #1 JavaScript language pain point. It was named by 32% in 2024 and was #1 again in 2025. The ecosystem answered by migrating: 40% now write exclusively TypeScript, up from 28% in 2022 ([Brosset on State of JS 2024](https://patrickbrosset.com/articles/2024-12-16-my-very-short-and-incomplete-analysis-of-the-state-of-js-2024-survey-results/), [InfoQ on State of JS 2025](https://www.infoq.com/news/2026/03/state-of-js-survey-2025)).

Each language's complaints are exactly the features it skipped.

Go is 91–93% satisfying overall. Yet its top frustrations are idiom uncertainty (33%), features other languages have (28%) and finding trustworthy modules (26%). In the write-ins on missing features, error handling, enums/sum types and nil safety dominate ([Go survey 2025](https://go.dev/blog/survey2025)).

Rust's users agree with its core value overwhelmingly: **86% adopted it to build correct, bug-free software, 83% for performance** ([Rust 2023 survey](https://blog.rust-lang.org/2024/02/19/2023-Rust-Annual-Survey-2023-results/)). But they suffer its tooling costs: slow compilation, subpar debugging and disk usage ([Rust 2024 survey](https://blog.rust-lang.org/2025/02/13/2024-State-Of-Rust-Survey-results/)).

No shipped language combines Elm-grade errors, Cargo-grade tooling, Go-grade build speed, Rust-grade safety and Python-grade readability. The complaints about each language are the part it left out.

The history of Go's error handling carries extra weight as evidence. It is a real-world test of changing a language later. Three major syntax proposals over seven years all failed to reach consensus:

- check/handle in 2018
- try in 2019, with ~900 GitHub comments
- the Rust-style `?` in 2024

In June 2025 the Go team closed all such proposals "for the foreseeable future". It also admitted that the complaint remains top-ranked ([go.dev/blog/error-syntax](https://go.dev/blog/error-syntax)). Better error handling fails as a retrofit because people cannot reach consensus. Technology is not the problem.

**Lesson for Kavach:** errors as values with low ceremony had to be designed in from day one, and [decision 0007](../decisions/0007-errors-as-values.md) did this. The Go evidence says never to treat this as something to revise later.

Build speed is also a permanent headline feature. Do not treat it as an optimization pass. Rust shows that even the most admired language loses users over rebuild latency. Kavach's split (interpreter for development, native for release) is the right architecture. Sub-second incremental feedback should be a stated, tested performance budget. An aspiration is not enough.

## Why developers stay, and what stops new ones

Why users stay: the Rust surveys measure the source of loyalty better than any other.

- 79–82% say Rust helped their company achieve its goals.
- 77% of organizations are likely to use it again.
- The top investment reason is correct, bug-free software ([Rust 2023](https://blog.rust-lang.org/2024/02/19/2023-Rust-Annual-Survey-2023-results/), [Rust 2024](https://blog.rust-lang.org/2025/02/13/2024-State-Of-Rust-Survey-results/)).

Go's loyalty rests on trust and simplicity: 80% trust the Go team to do what's best for users. Satisfaction is credited to the "gestalt" of standard library plus built-in tooling ([Go 2024 H1](https://go.dev/blog/survey2024-h1-results), [Go 2025](https://go.dev/blog/survey2025)).

Newcomers do not reject the goals. What stops them is the perceived cost of learning, and lack of opportunity. **31% of Rust non-users cite perceived difficulty** as their primary reason, and 67% simply never had the chance to prioritize it. Also, 63% of former users would consider returning ([Rust 2023](https://blog.rust-lang.org/2024/02/19/2023-Rust-Annual-Survey-2023-results/), [Rust 2024](https://blog.rust-lang.org/2025/02/13/2024-State-Of-Rust-Survey-results/)).

The threat to loyalty from inside is complexity creep. **43% of Rust users name "Rust becoming too complex" their biggest fear for the language's future**, ahead of every other worry. And 20% want feature development to slow down ([Rust 2023](https://blog.rust-lang.org/2024/02/19/2023-Rust-Annual-Survey-2023-results/)).

Practitioners teach the same instinct: complexity is "a spirit demon that enters the codebase" ([grugbrain.dev](https://grugbrain.dev/)). It also shows in the features people love most. Elm's error messages, gofmt and Cargo each took a "soft" problem (message tone, formatting disputes, dependency workflow) and solved it completely, with zero configuration. Each removed a whole category of argument ([Compiler Errors for Humans](https://elm-lang.org/news/compiler-errors-for-humans), [The Cultural Evolution of gofmt](https://golang.google.cn/talks/2015/gofmt-en.slide), [practitioner consensus on Cargo](https://tildes.net/~comp/6jg/)).

These wins were present early or made mandatory. Retrofits (Python packaging unification, Go error syntax) have repeatedly failed.

Two more things affect loyalty.

**Documentation.** It is how developers actually learn and judge a language. 84% of learners use technical documentation, and 90% of those use the docs shipped inside API/SDK packages ([SO blog 2024](https://stackoverflow.blog/2024/07/24/developers-want-more-more-more-the-2024-results-from-stack-overflow-s-annual-developer-survey/)). In-package docs beat external tutorials.

**Switching.** It only happens for a step change, through an incremental path: "a 10% improvement is not enough to make developers drop what they're doing" ([The Outsider Dev](https://theoutsiderdev.substack.com/p/is-your-programming-language-better)). JetBrains finds Rust adopters arriving from Python via PyO3 to speed up hot paths "without rewriting their entire codebase" ([JetBrains State of Rust 2025](https://blog.jetbrains.com/rust/2026/02/11/state-of-rust-2025/)). This is the general form of the TypeScript-over-JavaScript move.

**Lesson for Kavach:**

- Guard the complexity budget as a governed resource. The 43% figure says a safety language's own users will punish added features harder than missing features. This supports ranking Simple above Fast in [vision.md](vision.md), and argues for an explicit feature-budget note.
- Keep doc comments and `kavach outline`-style in-package documentation first-class. That is where learners actually look.
- Treat Go's 33% "how do I write this idiomatically?" finding as a brief. An authoritative source of style and idiom (the formatter, `example` clauses in the stdlib) is worth as much loyalty as any feature.

## AI tools, verification cost and new languages

Usage is now near-universal:

- 84% of developers use or plan to use AI tools (SO 2025, up from 76%).
- 90% use AI daily per DORA, with a median of two hours a day.
- 85% per JetBrains ([tech.co](https://tech.co/news/survey-developers-using-ai-more-trust-less), [Google DORA 2025](https://blog.google/technology/developers/dora-report-2025/), [JetBrains 2025](https://blog.jetbrains.com/research/2025/10/state-of-developer-ecosystem-2025/)).

Trust, meanwhile, is collapsing. Accuracy trust fell from 43% to 33% in a year, and 46% actively distrust it. **66% name "AI solutions that are almost right, but not quite" their top frustration** ([LeadDev](https://leaddev.com/technical-direction/trust-in-ai-coding-tools-is-plummeting), [tech.co](https://tech.co/news/survey-developers-using-ai-more-trust-less)).

The METR randomized trial measured the review burden. It studied 16 experienced maintainers using AI on familiar repos. They were **19% slower**, accepted under 44% of generated code, and spent ~9% of their time cleaning AI output. They still believed they had been 20% faster ([METR via getdx](https://getdx.com/blog/metr-study-on-how-ai-affects-developer-productivity/)).

Generating code is cheap. Checking it is expensive. So what developers need from a language has shifted toward whatever makes machine-written code cheap to check.

The market already judges languages on this. GitHub explicitly credits AI for TypeScript's rise to #1: "Statically typed languages give you guardrails. If an AI tool is going to generate code for me, I want a fast way to know whether that code is correct," and language choice "becomes an AI-compatibility decision" inside a self-reinforcing loop. Popular languages get better AI support, which attracts adoption, which feeds training data ([Octoverse / Gazit](https://github.blog/news-insights/octoverse/typescript-python-and-the-ai-feedback-loop-changing-software-development/)).

This extends the 2013 Meyerovich & Rabkin result, which found that social factors and existing code dominate adoption. The LLM training corpus now adds a third channel that reinforces it.

The threat to any new language has a number. LLM pass@1 falls to **0–1% for no-resource languages**, and plain self-instruction fails to bootstrap them. But training on test-validated translated corpora (MultiPL-T) demonstrably works ([arXiv 2606.16827](https://arxiv.org/pdf/2606.16827), [arXiv 2308.09895](https://arxiv.org/pdf/2308.09895v3)).

The most substantive design advice is Armin Ronacher's "A Language for Agents". He asks for:

- readable explicit types that don't require a language server to interpret
- one command that lints, compiles, tests and reports pass/fail deterministically
- compiler diagnostics that tell the agent its next action
- typed results instead of exceptions ("agents struggle with exceptions, they are afraid of them")
- greppable, token-friendly syntax with one-to-one declaration-to-import mapping
- explicit mockable effects and deterministic, cacheable tests

Most importantly, he argues that agentic coding makes new languages viable again because "the breadth of an ecosystem matters less". An agent can port a missing library ([lucumr.pocoo.org](https://lucumr.pocoo.org/2026/2/9/a-language-for-agents/)). That claim directly attacks the historic #1 barrier to adoption: libraries. It is the strongest available argument that Kavach's timing is right.

One caution: a counterpoint essay notes that models can produce code that passes a type checker while remaining subtly wrong ([The New Blub Paradox](https://www.iankduncan.com/engineering/2026-06-26-the-new-blub-paradox)). So static checking helps AI only when diagnostics give productive feedback and are not a gate to be gamed. Contracts and generated property tests check behavior, where type checks only see shape. They are the stronger answer to "almost right."

## Priorities for Kavach

1. **One deterministic command to verify code is the product.** The market behind the 66% "almost right" figure is asking for `kavach check` and `kavach test` as one fast pass/fail gate, identical in development and CI. Kavach's contracts-generate-tests design attacks the measured trust gap more directly than types alone. Behavior checks beat type-shape checks that models learn to pass. That should be the headline pitch.
2. **Write every compiler diagnostic as an instruction for the next action.** This is Elm's discipline, now with an agent as the reader.
3. **The no-resource-language problem is Kavach's largest external risk.** LLMs score 0–1% on languages absent from their training data. The fix with evidence behind it is to generate a test-validated synthetic corpus of Kavach code, with the conformance suite and examples as the seed. Do not wait for organic training data.
4. **Publish and hold a sub-second incremental-feedback budget** before users exist to complain. This is the Rust lesson. Rebuild latency is the one pain the design directly controls. It also loses users for even the most admired language.
5. **Guard the complexity budget explicitly.** The 43% "too complex" fear among Rust's own users says a safety language is punished harder for added features than for missing features.
6. **No one has claimed capability sandboxing for agent code yet.** No prominent 2025 essay demands language-level capabilities for agent-written code. The discussion happens at the tool and runtime layer. So Kavach's capabilities-in-signatures and guardrails have no named competitor. This fits with [language-design-research.md](language-design-research.md), which found no prior art for its token-efficient agent tooling.
7. **Keep in-package documentation first-class** (doc comments, `example` clauses). 90% of doc-using learners read the docs shipped inside the package, not external tutorials.

## Conclusion

The surveys all say one thing. Developers want the verify step of programming to be fast, certain and free of configuration. That means:

- correctness guarantees they can trust
- feedback in under a second
- one official tool
- errors that teach
- a language that refuses to grow complicated

The AI era has turned this preference into an economic need. When machines write the code, the language's job is to make checking it cheap.

The admired-language data proves a language can reach this group quickly on design merit alone (Gleam at 70% in its first year). The Go/Python/C++ retrofit failures prove it can only be reached from the start. That is exactly where Kavach is.

This evidence adds one main point beyond [language-design-research.md](language-design-research.md): what developers ask for has reversed. The old finding was that ecosystems beat features. The 2025 data shows AI weakening the advantage of libraries and strengthening the advantage of verification. Types, contracts, deterministic tests and action-guiding diagnostics are becoming the social factors.

This document, and no other, points to Kavach's two cheapest steps with a large effect:

- Publish a measured incremental-feedback-time budget and hold the toolchain to it.
- Start building the test-validated Kavach training corpus now. The 0–1% pass@1 figure for unknown languages is the one number that could nullify everything else an agent-first language gets right.
