---
format: aep.planning-md/3
id: story:agent-press-submit
kind: story
status: implemented
title: press submits a form and the session lands on the answer
summary: press <handle> on a submit button submits through the document and navigates with GET or POST; the test server receives the encoded entries.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-fill-pick
- depends_on: story:agent-session-events
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: README.md
- confidence: cited
  path: cli/surf-agent.yaml
- confidence: cited
  path: crates/agent-tasks/status.txt
- confidence: cited
  path: crates/agent/src/cli.rs
- confidence: inferred
  path: crates/agent/src/wire.rs
- confidence: inferred
  path: crates/agent/tests/drift.rs
- confidence: cited
  path: crates/answer/src/lib.rs
- confidence: inferred
  path: crates/answer/tests
- confidence: cited
  path: crates/fetch/src
- confidence: cited
  path: crates/fetch/tests
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/tests
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surfd/src/daemon.rs
- confidence: cited
  path: docs/agent-answer.md
- confidence: cited
  path: docs/design/agent-interface.md
- confidence: inferred
  path: generated/rust
- confidence: inferred
  path: spec/domains/agent.yaml
- confidence: inferred
  path: website/docs/drive-from-an-agent.md
revision: 23
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-23T04:21:54Z", actor: "human:operator", revision: 23, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Step 4 of design section 9: `press` activates a button; with no script engine the only activation
is form submission (design section 4). `agent-forms-spec` specifies `Press` and the submission;
`agent-form-port` computes its body, ends a submission with one of the eight reasons of the
specification (seven, and `AddressTooLong` by its section "As decided after the adversary's
pass") and gives forms their handles `F<n>`; `agent-fetch-post` sends `POST` and turns a
303 into `GET`. Design section 5.1 shows a button line naming what it submits, with the action
unquoted (`submits F1 post /session`, `docs/design/agent-interface.md:231`); this story quotes
it, like every page-derived field, and corrects that line.

`agent-form-port` is built (`58ed9b4`). What this story relies on of it, as built:

- A control's form owner is the form its `form` attribute names by ID, else its nearest
  ancestor `form` (`crates/page/src/forms.rs:14-18` at `58ed9b4`); a control the parser
  associated with a form it is not inside of has none (the same file, `:37-39`).
- The entry list is always encoded as UTF-8; a Hidden `input` named `_charset_` gives `UTF-8`,
  another Hidden `input` its `value` attribute, and an Image Button submitter gives
  `<name>.x=0` and `<name>.y=0` (`crates/surf/src/lib.rs:74-83` at `58ed9b4`). A Hidden `input`
  has no row and so no line and no handle (`crates/page/src/lib.rs:14-16` at `58ed9b4`); its
  value goes into the entry list all the same.
- `surf::Browser` has the type parameter `F = FormSubmitter` (`crates/surf/src/lib.rs:1119`),
  replaced with `Browser::with_submitter` (`:1200`); the session keeps the default, whose
  `surf::submit_form` (`:456`) ends a submission, and `conformance/src/target.rs:110` gives
  `Unsubmitted`, which never answers. This story adds no submission step.
- The session's bound of 256 row entries per request (`ELEMENT_ENTRIES_MAX`,
  `crates/session/src/lib.rs:246` at `58ed9b4`) counts `FormAdded` and `OptionAdded` with
  `ElementAdded`, and bounds only the entries written into the session log: the rows past it
  are left out of the log and counted by one `EventsLeftOut` line, not out of the document. A
  page with more rows than the bound still has every row, so every button has its `b<n>` line
  and handle (`surf_page::handles` over the whole tree), and every form its `F<n>`:
  `surf_page::form_handles` numbers `surf_page::forms::forms(tree).forms` in tree order
  (`crates/page/src/forms.rs:163-170` at `58ed9b4`), the same list the submission step reads
  (`crates/surf/src/lib.rs:359`, `Kept.forms` at `:1136`), so `F<n>` names the form the library
  submits.

## Change

- `crates/session`: `press <handle>` on a submit button issues `Press`; the answer shows the page
  the navigation committed at. The line of each submit button names what it submits:
  `submits F1 post "/session"`, the form's handle as `surf_page::form_handles` gives it (`F<n>`,
  a function `story:agent-form-port` added beside `surf_page::handles`, which stays unchanged;
  this story only prints it, and page text prints no `@F<n>`), the method, and the resolved action
  quoted like every page-derived field. The method and the action are the ones a `press` of that
  button submits with: the button's own `formmethod` and `formaction` when it has them (below).
  The action is shown by the rules of a link's shown target (`story:agent-follow-links`, "As
  decided after the adversary's pass"): never a userinfo; scheme, host and port whole, and the
  cut of 1,024 characters after them; a same-origin action whose path starts with `//` shown
  absolute; another same-origin action as its path. `docs/design/agent-interface.md:231` writes
  the action unquoted; this story corrects that line to `submits F1 post "/session"`.
