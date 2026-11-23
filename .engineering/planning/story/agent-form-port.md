---
format: aep.planning-md/3
id: story:agent-form-port
kind: story
status: implemented
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
  path: crates/page/tests/adversary_forms.rs
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
  path: crates/surf/tests/adversary_forms.rs
- confidence: cited
  path: crates/surf/tests/forms.rs
- confidence: cited
  path: crates/surf/tests/main.rs
- confidence: cited
  path: generated/rust
- confidence: cited
  path: spec/domains/document.yaml
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-23T04:21:57Z", actor: "human:operator", revision: 14, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:coordinator"}
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
- `crates/page`: the form handles, in a new function `surf_page::form_handles(tree)` that gives
  `F<n>` to each form, counted from 1 in document order, the order of the `Forms` view
  (`spec/domains/document.yaml:1312`). `surf_page::handles` (`crates/page/src/lib.rs:368`, `l<n>`,
  `b<n>`, `f<n>`) stays unchanged: the outline and the Markdown read it
  (`crates/page-text/src/outline.rs:30`, `markdown.rs:116`), and page text prints no `@F<n>`.
  `story:agent-press-submit` only prints the form handles.
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
  field `user` = `ada`, a password `pw` = `s3cret`, a checked checkbox `remember` without a
  `value`, an unchecked checkbox `news`, a radio group `size` with `s` and `m` (`m` checked) and
  a select `color` with the option `red` selected, in that tree order, is
  `user=ada&pw=s3cret&remember=on&size=m&color=red`;
  6. a disabled control left out; 7. the form of case 9 with two submit buttons
  `<button name="go" value="1">` (the submitter) and `<button name="other" value="2">`: the
  body is `user=ada&remember=on&go=1`;
  8. `GET` with `action="results?x=1"` and a field `q` = `lighthouse` navigates to
  `<document base>/results?q=lighthouse`: the entries replace the action's query;
  9. `POST` with body `user=ada&remember=on` for a text
  field `user` = `ada` and a checked checkbox `remember`; 10. an empty `action`: the document
  address; 11. a relative `action` under a `base`; 12. `multipart/form-data`: ended
  `UnsupportedEnctype`; 13. `action="javascript:x"`: ended `UnsupportedScheme`;
  14. `action="file:///x"`: ended `UnsupportedScheme`; 15. a field named `n&a=me` with the
  value `a b é` gives the body `n%26a%3Dme=a+b+%C3%A9`;
  16. a submit button with no form owner: ended `NoFormOwner`; 17. a `type=reset` and a
  `type=button` button pressed through `Press`: ended `NotSubmitButton`; 18. a `required` text
  field left empty: ended `InvalidControls`; 19. an `action` that does not parse as a URL
  (`http://[x`): ended `InvalidAction`; 20. `method="dialog"`: ended `DialogMethod`;
  21. `form_handles` of a page with two forms gives `F1` and `F2` in document order, and
  `handles` of the same page gives what it gave before (`l1`, `b1`, `f1`, no `F`).

  Where the expected values come from: the bytes of cases 9 and 15 from the serializer of
  `crates/url` (`crates/url/src/form_urlencoded.rs:9-19`: `&` between pairs, `=` inside; the
  form-urlencoded set of `crates/url/src/percent.rs:45-47`, a space as `+`, `:64-73`), read in the
  repository. The entry-list rules of cases 5, 7 and 8 (a checkbox without `value` submits `on`,
  only the submitter's button is an entry, `GET` replaces the action's query) are the HTML
  Standard's "constructing the entry list" and form submission algorithm, which are not in the
  repository: to be confirmed by the implementor against the standard's text (see Context).

  Case 17 drives `Press` from `crates/surf/tests` without a helper of `crates/session`: the
  pattern of `opened` in `crates/surf/tests/forget_delivered.rs:41-71` builds a `Browser` with
  `ComponentPorts::with_caller`, opens a session through `agent_mut().open_session` and sets the
  caller; `Press` takes the button's `element_id` from the `Elements` view.

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
`text/plain`; file upload; `formenctype` on a submitter (named in the crate documentation as not
taken); `formaction` and `formmethod` on a submitter, which `agent-press-submit` takes.

