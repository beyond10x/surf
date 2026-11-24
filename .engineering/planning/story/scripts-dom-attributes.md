---
format: aep.planning-md/3
id: story:scripts-dom-attributes
kind: story
status: draft
title: The 14 attribute members, held to the attribute and stored-byte limits
summary: Element's id, className, slot and the attribute operations as the DOM Standard says, each write held to the parser's limits, tested member by member.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
scope:
- confidence: cited
  path: crates/script/src/dom/attributes.rs
- confidence: inferred
  path: crates/script/tests/dom_attributes.rs
revision: 3
---
## Context

The epic's tenth candidate (lines 467-468) and the slice "Attributes, 14" (line 692-694):
`Element`'s `id`, `className`, `slot`, `hasAttributes`, `getAttributeNames`, `getAttribute`,
`getAttributeNS`, `setAttribute`, `setAttributeNS`, `removeAttribute`, `removeAttributeNS`,
`toggleAttribute`, `hasAttribute`, `hasAttributeNS`. Every write goes through the public write
surface of `story:html-script-hook`, held to the attribute limit (`max_attributes`, 1,024 by
default) and the stored-byte limit (`crates/html/src/limits.rs:28-43`).

## Change

- `crates/script/src/dom/attributes.rs`: the 14 members, with the DOM Standard's exceptions
  (`InvalidCharacterError` for a name that is not valid, `NamespaceError`), names lower-cased for
  an HTML element in an HTML document as the standard says.
- `crates/script/tests/dom_attributes.rs`.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s and no test
runs over 60 s; no model call and no network request in a test; each case parses a few lines and
makes one realm; no Python; no generator with a committed output is added (one that were would carry a digest of its sources, a fast check in the gate and full regeneration in the lane `slow`); a build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s is named.

1. A named case per member (14), over a parsed tree, comparing the element's attributes after the
   script with what the DOM Standard says, and each exception case named by its exception.
2. `setAttribute` past the attribute limit, and past the stored-byte limit, ends the script with
   that limit as the note's Q4 says, and the element keeps its attributes.
3. `a.setAttribute("href", "/next")` on an `a` of a parsed tree: the element's attribute in the
   `surf-html` tree reads `/next`.
4. `git diff --stat` names only the files of the Change.

## Out of scope

`attributes`, `Attr` and `NamedNodeMap` (generated to throw); `classList`; the example of a link
built by script in a session (`story:scripts-document-port`, which has mutation and attributes).

## Why `epic:scripts-in-a-page`

Example 2 of the epic sets `href` with `setAttribute`.
