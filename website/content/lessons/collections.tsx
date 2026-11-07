import Link from "next/link";
import { HopDiagram } from "@/components/hop-diagram";
import { C } from "@/components/inline-code";
import { RaceChart } from "@/components/race-chart";
import { SrcRef } from "@/components/src-ref";
import { VerifyBlock } from "@/components/verify-block";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, Note, P } from "./parts";
import { Limits } from "./limits";

// ---------- Lists ----------

export const listsDoc: LessonDoc = {
  lead: "A list holds many values of one type, in order, under one name: [\"mango\", \"fig\"].",
  simple: <>A list is like a numbered shopping list: many things of one kind, in order, under one name. You can add to the end, and find an item by its place, where the first place is 0.</>,
  minutes: 5,
  before: ["basics/for-loop", "basics/variables"],
  body: (
    <>
      <Ex name="lists/overview" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Square brackets make a list. Its type is written <C>[String]</C>: a list of strings.</li>
        <li><C>fruits[0]</C> is the first item: positions start at 0.</li>
        <li><C>push</C> adds to the end. It changes the list, so the list must be a <C>let</C>.</li>
      </ul>
      <Note>All the items of a list have the same type. To group values of different types, use a <Link href="/docs/collections/tuples/">tuple</Link> or a <Link href="/docs/types/records/">record</Link>.</Note>
    </>
  ),
  tryIt: {
    task: <>Make a list of three numbers, add a fourth, and print how many there are.</>,
    answer: <P><C>let xs = [1, 2, 3]</C>, then <C>xs.push(4)</C> and <C>print(xs.len())</C>: <C>4</C>.</P>,
  },
  deeper: (
    <>
      <P>
        Two things make Kavach lists fast. Values never alias: a list always has one owner, so the compiler can
        keep it in registers and skip defensive copies. And lists of small items are packed, so more items fit in
        each cache line:
      </P>
      <Limits id="lists" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="runtime/native/src/flat.rs" line={60} symbol="get_item" />
        <SrcRef file="docs/decisions/0030-byte-packed-lists.md" label="decision 0030" />
        <SrcRef file="docs/decisions/0045-packed-lists-of-small-numbers.md" label="decision 0045" />
      </div>
      <RaceChart suite="loops" id="lists" />
      <VerifyBlock suite="loops" id="lists" />
    </>
  ),
};

export const makeListDoc: LessonDoc = {
  lead: "Write the items in square brackets, or start empty with a type and push.",
  simple: <>You can write a list out in full, like a menu printed in advance, or start with an empty one and add items as they come. An empty list must say what kind of items it will hold.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="lists/sub-build" />
      <P>An empty list has no items to guess the type from, so write it: <C>let a: [Int] = []</C>.</P>
    </>
  ),
};

export const readListDoc: LessonDoc = {
  lead: "list[i] reads the item at position i, counting from 0; list.get(i) gives none instead of stopping.",
  simple: <>Like asking for the item on a given shelf of a numbered rack. Ask for a shelf that doesn&apos;t exist and the program stops; <C>get</C> answers &quot;nothing there&quot; instead, and you choose what to do.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="lists/index" />
      <P>
        <C>names[5]</C> on a three-item list can never work. When the compiler can see that, with a list written out
        and a number for the index, it refuses the program:
      </P>
      <Ex name="lists/index-past.fails" />
      <P>
        When the index is worked out while the program runs, the same mistake stops the program with a clear
        message instead. When a position may be past the end, <C>get</C> gives an optional, and <C>??</C> picks what
        to use then. <C>first()</C> and <C>last()</C> work the same way:
      </P>
      <Ex name="lists/sub-read" />
    </>
  ),
};

export const changeListDoc: LessonDoc = {
  lead: "list[i] = value replaces an item. The list must be a let.",
  simple: <>Like rubbing out one line of a pencil list and writing something new in its place. A <C>const</C> list is written in ink and can&apos;t be changed, so use <C>let</C>.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="lists/set-item" />
      <P>Changing a <C>const</C> list is refused, with the fix:</P>
      <Ex name="lists/const-push.fails" />
    </>
  ),
};