- A `required` checkbox, radio button or `select` with `multiple` is not checked: only a
  `required` control with an empty value ends `InvalidControls`.
- Values of the Range, Color, Number and date and time states are not sanitised as the HTML
  Standard's value sanitization algorithms do: the entry holds the value as filled.
- A `mailto:` or `about:` action ends `UnsupportedScheme`, where the HTML Standard gives each a
  submission of its own.

## As decided after the adversary's pass

The adversary's cases on the built port (`265a30a` on `adv/form-port`:
`crates/surf/tests/adversary_forms.rs`, `crates/page/tests/adversary_forms.rs`) showed four
defects red and one note. Decided by the coordinator; line numbers are those at `58ed9b4`:

1. An address over `surf_fetch::MAX_ADDRESS_BYTES` (65,536 bytes, `crates/fetch/src/lib.rs:572`)
   ends the submission in the port itself: the `GET` address built from the entry list
   (`crates/surf/src/lib.rs:515-518`, `with_query` at `:642`) and the action of a `POST`
   (`:531`). The end reason is a new variant of the specification, `AddressTooLong`, beside
   `InvalidAction` and the others in `surf.document.SubmissionEndReason`
   (`spec/domains/document.yaml:105-121`), so the enum has eight; nothing is navigated and no
   request is made. The port is public (`start_submission`, then `run`), and the session's
   refusal `address-too-long` (`story:agent-press-submit`, case 18) covers only the session. The
   specification change brings `spec/domains/document.yaml`, `generated/rust/`, the end-reason
   names of `conformance/src/target.rs` (`end_reason` and `end_reason_named`, `:862` and `:874`
   at `d5d7426`) and, when the suite's counts change, the files that state them (`AGENTS.md`
   "The gate's numbers", `README.md`, `docs/badges/make.sh` and its svg,
   `website/docs/{status,what-works,principles,how-it-is-built}.md`, `Taskfile.yml`
   `REFUSALS_MAX`). Red today: `a_get_submission_makes_no_address_longer_than_64_kib`,
   `a_submission_to_an_action_longer_than_64_kib_is_not_navigated`.
2. A `required` `select` whose submitted value is empty (its placeholder option selected, or no
   option selected) ends `InvalidControls`, as this story's sentence says ("only a `required`
   control with an empty value is checked"). Red today:
   `a_required_select_left_on_its_placeholder_is_not_submitted`.
3. The `Debug` output of `Filled` (`crates/surf/src/lib.rs:393`), `Submission` (`:375`) and of
   the body of `Submit::Navigate` (`:407`) prints a value's length, never its bytes. The address
   in `Submit::Navigate` stays whole: a `GET` with a filled password is refused by the session
   (`password-in-query`, `story:agent-press-submit`). Red today:
   `the_submission_steps_input_prints_no_password`.
4. `form_handles` (`crates/page/src/forms.rs:163`) numbers the forms without building the
   options and their text; it gives the same `F<n>` as before, so
   `adversary_forms_probe_form_handles_name_the_submitted_form` stays green.
5. Tests: no test asserts wall time. The three probes on pages of 100,000 forms, options or
   controls (`adversary_forms_probe_100k_forms` and `adversary_forms_probe_100k_options` in
   `crates/surf/tests/adversary_forms.rs`, `adversary_forms_probe_100k` in
   `crates/page/tests/adversary_forms.rs`; 7 s to 14 s each) go to a size at which each runs
   under 3 s alone, and keep their assertions on counts; `adversary_forms_probe_one_mib_value`
   loses its assertion on time too. The size is measured by the implementor and named in the
   unit report with each probe's time.
6. Named as differing from the HTML Standard and not promised: see "Out of scope".