- `formaction` and `formmethod` of the submitter: taken by this story in `surf::submit_form`
  (`crates/surf/src/lib.rs`, steps 4 and 5 of its "The submission step", `:84-88` at
  `58ed9b4`, which list them as not taken at `:98`). A submit button with a `formmethod`
  attribute submits with that method, and one with a `formaction` attribute submits to that
  action, resolved against the document base URL like the form's; a button without them takes
  the form's. The three address refusals below apply to the result. An empty `formaction` and a
  `formmethod` of an unknown value are to be confirmed by the implementor against the HTML
  Standard's form submission algorithm (the standard's text is not in the repository;
  `crates/surf/src/lib.rs`, `submit_form`, settles it, and its documentation states the rule
  taken). Cases 20 and 21.
- `no-default-action` is the session's refusal, before any command is issued, for a button of
  `type=button` and of `type=reset` (no reset is built: a reset button is refused by name, as a
  plain one is), until a script engine exists.
- The submission's address follows the three refusals `story:agent-follow-links` gives a link's
  target (its section "As decided after the adversary's pass"), decided by the session before a
  submission starts, with no event of a submission and no request, exit status 1. The address is
  the action (the submitter's `formaction`, else the form's `action`, else the document's
  address) resolved against the document base URL, and for `GET` that action with its query
  replaced by the entry list, as `surf::submit_form` builds it; the session computes it with the
  code the submission step uses, so the address checked is the address requested (how the two
  share it is to be confirmed by the implementor; `crates/surf/src/lib.rs`, `submit_form` at
  `:456` at `58ed9b4`, settles it):
  - an address over `surf_fetch::MAX_ADDRESS_BYTES` (65,536 bytes,
    `crates/fetch/src/lib.rs:572`), counted after the query is written: `did refused
    address-too-long`, case 18;
  - an address whose scheme surf does not fetch (surf fetches `http:` only,
    `crates/fetch/src/lib.rs:165-166`), `javascript:` and `mailto:` among them; the page
    stays: `did refused unsupported-scheme`, case 8;
  - from a page on a public address, an address on a loopback, private, link-local,
    unique-local, shared or unspecified address (`crates/fetch/src/lib.rs:100-107`): `did refused
    forbidden-destination`, with no connection; from a page on a local address the form is
    submitted as before, case 19.
  The three words come from `story:agent-follow-links`, whose fix adds them to the reason table,
  `RefusalReason` and its `ALL`, `refusal_status` and the `match` of `crates/surfd/src/daemon.rs`;
  this story adds no `RefusalReason` for them and uses those.
- The split between the session's refusals and the specification's end reasons, so the two do
  not overlap. Refused by the session before `Press`, so no submission starts:
  `no-default-action`, `disabled`, `password-in-query`, `address-too-long`,
  `unsupported-scheme`, `forbidden-destination`; when a form would end with an end reason and
  is refused too, the refusal is the answer. Ended by the specification's reason after the
  submission started (`SubmissionAbandoned.reason`, `spec/domains/document.yaml:105-121`, eight
  reasons with the `AddressTooLong` of `story:agent-form-port`), refused with its word, exit
  status 1; five are reached through the session and three are not:
  - `NoFormOwner`: `no-form-owner`, case 12;
  - `NotSubmitButton`: cannot be reached through the session, which refuses `type=button` and
    `type=reset` as `no-default-action` before `Press`; no word of its own and no case;
  - `InvalidControls`: `invalid-controls`, case 13;
  - `InvalidAction`: `invalid-action`, case 14: an action that does not parse gives the session
    no address to check, so the submission starts and ends with this reason;
  - `UnsupportedScheme`: cannot be reached through the session: the specification's set is every
    scheme but `http` and `https` (`crates/surf/src/lib.rs:89-90` at `58ed9b4`), the session
    refuses every scheme but `http`, so an address the specification would end with this reason
    is refused before; should it ever end a submission, the session answers the word
    `unsupported-scheme` of the session's refusal; no case of its own;
  - `UnsupportedEnctype`: `unsupported-enctype`, case 7;
  - `DialogMethod`: `dialog-method`, case 15;
  - `AddressTooLong`: cannot be reached through the session: the port ends a submission with it
    for an address over `surf_fetch::MAX_ADDRESS_BYTES`, the address the session refuses first
    as `address-too-long` (case 18) with the same limit, counted on the same address; should it
    ever end a submission, the session answers the word `address-too-long` of the session's
    refusal; no case of its own.