export const addRemoveDoc: LessonDoc = {
  lead: "push adds an item to the end; filter keeps only the items you want.",
  simple: <>Adding is like putting one more thing in your basket. Filtering is like going through the basket and keeping only what passes a test you choose, and leaving the rest out.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="lists/add-remove" />
      <P>
        <C>filter</C> makes a new list of the items its test is true for. Assigning it back to <C>cart</C>{" "}
        removes the rest.
      </P>
    </>
  ),
};

export const sortListDoc: LessonDoc = {
  lead: "sort() gives a sorted copy; sort_by sorts by a key; reverse() turns a list around.",
  simple: <>Like making a tidy copy of a messy list: <C>sort</C> puts the copy in order, <C>sort_by</C> orders it by something you choose, such as a score, and <C>reverse</C> turns it around. The original stays as it was.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="lists/sort" />
      <P>
        Each gives a new list and leaves the original as it was: <C>xs</C> is still <C>[3, 1, 4, 1, 5]</C>.{" "}
        <C>sort_by(.score)</C> sorts players by their score, keeping equal scores in their order.
      </P>
      <Note>
        How text, floats and records sort, sorting by two keys, and which algorithm runs: see{" "}
        <Link href="/docs/collections/sorting/">Sorting</Link>.
      </Note>
    </>
  ),
};

export const pipelineDoc: LessonDoc = {
  lead: "map changes every item, filter keeps some, sum adds them up, and count, any, all and find ask questions.",
  simple: <>Like a production line: each item passes stations that change it, drop it or count it, and at the end you get one answer. Each station is a word, so the whole line reads like a sentence.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="lists/pipeline" />
      <P>Chained, they read like a sentence, and run as a single loop with no list built between the steps:</P>
      <Ex name="functions/dot" />
    </>
  ),
};

// ---------- Maps ----------

export const mapsDoc: LessonDoc = {
  lead: "A map stores values under keys: [\"asha\": 31] keeps 31 under the key \"asha\".",
  simple: <>A map is like a row of labelled lockers: each label, the key, opens one locker with a value inside. You find things by their label, not by their position.</>,
  minutes: 5,
  before: ["collections/lists", "basics/if-let"],
  body: (
    <>
      <Ex name="maps/ages" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>[String: Int]</C> is a map from text keys to whole-number values; <C>[:]</C> is an empty one.</li>
        <li>Reading a key gives an optional: the key may not be there, so <C>?? 0</C> says what to use then.</li>
        <li>A loop visits the entries in the order they were added, every time.</li>
      </ul>
      <Note>Other languages call this a dictionary, a hash map or an object.</Note>
    </>
  ),
  tryIt: {
    task: <>Count how many times each word appears in <C>[&quot;a&quot;, &quot;b&quot;, &quot;a&quot;]</C>.</>,
    answer: (
      <P>
        Start with <C>let counts: [String: Int] = [:]</C>; for each word, <C>counts[w] = (counts[w] ?? 0) + 1</C>.
        It ends as <C>[&quot;a&quot;: 2, &quot;b&quot;: 1]</C>.
      </P>
    ),
  },
  deeper: (
    <>
      <P>
        Past the caches, each hop into memory costs about 70 ns whatever you compute there. The design question
        is not the hash, it is <strong className="text-ink">how many hops one lookup touches</strong>:
      </P>
      <HopDiagram />
      <P>
        Kavach keeps keys and values side by side in one flat table and probes it linearly at most half full, so
        a lookup is about 1.5 probes, each a quarter of a cache line. The hash is seeded for every run, so hostile
        keys can&apos;t force collisions.
      </P>
      <P>
        C++&apos;s <C>std::unordered_map</C> promises that erasing never moves other entries, which forces one
        heap node per entry: one more hop. Kavach promised insertion order and no aliasing instead. Check both:
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="runtime/native/src/table.rs" line={188} symbol="bucket_of" />
        <SrcRef
          file="libstdc%2B%2B-v3/include/bits/hashtable.h"
          symbol="_M_find_before_node"
          repo={{ url: "https://github.com/gcc-mirror/gcc/blob/master", label: "gcc-13" }}
        />
      </div>
      <RaceChart suite="lookups" id="dict" />
      <VerifyBlock suite="lookups" id="dict" />
    </>
  ),
};

