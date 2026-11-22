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
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: conformance/Cargo.lock
- confidence: cited
  path: conformance/src/target.rs
- confidence: inferred
  path: conformance/tests
- confidence: inferred
  path: crates/page/Cargo.toml
- confidence: cited
  path: crates/page/src/forms.rs
- confidence: cited
  path: crates/page/src/lib.rs
- confidence: cited
  path: crates/page/tests/forms.rs
- confidence: cited
  path: crates/page/tests/main.rs
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: inferred
  path: crates/surf-cli/src/main.rs
- confidence: inferred
  path: crates/surf/Cargo.toml
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests/forms.rs
- confidence: cited
  path: crates/surf/tests/main.rs
revision: 9
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
- `crates/page`: the form handles. `surf_page::handles` (`crates/page/src/lib.rs:368`, which gives
  `l<n>`, `b<n>` and `f<n>` today) also gives `F<n>` to each form, counted from 1 in document
  order, the order of the `Forms` view (`spec/domains/document.yaml:1312`). `story:agent-press-submit`
  only prints them.
- `crates/surf/src/lib.rs`: the parse step issues the new rows before `FinishParsing`
  (`crates/surf/src/lib.rs:1085`), which takes no rows: each form as `AddForm`
  (`spec/domains/document.yaml:561`), each option as `AddOption` (`:617`), and each control's
  `name` and `form_id` in its `AddElement` (`None` today, `crates/surf/src/lib.rs:1076-1079`). A
  submission step, a `Browser` parameter like the parse step, answers each started submission:
  the entry list from the rows of the document (values as filled), encoded with `surf-url`, the
  action resolved with `crates/page` `document_base_url`; `GET` puts the entries in the query,
  `POST` in the body as `application/x-www-form-urlencoded`; it ends the submission with
  `SubmitForm` (`spec/domains/document.yaml:1018`) when it navigates and with
  `AbandonSubmission` (`:1072`) and one of the seven `SubmissionEndReason` variants (`:105-121`)
  when it does not: `NoFormOwner`, `NotSubmitButton`, `InvalidControls` (only a `required`
  control with an empty value is checked; the other constraints of constraint validation are not
  built), `InvalidAction`, `UnsupportedScheme`, `UnsupportedEnctype` and `DialogMethod`, each
  with its case below. `conformance/src/target.rs`: a submission step that never answers.
  `crates/session/src/`: the default submission step where it constructs `Browser`.
- The counted cases, in `crates/page/tests/forms.rs` and `crates/surf/tests/forms.rs`:
  1. a control owned through the `form` attribute; 2. through nesting; 3. a control in no form;
  4. option selectedness of a `select` without `multiple`; 5. the entry list of a form with a text
  field, a password, a checked and an unchecked checkbox, a radio group and a select;
  6. a disabled control left out; 7. the submitter included and another submit button not;
  8. `GET` puts the entries in the query; 9. `POST` with body `user=ada&remember=on` for a text
  field `user` = `ada` and a checked checkbox `remember`; 10. an empty `action`: the document
  address; 11. a relative `action` under a `base`; 12. `multipart/form-data`: ended
  `UnsupportedEnctype`; 13. `action="javascript:x"`: ended `UnsupportedScheme`;
  14. `action="file:///x"`: ended `UnsupportedScheme`; 15. names and values with `&`, `=`, space
  and bytes outside ASCII encoded as the standard says;
  16. a submit button with no form owner: ended `NoFormOwner`; 17. a `type=reset` and a
  `type=button` button pressed through `Press`: ended `NotSubmitButton`; 18. a `required` text
  field left empty: ended `InvalidControls`; 19. an `action` that does not parse as a URL
  (`http://[x`): ended `InvalidAction`; 20. `method="dialog"`: ended `DialogMethod`;
  21. `handles` of a page with two forms and a link, a button and a field: `F1` and `F2` in
  document order beside `l1`, `b1`, `f1`.

Surfaces written: `crates/page/src/forms.rs`, `crates/page/src/lib.rs`,
`crates/page/tests/forms.rs`, `crates/page/tests/main.rs` (one `mod` line), `crates/surf/src/lib.rs`,
`crates/surf/tests/forms.rs`, `crates/surf/tests/main.rs` (one `mod` line),
`conformance/src/target.rs`, `crates/session/src/`.

## Acceptance

`cargo nextest run -p surf-page -p surf` passes with the 21 numbered cases, each a test named
after its case, case 9 producing the body `user=ada&remember=on`, and `slot task check` exits 0.
No test runs over 60 s, and the test step ends within 120 s.

## Out of scope

The session commands (`agent-fill-pick`, `agent-press-submit`); `multipart/form-data`,
`text/plain`; file upload; `formaction`, `formmethod` and `formenctype` on a submitter (named in the
crate documentation as not taken).