- `password-in-query` is the session's refusal, like `no-default-action` and not one of the
  specification's `SubmissionEndReason` values: `press` on a submit button whose form has method
  `GET` and whose entry list holds a control of `type=password` with a non-empty value is
  refused before any submission starts, `did refused password-in-query`, exit status 1 as the
  other session refusals, with no event of a submission and no request. A `GET` submission
  would put the value into the URL, which the event log writes as text (`FormSubmitted.url`,
  `RequestStarted.url` and the fields after them) and the `page` and `path` lines print; story
  `values-and-labels-before-fill` promises that a password value is printed nowhere. A `GET`
  form whose password control is empty or absent is submitted as before; a `POST` form is not
  affected (its body is logged as `<n> bytes`). The limit: a site whose sign-in form uses `GET`
  cannot be signed in to; the refusal names why. A filled value reaches no URL by `follow` or
  `go`: `SetValue` sets only the control's `value` (`spec/domains/document.yaml:785-786`),
  `follow` takes the link's `href` and `go` the address typed, so only a submission builds a
  URL from control values. The method is the one the submission uses, the submitter's
  `formmethod` included. Cases 16 and 17.
- `crates/answer`, `docs/agent-answer.md`: the `submits` part of a button line, with its `parse`
  beside its rendering (decision D20 of `docs/plan/agent-plan-revision-2.md`).
- The event entries exist already: `PressRequested`, `SubmissionStarted`, `FormSubmitted` (the
  body as `<n> bytes`, decision D22) and `SubmissionAbandoned` are mapped
  (`crates/session/src/entries.rs:250`, `:268`, `:271`, `:281`). This story adds no mapping;
  case 10 tests them.
- The `did` and `path` lines of `press`. `did press "<handle>"`: the handle quoted, as the caller
  typed it. The `path` entry takes the form `story:agent-follow-links` gives an action on an
  element (`spec/domains/agent.yaml:302-304`): `<n> press b1`, the handle bare (the design shows
  no `press` entry; `docs/design/agent-interface.md:223` shows `3 fill f1`, and this follows it).
  The reasons `no-default-action`, `password-in-query`, `disabled` (if `agent-fill-pick` has not
  added it) and the five submission words `no-form-owner`, `invalid-controls`,
  `invalid-action`, `unsupported-enctype` and `dialog-method` go into the reason table of
  `docs/agent-answer.md` with exit status 1, into `RefusalReason` and its `RefusalReason::ALL`
  (`crates/answer/src/lib.rs:150`), into `refusal_status` (`crates/session/src/lib.rs:924`),
  into the exhaustive `match` of `crates/surfd/src/daemon.rs:815-821`, and into a test;
  `unsupported-scheme`, `address-too-long` and `forbidden-destination` are there already
  (above). The session's word for each end reason is one `match` on `SubmissionEndReason`, with
  `UnsupportedScheme` to `unsupported-scheme` and `AddressTooLong` to `address-too-long`.