export const makeMapDoc: LessonDoc = {
  lead: "Write key: value pairs in square brackets, or start empty with [:] and a type.",
  simple: <>Write the labels and their values out in full, or start with an empty map and fill it later. An empty map must say what kind of labels and values it will hold.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="maps/sub-create" />
      <P>When you know a map will get many entries, <C>Map.with_capacity(n)</C> makes room for them up front, so filling it never stops to grow:</P>
      <Ex name="maps/sub-room" />
    </>
  ),
};

export const readMapDoc: LessonDoc = {
  lead: "map[key] gives an optional: the value, or none when the key isn't there.",
  simple: <>Like asking at a cloakroom for a coat by its ticket number: you may get the coat, or be told there is none for that ticket. So you also say what to use when nothing is there.</>,
  minutes: 2,
  body: <Ex name="maps/sub-read" />,
};

export const changeMapDoc: LessonDoc = {
  lead: "map[key] = value adds a key or changes its value; remove(key) takes it out.",
  simple: <>Like a notice board with name tags: pin a note under a new name, swap the note under a name that is already there, or take a name and its note down.</>,
  minutes: 2,
  body: <Ex name="maps/sub-change" />,
};

export const walkMapDoc: LessonDoc = {
  lead: "Loop over (key, value) pairs, or over keys() or values() alone.",
  simple: <>Going through a map is like reading a notebook of names and numbers from the first page to the last, in the order they were written: take each name with its number, or only the names, or only the numbers.</>,
  minutes: 2,
  body: <Ex name="maps/sub-walk" />,
};

// ---------- Sets ----------

export const setsDoc: LessonDoc = {
  lead: "A set holds each value once. Adding a value it already has changes nothing.",
  simple: <>A set is like a guest book where each person signs only once: however often someone visits, their name appears one time. It quickly answers &quot;have I seen this before?&quot;</>,
  minutes: 3,
  before: ["collections/lists"],
  body: (
    <>
      <Ex name="sets/sub-dedupe" />
      <P>
        Five visits, three people: the set kept each name once, in the order first seen. Use a set to remove
        duplicates, or to ask quickly whether you have seen something.
      </P>
    </>
  ),
};

export const makeSetDoc: LessonDoc = {
  lead: "Set.of(…) makes a set; add and remove change it, so it must be a let.",
  simple: <>Make a set from a few values, then add or cross out values later, like names on a guest list. Because it changes, it must be a <C>let</C>.</>,
  minutes: 2,
  body: <Ex name="sets/sub-build" />,
};

export const checkSetDoc: LessonDoc = {
  lead: "contains asks whether a value is in the set; a loop visits the values in insertion order.",
  simple: <>Like checking the guest list at the door: is this name on it, yes or no? You can also read the whole list, in the order the names were added.</>,
  minutes: 2,
  body: <Ex name="sets/sub-walk" />,
};

// ---------- Tuples ----------

export const tuplesDoc: LessonDoc = {
  lead: "A tuple groups a few values, of any types, in brackets: (200, \"OK\").",
  simple: <>A tuple is like a small envelope holding a few things that belong together, of any kinds, such as a code number and a short message. It is handy when a function must give back two things at once.</>,
  minutes: 3,
  before: ["functions/functions"],
  body: (
    <>
      <Ex name="types/tuples" />
      <P>A tuple is handy for giving back two values from a function. Take it apart into names in one line:</P>
      <Ex name="types/tuple-parts" />
      <Note>
        Use tuples for short-lived groupings. When the same group is passed around, give its parts names with a{" "}
        <Link href="/docs/types/records/">record</Link>.
      </Note>
    </>
  ),
};
