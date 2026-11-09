# 0047. The docs teach one idea per page

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

[Decision 0033](0033-docs-site-in-the-repo.md) made each docs chapter a page of tabs (Introduction, Code, Understand, Benchmark, Quiz). Mithlesh reviewed the site on 2026-11-03 and found it hard for beginners: one chapter mixed several ideas, such as `for`, `while` and `break` in "Loops", and printing inside "Hello". Sites beginners like (w3schools) give each idea its own small page. Sites experienced readers like (learncpp) add depth and exercises. Mithlesh approved a design that does both, on Mithlesh's design canvas.

## Decision

This replaces point 1 of decision 0033. Its other points stay.

1. **One tree: parts → pages → sub-pages.** The course has 14 parts, in the order a newcomer needs them (Start here, Basics, Functions, Collections, …). A page teaches one idea, normally one section of the spec. A page whose idea has several forms gets sub-pages under it, each a page of its own: For Loop › Over a list, Over a range, Over a map.
2. **A page is one scroll, not tabs.** In order: the idea in plain words, its syntax, examples that run, "Try it yourself" with a hidden answer, and a closed "Go deeper" part for readers who want the why. A short quiz may end it. Each page shows its level, its time to read, and the pages to read before it.
3. **The tree lives in one file** (`website/lib/course.ts`). A page is listed in the sidebar, the routes, the sitemap and `llms.txt` only once its content exists, so the site is complete at every step while the course is written.
4. **Old chapters move in whole until they are split.** An old tabbed chapter can fill a page of the new tree (Lists, Maps, …) until it is rewritten as one-idea pages.

## Consequences

- Beginners meet one new thing per page, and a page's address names exactly what it teaches (`/docs/basics/for-loop/over-a-range/`).
- Many more pages: about 100 in the sidebar and 130 sub-pages. Each needs its own checked samples, so the course is written part by part, in order.
- Links to the old `/docs/<section>/<chapter>/` addresses change. No version of the site is published yet, so nothing outside breaks.

## Alternatives considered

- **Keep the tabs and split only the big chapters:** fewer pages, but a five-line idea behind five tabs is still heavy for a beginner, and "Introduction" and "Code" separate the explanation from its example.
- **Two sites, one for beginners and one for experts:** every idea written twice, and the copies drift apart. Rejected: one page serves both through its "Go deeper" part.
