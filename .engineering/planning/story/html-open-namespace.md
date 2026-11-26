---
format: aep.planning-md/3
id: story:html-open-namespace
kind: story
status: draft
title: Store an attribute namespace outside the parser's six
summary: The tree holds a namespace as one of six values, so setAttributeNS with any other namespace throws NotSupportedError, a named deviation of story:scripts-dom-attributes; this draft is the story that lets the tree store such a namespace, bounded and charged, and removes the deviation. In no wave.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-dom-attributes
scope:
- confidence: inferred
  path: crates/html/src/tree.rs
- confidence: inferred
  path: crates/script/src/dom/attribute_reads.rs
- confidence: inferred
  path: crates/script/src/dom/attribute_writes.rs
- confidence: inferred
  path: crates/webidl/selection.txt
revision: 3
---
## Context

The tree of `crates/html` stores the namespace of an element and of an attribute as one of six
values: the enum `Namespace` (`crates/html/src/tree.rs:77-92 @ 83e6b8a`), whose `url` gives the
six addresses (`:94-106`). An attribute holds `namespace: Option<Namespace>` (`:111-113`). The
parser makes no other namespace, so the type fits every tree a parse can build.

A script can ask for another one. The DOM Standard's `setAttributeNS(namespace, qualifiedName,
value)` takes any string as the namespace, and so do `getAttributeNS`, `hasAttributeNS`,
`removeAttributeNS` and `createElementNS`. `story:scripts-dom-attributes` cannot store such an
attribute, so it throws `NotSupportedError` for a namespace outside the six and names that as a
deviation of surf (a browser stores the attribute). The reads need no deviation: a namespace
that is none of the six matches no attribute.

This draft is the story that lifts the deviation. It is not planned into a wave: it waits for a
page or a consumer that needs a seventh namespace.

## Decisions to take when the story is revised

1. The form of an open namespace in the tree: a seventh variant that holds the address
   (`Namespace::Other`, an interned string or an index into a table of the tree), so that the six
   stay values without an allocation and `Namespace` stays `Copy` if the code relies on it (read
   every `match` on `Namespace` first: the tree builder's adjustments of foreign attributes, the
   serializer, the accessibility and page-text crates).
2. The limits: the bytes of a namespace address count toward the stored-byte limit of the tree
   and are charged to the script that writes them, at the price of a byte the DOM slices use; the
   count of distinct addresses of a tree is bounded.
3. Whether an element in another namespace is in scope (`createElementNS`), or attributes only.
   `story:scripts-dom-mutation` makes `createElementNS` with a prefix or a namespace outside the
   six throw `NotSupportedError`; this story is the one that can lift that.
4. What the serializer and the text extraction do with an attribute or an element whose
   namespace is not one of the six.

## Change

- `crates/html/src/tree.rs`: the open namespace, its storage, its bytes in the stored-byte
  count, the write surface that takes it.
- `crates/script/src/dom/attribute_reads.rs` and `attribute_writes.rs`: the mapping of an
  address to a namespace no longer refuses a seventh; the four attribute members with `NS` store,
  find and remove such an attribute.
- `crates/webidl/selection.txt`: the note of the deviation on `setAttributeNS` goes.
- The epic's list of deviations loses the line.

## Acceptance

1. `el.setAttributeNS("urn:x", "p:a", "1")` stores an attribute that
   `el.getAttributeNS("urn:x", "a")` reads as `"1"`, that `el.hasAttributeNS("urn:x", "a")`
   finds and that `el.removeAttributeNS("urn:x", "a")` removes; none throws
   `NotSupportedError`.
2. A script that writes attributes in many distinct namespaces ends at a limit of the tree with
   the limit's word, and each write is charged before it is done.
3. Every tree a parse builds is byte for byte what it was (the conformance suites pass with
   their totals unchanged).

## Out of scope

Attribute nodes (`Attr`, `NamedNodeMap`): the epic excludes them. XML documents.