- `cli/surf-agent.yaml`, `crates/agent`: `press`, together, so the drift test holds.
- `crates/session/tests/`, the counted cases, against a server the test starts:
  1. a login form filled and pressed: the server reads `POST /session HTTP/1.1` with
     `Content-Type: application/x-www-form-urlencoded` and the body `user=ada&password=s3cret`,
     answers 303 to `/home`, and the answer shows the page at `/home`;
  2. the `GET` form of the `search` site (`crates/agent-tasks/sites/search/search.html:10-13`:
     `action="results"`, a field `q`, a button `Find`), `q` filled with `lighthouse` and pressed:
     the server reads `GET /results?q=lighthouse HTTP/1.1`, and the answer's `page` line ends in
     `/results?q=lighthouse"`;
  3. the button line of a form reads `submits F1 post "/session"`;
  4. `press` on a `type=button`: `no-default-action`; 5. on a `reset`: `no-default-action`;
  6. on a disabled submit button: `disabled`; 7. a `multipart/form-data` form with an `http:`
     action: `unsupported-enctype`;
  8. a form with `action="javascript:x"` and a form with `action="mailto:a@example.org"`, each
     pressed: status 1, `did refused unsupported-scheme`, the page stays, no submission entry
     in the event log and no request;
  9. forging: an action holding `"`, a line feed and a bidirectional control is printed quoted and
     escaped on the button line, with no extra line;
  10. `press_events_map_with_their_fields`, with `surfd` run in the test process (decision D29):
      after case 1 the session log holds the `PressRequested`, `SubmissionStarted` and
      `FormSubmitted` entries with their fields, none `unmapped`, the request body as
      `<n> bytes`, and `s3cret` in no entry;
  11. round trip: `surf_answer::parse` of the answer of case 3 and of case 9 gives the typed
      answer, and rendering that gives the same lines byte for byte;
  12. a submit button with no form owner: `no-form-owner`; 13. a `required` field left empty:
      `invalid-controls`; 14. an action that does not parse as a URL: `invalid-action`;
  15. a form with `method="dialog"`: `dialog-method`;
  16. `password_in_query_is_refused`: a `GET` form with a text field and a password control,
      the password filled with `s3cret-get`, pressed: status 1, `did refused password-in-query`;
      the server's log shows no request, and no entry of the event log (with `surfd` run in the
      test process, decision D29) holds a byte of `s3cret-get`;
  17. the same `GET` form with the password control left empty, pressed: submitted, the server
      reads `GET` with the entries in the query;
  18. `address_too_long_is_refused`: a `GET` form with `action="/results"` and a Hidden `input`
      `q` whose `value` attribute is 65,536 `a` characters (the action alone is short; the
      address after the query is written is over 65,536 bytes), pressed: status 1, `did refused
      address-too-long`, no request;
  19. `forbidden_destination_is_refused`: a form on a page at a public address with
      `action="http://127.0.0.1:<port>/session"`, pressed: status 1, `did refused
      forbidden-destination`, the server on `127.0.0.1` reads no request; the same form on a
      page at `127.0.0.1`, pressed: submitted, the server reads `POST /session`. How the test
      serves a page at a public address is to be confirmed by the implementor: the pattern of
      the follow-links fix's case for `forbidden-destination` settles it, and
      `crates/fetch/tests/redirects.rs` routes `203.0.113.5:80` to the test's server with
      `TestNetwork` (case 21, `:656-687`);
  20. `submitter_formaction_and_formmethod`: a form with `action="/session" method="post"`, a
      field `user` filled with `ada`, a button `Save` and a button `Draft` with
      `formaction="/draft" formmethod="get"`: the line of `Draft` reads `submits F1 get
      "/draft"`, the line of `Save` `submits F1 post "/session"`; `press` on `Draft`: the server
      reads `GET /draft?user=ada HTTP/1.1`;
  21. a submit button with `formaction="mailto:a@example.org"` in a form whose `action` is
      `/session`, pressed: status 1, `did refused unsupported-scheme`, no request;
  22. the shown action: a form with `action="http://user:pw@127.0.0.1:<port>/session"` shows
      `submits F1 post "/session"` with no `user` and no `pw` on any line of the answer; a form on
      the same origin with `action="http://127.0.0.1:<port>//x"` shows the action absolute,
      `"http://127.0.0.1:<port>//x"`;
  23. `buttons_share_one_resolved_action`: the answer of `look` on a page of 1,000 submit
      buttons in one form whose action is 10,000 bytes holds 1,000 button lines, each with its
      `submits` part, and the test runs under 3 s alone (decision 3 of "As decided after the
      adversary's pass"; no assertion on wall time).

- `crates/agent-tasks/status.txt`, the round's measure: `search pass` (line 4). A task listed
  `pending` that passes makes the report print `raise: search` (`crates/agent-tasks/src/report.rs:9-14`),
  so the line is changed in this story.

Surfaces written: `crates/session/`, `crates/answer/`, `docs/agent-answer.md`,
`cli/surf-agent.yaml`, `crates/agent/`, `crates/surfd/src/daemon.rs`,
`docs/design/agent-interface.md` (line 231), `crates/agent-tasks/status.txt`,
`crates/surf/src/lib.rs` (`submit_form`: `formaction`, `formmethod`, and the address the session
checks), `crates/fetch/src` and `crates/fetch/tests` (the destination rule on the first request,
"As decided after the adversary's pass").

## For README.md (handed over in the unit report, decision D7)

- "Driving surf from an agent": "`press b1` submits the form of a submit button; the answer shows
  the page the server answered with. A button that only a script would act on is refused."

## Acceptance

`cargo nextest run -p surf-session` passes with the 23 numbered cases, each a test named after its
case, and with the adversary's tests named in "As decided after the adversary's pass";
`cargo nextest run -p surf -p surf-fetch` passes with `agent-form-port`'s cases unchanged, case 1 reading the request line `POST /session HTTP/1.1` and the body
`user=ada&password=s3cret` and answering with the page at `/home`. The round's measure:
`crates/agent-tasks/status.txt` reads `search pass` (line 4 reads `search pending
story:agent-fill-pick story:agent-press-submit` today), `tasks::every_task`
(`crates/agent-tasks/tests/tasks.rs:74`) passes, and `slot task agent-tasks-report` prints
`task search pass` with `2 connections` (`crates/agent-tasks/tasks.txt`, task `search`) and no
`raise:` line (`crates/agent-tasks/src/report.rs:9-14`); today `search` fails. No test runs over
60 s, and the test step ends within 120 s.

## Out of scope

Scripts and elements a script made clickable; `changed` (`agent-change-and-log`). Not taken,
as `agent-form-port` left them (`crates/surf/src/lib.rs:98-103` at `58ed9b4`), one sentence
each:

- `formenctype`: a submitter's `formenctype` is not read; the form's `enctype` decides.
- `formnovalidate`: a submitter's `formnovalidate` is not read; a form with an empty `required`
  field ends `invalid-controls` though that button would skip the check.
- `formtarget`: a submission navigates the tab of its document; no other target is taken.
- `dirname`: no entry with the text's direction is added for a control with `dirname`.
- The `submit` event: without a script engine nothing listens to it, so nothing can cancel a
  submission.
- "cannot navigate": not checked; `press` acts on the document the tab shows now, whose
  handles are the only ones the session holds.

## As decided after the adversary's pass

The adversary's cases on the built story (code `b11ed6c`; cases at `8626f23` on
`adv/press-submit`: `crates/session/tests/adversary_press.rs`, 22 cases, 2 red). Decided by the
coordinator; line numbers are those at `b11ed6c`:

