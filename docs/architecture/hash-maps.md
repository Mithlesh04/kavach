# Hash maps and sets

How native Kavach stores `Map` and `Set` (spec 2.2), why, and what was compared before choosing. The code is in [runtime/native/src/table.rs](../../runtime/native/src/table.rs) and [rt_maps.rs](../../runtime/native/src/rt_maps.rs).

## Requirements

- **Fast:** at least 2× as fast as C++'s `std::unordered_map` (to-do item 1).
- **Insertion order:** a map iterates in the order keys were added (spec 2.2).
- **Safe against hash flooding:** an attacker who chooses the keys (from a request, a file, a message) must not be able to make every key collide, which turns each operation from a few steps into a scan of the whole map.

## The design

```
entries:  [k0 v0 k1 v1 k2 v2 …]     keys and values in insertion order
hashes:   [h0 h1 h2 …]               one per entry, so growing never rehashes (not for integer keys)
index:    [0 3 0 1 0 2 0 0 …]        entry number + 1 at each position; 0 = empty
```

- The index uses open addressing with linear probing. Its size is a power of two, and it doubles when it is half full.
- Integer, boolean and character keys are compared as slots, so a lookup reads only the index and the entry. Their hash is cheaper to compute again than to store, so such tables keep no `hashes`.
- Setting `m[k] = v` probes once: the probe ends at the key's entry (its value is replaced) or at the empty position where the new entry goes.

### Hashing integer keys: cheap first, strong when needed

1. **Every integer map starts with a cheap hash:** `2 * (key ^ seed)`, where the seed is random for each run.
   - Keys 0, 1, 2, … land on every other position, so reading them in order reads memory in order.
   - This is where C++ gets its speed: its integer hash is the key itself. Without this, each lookup in a 2-million-key map was a cache miss, and Kavach was 4× slower than C++.
2. **The table watches every probe**, on inserts and on lookups. A probe longer than 64 positions means keys are colliding, by accident or by attack.
3. **Then that map switches to the strong hash for good**, and its index is rebuilt once.
   - The strong hash is a seeded 128-bit folded multiply.
   - Entry order does not change.

Lookups must watch too. Keys `k` and `k + size/2` share a position, so pairs of them build one long run while each insert probes only a step. A lookup of a missing key that lands in the run sees the long probe and triggers the switch.

The worst case an attacker can cause is therefore one slow operation per map, followed by a rebuild. Rust's standard `HashMap` once used the same idea ("adaptive hashing").

### Hashing other keys

- Text, records and other keys always use the strong seeded hash: a folded multiply over 8-byte words.
- Floats use it too, compared by value (`0.0` and `-0.0` are the same key).

### Small maps

A map whose keys are not integers keeps no index while it has 8 entries or fewer: a lookup compares the stored hashes in order, which is as fast at that size, and the map needs one allocation fewer. JSON objects are mostly this small; parsing a document of 150,000 small objects got 10–20% faster. The index is built when the ninth entry arrives.

### Removing

Removing shifts later entries up one place, to keep order, and rebuilds the index from the stored hashes. That costs O(n). See the open questions.

## What was compared

The research came from published benchmarks and papers. Every number has its source.

