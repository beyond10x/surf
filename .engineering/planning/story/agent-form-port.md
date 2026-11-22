---
format: aep.planning-md/3
id: story:agent-form-port
kind: story
status: active
title: Fill the form rows from the page and compute a submission body
summary: surf-page finds forms, form owners, names and options; a submission step in crates/surf builds the entry list, encodes it with surf-url and resolves the action.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-url-parser
- depends_on: story:agent-forms-spec
- depends_on: story:html-actionable-elements
- depends_on: story:html-document-port
- depends_on: story:runner-surf-cli-package
- depends_on: story:html-tree-tables
- depends_on: story:agent-follow-links
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 4}
---
## Context

`agent-forms-spec` specifies forms, the form owner, `name`, the options of a select (selectedness
only, decision D11), and a submission as a command that starts it and one that ends it. Nothing
fills those rows from a page and nothing computes a body. `epic:html-parser` computes the other row
values in `crates/page` (`html-actionable-elements`) and issues the ending commands of a parse from
a step of `crates/surf` (`html-document-port`); this story extends both, after them. Select parsing
is `html-tree-tables` (decision D12), and this story depends on it.

The document base URL is the function `agent-follow-links` writes in `crates/page` (decision D10);
this story calls it. The submission step is a new parameter of `Browser`, and `crates/session`
constructs `Browser`, so this story changes `crates/session` where it does. The binary is in
`crates/surf-cli` (decision D6): this story changes no binary behaviour; if a binary test there
prints rows this story adds, it is updated there.

Standards, from memory, to be confirmed: HTML Standard "form owner" (the `form` attribute and the
nearest `form` ancestor), "constructing the entry list" (disabled controls, unchecked checkboxes
and radios, buttons other than the submitter and controls without a name left out; a checkbox
without a value submits `on`), the `select` selectedness rules, the form submission algorithm
(empty `action` means the document address; `method` and `enctype` keywords and defaults; `GET`
replaces the query of the action URL, `POST` sends the body); URL Standard
"application/x-www-form-urlencoded" (`agent-url-parser`).

## Change

- `crates/page/src/forms.rs` (new) and one `pub mod` line in `crates/page/src/lib.rs`: forms in
  document order with `action`, `method`, `enctype`; the form owner and `name` of each control; the
  options of each select with their selectedness.
- `crates/surf/src/lib.rs`: the parse step carries the new rows in the ending commands; a
  submission step, a `Browser` parameter like the parse step, answers each started submission: the
  entry list from the rows of the document (values as filled), encoded with `surf-url`, the action
  resolved with `crates/page` `document_base_url`; `GET` puts the entries in the query, `POST` in
  the body as `application/x-www-form-urlencoded`; an action that does not resolve, an `enctype`
  other than urlencoded, and a resolved scheme other than `http:` end the submission with a reason.
  `conformance/src/target.rs`: a submission step that never answers. `crates/session/src/`: the
  default submission step where it constructs `Browser`.
- The counted cases, in `crates/page/tests/forms.rs` and `crates/surf/tests/forms.rs`:
  1. a control owned through the `form` attribute; 2. through nesting; 3. a control in no form;
  4. option selectedness of a `select` without `multiple`; 5. the entry list of a form with a text
  field, a password, a checked and an unchecked checkbox, a radio group and a select;
  6. a disabled control left out; 7. the submitter included and another submit button not;
  8. `GET` puts the entries in the query; 9. `POST` with body `user=ada&remember=on` for a text
  field `user` = `ada` and a checked checkbox `remember`; 10. an empty `action`: the document
  address; 11. a relative `action` under a `base`; 12. `multipart/form-data`: ended with a reason;
  13. `action="javascript:x"`: ended with a reason; 14. `action="file:///x"`: ended with a reason;
  15. names and values with `&`, `=`, space and bytes outside ASCII encoded as the standard says.

Surfaces written: `crates/page/src/forms.rs`, `crates/page/src/lib.rs` (one line),
`crates/page/tests/forms.rs`, `crates/page/tests/main.rs` (one `mod` line), `crates/surf/src/lib.rs`, `crates/surf/tests/forms.rs`, `crates/surf/tests/main.rs` (one `mod` line),
`conformance/src/target.rs`, `crates/session/src/`.

## Acceptance

`cargo test -p surf-page -p surf` passes with the 15 numbered cases, each a test named after its
case, case 9 producing the body `user=ada&remember=on`, and `slot task check` exits 0.

## Out of scope

The session commands (`agent-fill-pick`, `agent-press-submit`); `multipart/form-data`,
`text/plain`; file upload; `formaction`, `formmethod` and `formenctype` on a submitter (named in the
crate documentation as not taken).