1. The destination rule applies to the address a request connects to, not only to the host's
   text. Today the session refuses `forbidden-destination` by reading the host
   (`crates/session/src/lib.rs:1208`; `host_is_local` at `:1829`, an IP literal or a
   `localhost` name at `:1837-1840`), and the fetch applies its rule to redirects only
   (`crates/fetch/src/lib.rs`, "Destinations": "The host of every request of a fetch is resolved
   once ... the addresses resolved are the ones checked and then dialed"). Decided: for `follow`
   and `press` from a page that counts as public, the fetch applies that same rule to the first
   request. The session tells the fetch that the request comes from a public page; a host that
   resolves to a local address, any one of those resolved, is refused before a connection is
   made and before a byte is sent, and the session answers `did refused
   forbidden-destination`. The session's text check stays the first check. `go` with an address
   the agent typed is not changed. Which page counts as public stays as decided: by its address
   text, local only when its host is a local IP address or `localhost`. The path, the page and
   the filled values after this refusal are as after the other `forbidden-destination`; how the
   session keeps them so when the navigation has started and its fetch fails is the
   implementor's, with the reason in the unit report. Test:
   `adversary_press_a_host_name_of_the_loopback_address_from_a_public_page_is_not_refused`
   (`crates/session/tests/adversary_press.rs:874`, renamed for what it then asserts), which
   listens on `[::1]` and uses the name `ip6-localhost`; if that name is not portable to the
   forge's runner (it is `::1` in `/etc/hosts` of Debian, Ubuntu and Manjaro, `:869`), the test
   uses a `Network` of its own (`crates/fetch/src/lib.rs:397`, `send_through`) with a resolver
   that maps a name to a local address. `story:agent-follow-links` gains the same rule and one
   case of the same shape for `follow`.
2. An action that does not parse is not shown: the button line reads
   `submits F<n> <method> unresolved`, with no text of the action, which could hold a userinfo.
   `docs/agent-answer.md:273` ("An action that does not resolve is written as the page wrote
   it, followed by `unresolved`") changes with it. Test:
   `adversary_press_an_unresolved_action_shows_no_userinfo` (`:665`).
3. A form's action and each distinct `formaction` are resolved once per document for an answer,
   not once per button: today `crates/surf/src/lib.rs:606` parses the action for each node
   (called per node at `:1856`, from `crates/session/src/lib.rs:1001`); 1,000 buttons with an
   action of 60,000 bytes took 14.7 s per `look` in a debug build. No test asserts wall time.
   Case 23 holds the answer of a page of 1,000 buttons with an action of 10,000 bytes (2.5 s per
   `look` before) and runs under 3 s alone; the unit report gives its time before and after.

Scope: `crates/fetch/src` and `crates/fetch/tests` for decision 1.