| Option | Result | Source |
|---|---|---|
| SipHash-1-3 (Rust's default) | about 9 ns per `u64`, about 12× slower than the fastest; safe | [foldhash](https://github.com/orlp/foldhash) |
| FxHash | fastest (0.67 ns), but no seed: trivially flooded. Keys that differ only in high bits take 67 ns per lookup instead of 1.8 | [foldhash](https://github.com/orlp/foldhash) |
| foldhash, rapidhash | about 0.8 ns, seeded, "minimally" flooding-resistant | [foldhash](https://github.com/orlp/foldhash), [rapidhash](https://docs.rs/rapidhash/latest/rapidhash/) |
| wyhash and rapidhash with public constants | collisions can be built when the secret constants are known | [ruby-core discussion](https://ml.ruby-lang.org/archives/list/ruby-core@ml.ruby-lang.org/message/QJT2STYSCXWVBN7HS24N4DO7CKBERL2N) |
| V8 (Node.js), March 2026 | integer-like strings were hashed with no seed (CVE-2026-21717); fixed with a seeded bijective mix | [Node.js advisory](https://nodejs.org/blog/vulnerability/march-2026-hashdos) |
| `std::unordered_map` | node per entry, chaining, prime modulo: 3.9–4.3× slower than `boost::unordered_flat_map` on integer keys | [Boost benchmark](https://bannalia.blogspot.com/2022/11/inside-boostunorderedflatmap.html) |
| Swiss tables (abseil, hashbrown) | SIMD probing of 16 control bytes; tombstones on removal build up | [abseil design](https://abseil.io/about/design/swisstables) |
| `ankerl::unordered_dense` | dense entries plus an index, like ours; removal swaps in the last entry, which breaks order | [design notes](https://raw.githubusercontent.com/martinus/unordered_dense/main/doc/design.md) |
| Elastic and funnel hashing (2025) | better worst-case probes near a full table; no gain at our load | [arXiv 2501.02305](https://arxiv.org/abs/2501.02305) |

Choices this led to:

- **Keep insertion order:** the dense entries plus an index, like Python's `dict`, `indexmap` and `ankerl`.
- **Never use an unseeded hash** (FxHash, identity) on its own. The V8 advisories show the cost.
- **Use a cheap hash with a watched, one-way switch** instead of always paying for the strong one. Applications run fast on ordinary keys, and attackers gain nothing.

## Results

`maps` benchmark: insert 2 million integer keys, then look each one up. Times in [benchmarks/loops](../../benchmarks/loops/README.md).

| Version | Kavach ÷ C++ |
|---|---|
| SipHash and a `HashMap` of buckets | 12.3 |
| Open addressing, folded-multiply hash | 5.7 |
| `m[k] ?? 0` as one lookup (no boxed optional) | 4.0 |
| Cheap integer hash with the watched switch | 0.99 |
| Direct calls for `m[k] ?? d` and `m[k] = v` (no generic dispatch) | 0.71 |
| One probe per `m[k] = v`; no stored hashes for integer keys | **0.48** |

## Open questions

- **Faster removal that keeps order.** Mark the entry as a hole, delete from the index by backward shift (Knuth's Algorithm R), and compact when holes pass half the entries. Python's `dict` works this way.
- **SIMD probing.** Probe groups of 16 one-byte tags, as Swiss tables do, for maps much larger than the cache.
- **Text keys.** Store each text's hash in the text object, so it is computed once.
- **Beating C++ by 2×.** Each operation is still a call into the runtime. The next step is generating the integer-key lookup inline.
- **Growing costs the most of what is left** (measured 2026-11-02, WSL, the whole `maps` program, C++ 82 ms): Kavach 60 ms. A table given its final size up front, so it never grows, takes 41 ms: exactly 2×. What was tried, and why none of it shipped:

  | Change | Kavach ms | Why not |
  |---|---|---|
  | Not writing the ordered entries on an integer insert | 53 (C++ 75) | No real gain: writing them is not the cost |
  | Zeroed buckets (keys stored XORed with the empty marker) | 124 | The system hands zeroed pages over one at a time as scattered inserts touch them; filling the index in order first is cheaper |
  | Reserving the entries' room when the index grows | 59 | No gain |
  | Growing four times at once (below 2^24 positions) | 51 | Up to three times C++'s memory for a map just past a growth |
  | A cheap hash without the factor 2 (keys packed densely) | 49, with the above | Too small to be worth losing the gaps |

  A size the program gives now skips growing: `Map.with_capacity(n)` (spec 2.2, 2026-11-02). With it the `sizedmaps` benchmark is 1.6× C++ given `reserve` (37 ms against 59), and 2× C++ without (37 against 73, the same run's `maps`). Growing in place without rehashing everything at once would only spread the same work out.

  **Doubling in place** (2026-11-09, `runtime/native/src/table/grow.rs`): an integer index that doubles keeps its old buckets, a `realloc` that moves pages without copying, and each key moves at most once, to its old position or that plus the old size. Only a key that would wrap past the end makes it place every key again. The insert phase of `maps`, 15 runs in turns on a loaded WSL: best 63 → 57 ms, median 91 → 83, with the same memory.

  **What a map given its size still spends** (2026-11-09, `samply`, `sizedmaps`): 43% filling the index once (64 MB of buckets set to the empty marker, which isn't zero), 16% the runtime call around each insert (`rt_maps::insert`: the map's type, its `RefCell`), 6% `unique`, 13% the lookups. So 2× on `maps` needs a smaller index or an insert generated inline, not more tuning of growth: even without growing, `maps` would take about 45 ms, and 2× is 44.
