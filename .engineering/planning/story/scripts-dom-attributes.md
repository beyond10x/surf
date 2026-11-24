---
format: aep.planning-md/3
id: story:scripts-dom-attributes
kind: story
status: draft
title: The 14 attribute members, held to the attribute and stored-byte limits
summary: Element's id, className, slot and the attribute operations as the DOM Standard says, each write over the attribute or stored-byte limit ending the script through the generated limit result, the TrustedType deviation tested, 14 cases asserted equal to the slice's rows.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
scope:
- confidence: cited
  path: crates/script/src/dom/attributes.rs
- confidence: cited
  path: crates/script/tests/dom_attributes.rs
revision: 11
---
## Context

The epic's tenth candidate (lines 467-468) and the slice "Attributes, 14" (lines 692-694):
`Element`'s `id`, `className`, `slot`, `hasAttributes`, `getAttributeNames`, `getAttribute`,
`getAttributeNS`, `setAttribute`, `setAttributeNS`, `removeAttribute`, `removeAttributeNS`,
`toggleAttribute`, `hasAttribute`, `hasAttributeNS`. Every write goes through the public write
surface of `story:html-script-hook`, held to the attribute limit (`max_attributes`, 1,024 by
default) and the stored-byte limit (`crates/html/src/limits.rs:28-43`).

`story:webidl-bindings` lists this slice's rows in `crates/webidl/selection.txt`: those 14 and
none of the returned interfaces (the 7 attribute operations of `ProcessingInstruction` are
`throws`), with the deviation that `TrustedType` is read as `DOMString` on `setAttribute` and
`setAttributeNS`. Every trait method answers `DomResult`, whose `DomEnd::Limit` ends the script
with the parser limit it reached (decision 9 of that story).

## Change

- `crates/script/src/dom/attributes.rs`: the 14 members, with the DOM Standard's exceptions
  (`InvalidCharacterError` for a name that is not valid, `NamespaceError`) as `DomEnd::Throw`,
  names lower-cased for an HTML element in an HTML document as the standard says, and a write
  over the attribute or stored-byte limit answered as `DomEnd::Limit`.
- `crates/script/tests/dom_attributes.rs`: the module `dom_attributes`, declared by
  `story:webidl-bindings`.
- In the last commit, the public surfaces of acceptance 6.

## Acceptance

1. A named case per member, over a parsed tree, comparing the element's attributes after the
   script with what the DOM Standard says, and each exception case named by its exception; a test
   asserts that the set of case names equals the rows of `selection.txt` with the slice
   `attributes`: 14.
2. `setAttribute` past the attribute limit, and past the stored-byte limit, ends the script with
   the end `limit: attributes` (then `limit: stored-bytes`), also inside `try { ... } catch {}`,
   and the element keeps its attributes.
3. `a.setAttribute("href", "/next")` on an `a` of a parsed tree: the element's attribute in the
   `surf-html` tree reads `/next`; `a.setAttribute("x", { toString() { return "y" } })` sets `y`
   (the `TrustedType` deviation).
4. A member of another slice still throws `NotSupportedError` naming the member.
5. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
6. Public surfaces: the story names every public surface its change makes false (a badge, a
   `README.md` line, a website page, a count) and changes it in its last commit, every number
   from the log of a gate run on its tip, which the coordinating agent orders. For this story,
   lines on `main` at `29945f5` (2026-11-24): the test count of `README.md:11` and `:285-286` and `docs/badges/make.sh:38` with
   `docs/badges/tests.svg`; the gate block of `README.md:584-625` and of
   `website/docs/how-it-is-built.md:40-81`.
7. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
   call and no network request in a test; each case parses a few lines and makes one realm; no
   Python; no generator with a committed output is added; a build or test job in the pool ends
   within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s
   is named.

## Out of scope

`attributes`, `Attr` and `NamedNodeMap` (generated to throw); `classList`; the example of a link
built by script in a session (`story:scripts-document-port`).

## Placement

After `story:html-script-hook` and `story:scripts-dom-reading`; beside
`story:scripts-dom-mutation` and `story:scripts-dom-selectors` (each writes its own file).

## Why `epic:scripts-in-a-page`

Example 2 of the epic sets `href` with `setAttribute`.
