---
format: aep.planning-md/3
id: story:scripts-dom-collection-names
kind: story
status: draft
title: An HTMLCollection answers its elements by id and name as properties
summary: collection["id"] and collection.name answer the element namedItem answers, as the DOM's supported property names of an HTMLCollection say, instead of undefined.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-dom-reading
revision: 1
---
## Context

`story:scripts-dom-reading` gives a script `HTMLCollection.namedItem` and the index
properties of a collection, and names one deviation in the notes of the `HTMLCollection` rows
of `crates/webidl/selection.txt`: a collection has no named properties, so `collection["id"]`
is undefined where a browser answers the element. A collection there is a plain object whose
index properties the glue writes, refilled all or nothing when the getter is read after a
write to a child list (its decision 17). The DOM gives an `HTMLCollection` supported property
names: the ids of its elements and the `name` attributes of its HTML elements, in tree order,
without duplicates, and not enumerable (`[LegacyUnenumerableNamedProperties]`). A named
property never hides a property of the prototype (`length`, `item`, `namedItem`), and an index
wins over a name.

How many real pages read a collection by name is not measured. `story:scripts-conformance`
gives a count over real pages; this story is ordered by that count.

## Change

- The glue writes, with the index properties, one non-enumerable own property per supported
  name of the collection, by the DOM's rule above, with define and never set.
- The refill removes the names of the old list and writes those of the new one under the
  all-or-nothing rule of the reading story; a failed refill leaves the old names whole.
- The steps: one per element, and one per 64 bytes of each id and name read, charged before
  the work.
- If the pinned swell tag holds a host object with named property hooks by then (its
  requirements for live collections), the story is built on that instead and says so.

## Acceptance

1. `c["a"]` is the first element of the collection whose id is `a` or, for an HTML element,
   whose `name` is `a`, and `c["a"] === c.namedItem("a")`.
2. `Object.keys(c)` holds the indices only; `"a" in c` is true.
3. A name `length`, `item` or `namedItem` on an element does not change what those members
   answer; a name `0` does not change `c[0]`.
4. After a write that removes the element, the getter's next read answers a collection
   without the name; after a failed refill the old names and the old indices agree.
5. The notes of the `HTMLCollection` rows no longer name the deviation, and the test that
   pinned `undefined` asserts the element.
6. The folder's bounds hold: no test over 60 s, the test step within 120 s, a test over 5 s
   named with its time, no Python, no model call and no network request in a test.

## Out of scope

`document.all`, the named properties of `Window`, of a form and of `document`, which are other
interfaces. A live collection (swell's later slice).
