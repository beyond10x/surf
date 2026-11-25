# Design: the HTML parser, its test data and the label

Decision note of `story:html-decisions` (`epic:html-parser`), written on 2026-11-21. It answers
the nine items of the story; each says whether it confirms or replaces the story's proposal, what
the answer rests on, and what it leaves open. The blocks `html-inventory` (item 9) and
`html-sections` (item 8) are read by programs; nothing else in this note is.

The texts read, all fetched on 2026-11-21:

| text | version read |
|---|---|
| WHATWG HTML Standard, `multipage/parsing.html`, `named-characters.html`, `rendering.html` | "Last Updated 20 November 2026", snapshot `0e292f402d0c3dd17fe00c1bb2ef3a3c94cd8bce` |
| WHATWG Encoding Standard and its `encodings.json` | "Last Updated 20 November 2026", snapshot `7d2e3e08e11f57ba1b080390b490d53648c6f69c` |
| W3C Accessible Name and Description Computation 1.2 | Editor's Draft 29 May 2026 |
| W3C HTML Accessibility API Mappings | Editor's Draft 29 July 2026 |
| html5lib-tests, `https://github.com/html5lib/html5lib-tests` | commit `c5495c539ca8a79c3c5107f5f14010bcff9a35d5`, 2026-11-05 |
| web-platform-tests, `https://github.com/web-platform-tests/wpt` | commit `13f8051342260922e50fd1f9d9877007ea025d2b`, 2026-11-20 |

Section names below are the HTML Standard's anchors on `https://html.spec.whatwg.org/multipage/`;
the numbers (13.2.3 and so on) are those of the snapshot above and change when the standard does.

## What the story assumed and the sources say otherwise

- **The tree-construction data is no longer in html5lib-tests.** At the pinned commit the
  repository holds `encoding/`, `serializer/` and `tokenizer/` and no `tree-construction/`. Its
  `README.md` reads in full: "The HTML parser tree construction tests are now solely maintained
  on web-platform-tests: https://github.com/web-platform-tests/wpt/tree/master/html/syntax/parsing".
  The directory was deleted by `224991e` ("Tree construction tests have moved to WPT",
  2026-06-26). The same `.dat` files are now `html/syntax/parsing/resources/*.dat` in wpt, with
  `scripted/x.dat` renamed `scripted_x.dat`. Item 3 takes them from there.
- **The standard now has processing instructions.** The tokenizer has five processing
  instruction states (13.2.5.72 to 13.2.5.76, `#processing-instruction-open-state` and after),
  the tokenizer data emits `["ProcessingInstruction", <target>, <data>]` tokens, and the tree data
  writes processing instruction nodes as `| <?target data?>`: 81 tree-construction cases hold one.
- **Seven encoding cases need the encoding changed while parsing**, so "a `meta` beyond the
  prescan window does not change the encoding" would leave them failing for good (item 4).

## 1. The node tree: behind the port, read only by the session process

**Confirmed.** The specification holds the document and its actionable elements as rows (item
6); the node tree is not modelled in ESS. The tree is built by the parser step of `crates/surf`
(item 2), kept by `Browser` per document, and read through `Browser::document_tree`, read only,
by the session process. No component's port receives it, the `agent` component's included
(`docs/plan/agent-plan-revision-1.md`, D1). Nothing changes the tree after parsing before a
script engine exists; a control's value, checkedness and selectedness are rows (D2).

Rests on: D1 and D2; `AGENTS.md` "Rules" (a component holds its own rows); the design's own
reason for not modelling every node (`docs/design/agent-interface.md`, section 6: "a
specification that holds every node of every page is a large model"). The HTML Standard keeps
control state apart from attributes (a control's value and checkedness have a dirty flag and are
not the `value` and `checked` attributes).

The epic's Outcome sentence this narrows ("the node tree is reachable by the `agent` component's
port") is already rewritten: `epic:html-parser` at revision 4 says "The node tree is reachable
inside the session process through `Browser` (`Browser::document_tree`) ... No component's port
receives the tree". Nothing is left to hand over.

Open: parse errors are not rows. No reader needs them (the answer format prints none), and
13.2.2 "Parse errors" obliges only conformance checkers to report them. The `UNMAPPED:` marker of
`spec/domains/document.yaml` keeps naming them.

## 2. The command shape: one command per element, then one ending command

**Replaced in part.** The `net` precedent stands: `ParseDocument` starts parsing and creates the
document in a state `Parsing`; the parser step of `crates/surf`, a parameter of `Browser` like the
transport, parses and issues the ending commands on the `document` port only; the suite issues
the ending commands itself, so the interpreter and the Rust system both run every scenario. What
the measurement replaces: the elements cannot be created from one list input without leaving the
ending command an obligation in the generated Rust. The shape is:

1. `ParseDocument` (`url`, `body`, and the navigation's `surf.net.RequestId`, item 6) creates the
   document in `Parsing` and publishes an event that carries what the parser step needs (the
   document's identity and the body), as `RequestStarted` carries the address to the transport.
2. One command per actionable element, in document order, creating that element's row with the
   fields of item 6 and the document's identity.
3. One ending command that moves the document to `Parsed` and sets the title, the encoding used,
   the encoding name not honoured (item 4) and the limit that stopped parsing (item 7), and
   publishes the event that the document is parsed.

The command names are `story:html-document-spec`'s to choose, in the standard's terms.

Measured with ess 0.96.0 on copies of `spec/` under `.out/html-decisions/<variant>/spec`, each
with `spec/domains/document.yaml` replaced and the `document` component's `accepts:` and
`publishes:` extended, each run with:

```sh
ess --strict-requires specify validate --path .out/html-decisions/<variant>/spec
ess verify conform synthesize --path .out/html-decisions/<variant>/spec \
  --scenarios .out/html-decisions/<variant>/spec --out .out/html-decisions/<variant>/run/suite.json
ess generate synthesize --strict-requires --path .out/html-decisions/<variant>/spec \
  --target rust --out .out/html-decisions/<variant>/run/rust
```

| variant | validate | conformance synthesis | `generate synthesize --target rust` |
|---|---|---|---|
| `base`: `spec/` as on `main` | `surf v1 — 8 file(s), 3 scenario(s), valid` | `51 scenario(s) (3 authored), 10 refusal(s)` | `92 capabilities: 90 generated, 0 obligation(s), 2 refused` |
| `list`: the ending command takes `elements: List<ParsedElement>` and writes one row per element with `affects: [{entity, each: {in: input.elements, as: e}, instance: e.element_id, sets}]`, a refusal `when: {not: {distinct: ...}}` beside it | valid, 1 warning (`ESS-ENTITY-019`, no relation from element to document) | `56 scenario(s) (3 authored), 10 refusal(s)` | `105 capabilities: 102 generated, 1 obligation(s), 2 refused`; the obligation: "kept an obligation by `distinct e in elements by e.element_id as uuid`, distinct list members no generated behaviour compares" |
| `list-rel`: `list` with `relations: [{name: document, kind: references, target: surf.document.Document, cardinality: one, via: document_id}]` on the element | valid, no warning | `56 scenario(s) (3 authored), 10 refusal(s)` | `105 capabilities: 102 generated, 1 obligation(s), 2 refused` |
| `list-lastwins`: `each: {..., repeated: last_wins}` and no distinct refusal | valid | `55 scenario(s) (3 authored), 10 refusal(s)` | `105 capabilities: 102 generated, 1 obligation(s), 2 refused`; the obligation: "`repeated: last_wins`" |
| `list-nodistinct`: `each:` with neither | refused, `exit 1`: `error[ESS-COMMAND-005]` "no `distinct:` holds that member distinct across the list" | not run | not run |
| `per-element`: one command per element creating the row, refusing an unknown document with `when_related: {entity: surf.document.Document, where: document_id == input.document_id, exists: false}` | valid | `56 scenario(s) (3 authored), 11 refusal(s)`; the new one `ESS-SYNTH-001`: "outcome surf.document.AddElement/added has no scenario" | `108 capabilities: 105 generated, 1 obligation(s), 2 refused`; the obligation: "a guard over the rows a selector selects (`when_related: ...`)" |
| `per-element-subject`: the document as the command's subject (`unknown_instance` refusal), the row written by an `affects:` entry without `each:` | refused, `exit 1`: `error[ESS-COMMAND-005]` "declares `affects[0]` with neither `where:` nor `each:`, so the entry names no rows" | not run | not run |
| `per-element-plain-rel`: one command per element creating the row, no guard, the relation declared | valid, no warning | `56 scenario(s) (3 authored), 10 refusal(s)` | `108 capabilities: 106 generated, 0 obligation(s), 2 refused` |
| `per-element-nav`: `per-element-plain-rel` with `navigation_id` on the document, mapped by `parse-on-navigation-committed` as `navigation_id: event.navigation_id` | valid | `56 scenario(s) (3 authored), 10 refusal(s)` | `108 capabilities: 106 generated, 0 obligation(s), 2 refused` |

So: one command per element is the only measured shape with no obligation and no new refusal.
The variants are in the worktree's `.out/html-decisions/` (not committed).

Open: in ess 0.96.0 the per-element command cannot refuse an unknown document, or one already
`Parsed`, without becoming an obligation (`per-element`) or being refused (`per-element-subject`).
It is accepted here because no actor is granted the command and only the parser step issues it,
in order, before the ending command. A generated Rust behaviour for `distinct:` over a list, or
for `when_related: {exists: false}`, would remove the gap; that is a question for ESS, not filed
by this story.

## 3. How the conformance data enters the repository

**Replaced in part**: two sources, not one.

| source | pinned at | licence | what is kept | vendored under `crates/html-conformance/data/` |
|---|---|---|---|---|
| html5lib-tests | `c5495c539ca8a79c3c5107f5f14010bcff9a35d5` | MIT (`LICENSE`: "Copyright (c) 2006-2013 James Graham, Geoffrey Sneddon, and other contributors", "Permission is hereby granted, free of charge") | `tokenizer/*.test` (14 files), every file of `encoding/` (5 files) | `html5lib-tests/`, with `LICENSE` and a `SOURCE` file (address, commit, date) |
| web-platform-tests | `13f8051342260922e50fd1f9d9877007ea025d2b` | 3-Clause BSD (`LICENSE.md`: "Copyright © web-platform-tests contributors") | `html/syntax/parsing/resources/*.dat` (62 files) | `wpt/html/syntax/parsing/resources/`, with `wpt/LICENSE.md` and `wpt/SOURCE` |

Not kept: html5lib-tests `serializer/` (5 files, 230 tests of 13.3 "Serializing HTML fragments",
which the epic leaves out and no section of this note depends on), `lint`, `lint_lib/` and
`pyproject.toml` (Python tooling), `tokenizer/README.md`; wpt `resources/` files that are not
`.dat` (`common.js`, `test.js`, `template.js`, `adoption-agency-reparenting.js`, which are wpt's
own script harness, and `chunked-response.py`). No Python file enters the repository. The data
is read only by the crate `surf-html-conformance` (standard library only, `publish = false`,
nothing depends on it); nothing converts it.

Why two sources: the tree-construction data at the last html5lib-tests commit that has it
(`9329e64`) is 60 `.dat` files with 1,713 cases; at the pinned wpt commit it is 62 files with
1,973 cases, among them `processing-instructions.dat` (124 cases) and `scripted_foster01.dat`.
wpt is where the standard's own suite keeps it now, and the completeness rule of `AGENTS.md`
counts wpt's tests. The tokenizer and encoding data stay in html5lib-tests, which still
maintains them (`c777c40` added processing instruction tokenizer tests on 2026-10-01).

`entities.json` (`https://html.spec.whatwg.org/entities.json`, the table of 13.5 "Named character
references", linked from that section as "a JSON file"): the WHATWG IPR policy
(`https://whatwg.org/ipr-policy`) licenses Living Standards under CC BY 4.0, and "To the extent
portions of such Living Standards or Review Drafts are incorporated into source code, such
portions in the source code are licensed under the BSD 3-Clause License instead." So the vendored
`crates/html-conformance/data/whatwg/entities.json` is CC BY 4.0 and its `SOURCE` says so, and the
generated `crates/html/src/charref_table.rs`, which ships, carries the BSD 3-Clause notice. Place
as `story:html-character-references` says.

**What one case is.**

- Tokenizer: one test object in one initial state. `initialStates` lists the states; absent, it
  is `["Data state"]`. `tokenizer/README.md`: "The test should be run once for each string".
- Tree construction: one `#data` section with what follows it up to the next case. A line that
  starts with `#` and has more after it is a section heading; `#description` starts a case and
  the `#data` after it belongs to the same case; otherwise `#data` starts a case. This is the rule
  of wpt's own reader (`resources/test.js`, `parseDat`), and `resources/README.md` adds the
  `#description` section to the format html5lib-tests had.
- Encoding: one `#data` section with its `#encoding`; the file `encoding/chardet/test_big5.txt`,
  which has neither, is one case.

**How a case runs and what is compared.**

- Tokenizer: the input (unescaped once more where `doubleEscaped` is true), preprocessed as
  13.2.3.5 says, tokenized from the initial state with `lastStartTag` as the last start tag
  emitted. Compared: the tokens, adjacent `Character` tokens coalesced, and the parse error codes
  in order. Not compared: an error's `line` and `col`, which the data adds and 13.2.2 "Parse
  errors" does not define. Processing instructions are compared in the form the data uses,
  three members, although `tokenizer/README.md` still gives two.
- Tree construction: the input is the `#data` lines joined by LF without the final LF. With
  `#document-fragment` it is parsed by the fragment parsing algorithm (13.4) for the context
  element the line names (`svg ` and `math ` prefixes give the namespace), with no sanitizer and
  no declarative shadow roots; otherwise as a document. Parser scripting disabled (13.2.4.5, the
  standard's term for the flag the data calls scripting) is false with `#script-on` and true
  otherwise, which is also how surf parses every page: it has no script engine. Compared: the
  result written in the `| ` format of `resources/README.md`, byte for byte against `#document`.
  Not compared: `#errors` and `#new-errors`. The README says of them "the only thing that matters
  is that there be the right number of parse errors"; their lines are html5lib's own names, not
  the codes of 13.2.2; and the standard's suite does not compare them either (`resources/test.js`:
  "parsing errors (ignored here)"). Comparing the number is stricter than the standard's suite and
  is left open.
- Encoding: the bytes of the `#data` section (raw bytes, no final LF) through the encoding
  sniffing algorithm with no transport-layer encoding and the default of item 4, then parsed, so
  that a `meta` met by the tree builder can change the encoding (item 4). Compared: the encoding
  used at the end with the `#encoding` value read as a label of the Encoding Standard
  (`Windows-1252` and `iso-8859-2` are labels, not names).

**Not run**: 12 cases, each with its reason (the `not-run` tag of item 8): 4 of
`tokenizer/xmlViolation.test` (`xml-infoset`), 7 that run script while parsing (`needs-script`:
`scripted_adoption01.dat` 1, `scripted_ark.dat` 1, `scripted_foster01.dat` 2,
`scripted_webkit01.dat` 2, `encoding/scripted/tests1.dat` 1), 1 of autodetection
(`autodetect`: `encoding/chardet/test_big5.txt`). No case of the data uses declarative shadow
roots (`rg -c -i shadowroot html/syntax/parsing/resources/*.dat` prints nothing), so that
exclusion costs no case.

**web-platform-tests tests for the parser that are not the `.dat` data.** `html/syntax/parsing/`
and `html/syntax/parsing-html-fragments/` hold 101 test files besides `html5lib_url.html`
(counted with `git ls-tree -r --name-only HEAD html/syntax/parsing
html/syntax/parsing-html-fragments`, leaving out `resources/`, `support/`, `README`,
`named-character-references-data.js`, `WEB_FEATURES.yml` and `*.headers`). Every one is a
testharness.js test, a reftest or a crashtest that needs a script engine or rendering, and none
runs here; each is in `html-sections` with its section and what it needs, as are the directories
`html/syntax/charset/`, `html/syntax/xmldecl/`, `html/syntax/speculative-parsing/`,
`html/syntax/speculative-charset/` and the Encoding Standard's `encoding/`. The three wrappers
of the `.dat` files: `html5lib_url.html` runs each case through a Blob URL and compares the same
tree dump, so a case the harness runs and passes stands for its `html5lib_url.html` variant;
`html5lib_write.html` and `html5lib_write_single.html` feed the same input through
`document.write` and do not run.

Open: the wpt tests above. Until a script engine exists, every section that has one stays
`incomplete` by the completeness rule.

## 4. Which encodings first

**Confirmed, with one part replaced.**

- The whole label table of the Encoding Standard: 40 encodings, 228 labels (`jq
  '[.[].encodings[]]|length' encodings.json` prints `40`, `jq '[.[].encodings[].labels[]]|length'
  encodings.json` prints `228`), so BOM sniffing, the prescan (with its "get an XML encoding" step
  for a bogo-XML declaration) and a `meta` name every encoding.
- Decoders for UTF-8, UTF-16LE, UTF-16BE, windows-1252 and replacement. An encoding without a
  decoder is decoded as windows-1252, and the document records the name it could not honour. This
  is a deviation: 13.2.3.3 says "User agents must support the encodings defined in Encoding". The
  next step, if one is wanted, is the 28 legacy single-byte encodings, which share one decoder and
  differ only by an index of 128 code points.
- The default, when nothing else decides, is windows-1252: the last step of the encoding sniffing
  algorithm returns "an implementation-defined or user-specified default", and 35 of the 82 run
  encoding cases expect `windows-1252` (`tests1.dat` begins with a control case that "should pass
  - you may have to set your defaults appropriately").
- The transport-layer step ("If the transport layer specifies a character encoding, and it is
  supported, return that encoding with the confidence certain") is skipped while `surf.net`
  drops the `charset` parameter of `Content-Type` (`surf.net.MediaType`: "without parameters").
- The optional steps are not taken: the user's override, the likely encoding from an earlier
  visit, the container document's encoding (surf has no nested documents), and autodetection.
- **Replaced**: the prescan reads the first 1,024 bytes ("User agents are encouraged to only
  prescan the first 1024 bytes"), and a `meta` that the tree builder meets while the confidence is
  tentative changes the encoding as 13.2.3.4 "Changing the encoding while parsing" says. surf
  holds the whole body before parsing, so it takes the last step of that algorithm as the
  standard prefers it ("the bytes should be re-parsed from memory"): it parses the same bytes again
  with the new encoding and the confidence certain, inside the one parse, with no second
  navigation. The optional on-the-fly switch is not taken.

Why the replacement: seven encoding cases have their only `meta` after byte 1,024 and expect it
honoured. Measured with:

```sh
cd html5lib-tests && LC_ALL=C awk -f encoding-case-sizes.awk encoding/tests1.dat \
  encoding/tests2.dat encoding/test-yahoo-jp.dat | awk '$3 > 1000 || $5 > 1000'
```

where `encoding-case-sizes.awk` prints, per case, the file, the case number, the bytes of its
`#data`, the expected encoding and the offset of the first `charset`:

```
encoding/tests1.dat 46 1022 iso-8859-2 1002
encoding/tests1.dat 47 1023 iso-8859-2 1003
encoding/tests1.dat 48 2046 iso-8859-2 2026
encoding/tests1.dat 49 2047 iso-8859-2 2027
encoding/tests1.dat 50 4094 iso-8859-2 4074
encoding/tests1.dat 51 4095 iso-8859-2 4075
encoding/tests1.dat 52 8190 iso-8859-2 8170
encoding/tests1.dat 53 8191 iso-8859-2 8171
encoding/tests1.dat 54 8387 iso-8859-2 8323
```

Cases 48 to 54 (a comment of 2,048 to 8,193 characters, then `<meta charset="iso8859-2">`)
expect `iso-8859-2`. A prescan that stops at byte 1,024 is still inside the comment for them, so
by the algorithm (not measured: no parser exists yet) the sniffing falls through to the default.

Open: the transport-layer step and the 35 decoders, both listed in `html-sections`. The seven
cases also tie the encoding cases to the tree builder: they pass only once the `meta` rules of
the "in head" insertion mode exist.

## 5. What "label" is before CSS

**Confirmed, with "hidden" widened.** The label is the accessible name of Accessible Name and
Description Computation 1.2 ("Computation steps", 4.3.2), with the HTML Accessibility API
Mappings for each element ("Accessible Name Computations By HTML Element", for example
`#input-type-text-input-type-password-input-type-number-input-type-search-input-type-tel-input-type-email-input-type-url-and-textarea-elements-accessible-name-computation`),
computed over the tree (`crates/page/src/label.rs`, `story:html-accessible-name`). The row holds
the name cut at `surf_page::LABEL_BYTES_MAX`, 1,024 bytes: a longer name ends at the last UTF-8
character boundary at or below 1,024 bytes, without a space the cut would leave at its end, the
walk for that name stops there, and the row's `label_cut` is set. Cutting it to 60 characters is
the answer format's job. `surf_page::LABEL_WORK_MAX`, 16,777,216, bounds the work for all labels
of one call of `actionable_elements`: one unit per node read, one per byte read from a text node
or an attribute value into a name, and the bytes of each `aria-labelledby` value split into
IDREFs. Once it is spent, the name being computed and each later one come from the element's own
attributes only (`aria-label`; an `input`'s `value`, or its `Submit` or `Reset`; `alt`; `title`;
`placeholder`; `aria-placeholder`), without `aria-labelledby`, `label` elements or the subtree,
and `label_cut` is set. So the time of the labels is linear in the nodes and the attributes of
the tree plus that budget, and their memory at most 1,024 bytes a row and the budget plus the
attribute bytes the tree stores. Both bounds are a deliberate deviation: the standards bound
neither a name nor its work. A hidden actionable element, whose row is kept and flagged, is named
from its own subtree, hidden parts included, as a hidden node referenced by `aria-labelledby` is
(step 2A would give it the empty name); a deviation too.

"Hidden" in the step "Hidden Not Referenced": accname's own note counts "Nodes with CSS
properties display:none, visibility:hidden, visibility:collapse or content-visibility:hidden" as
hidden. Without CSS, surf reads a node as hidden when it or an ancestor has
`aria-hidden="true"` or matches a rule of the HTML Standard's user-agent style sheet that needs
no style computation. In 15.3.1 "Hidden elements" (`rendering.html#hidden-elements`): the
elements `area, base, basefont, datalist, head, link, meta, noembed, noframes, param, rp, script,
style, template, title`; `[hidden]` on any element but `embed`, whatever its value (the
`until-found` value gives `content-visibility: hidden`, which accname also counts);
`input[type=hidden i]`. In 15.3.3 "Flow content": `dialog:not([open])`, a `dialog` without an
`open` attribute, and `[popover]:not(:popover-open):not(dialog[open])`, an element with a
`popover` attribute, whatever its value, but an open `dialog`: surf runs no script and no user
action, so no popover is ever open. The copy of the selected `option` that the parser puts into
a `selectedcontent` (one with a single `select` without `multiple` above it and no `option` or
`selectedcontent` above it, the elements the update fills) is hidden, and `actionable_elements`
lists no row from it and records no `id` of it: the option's own content is listed once. This
is surf's reading, not the standard's rendering, where a browser shows the copy in the closed
control and the options only in the picker. A `selectedcontent` the update does not fill keeps
its content as parsed and is read as any element; one inside a `select` whose update a limit
stopped is hidden all the same, as the tree does not tell it apart. In 15.5.5 "The details and summary elements": a `details` element without an
`open` attribute gives `content-visibility: hidden` to the slot that takes every child but its
first `summary` element child, so only that `summary` is shown (a `details` with no `summary`
child shows its default summary, which is not in the tree, and none of its children).
`noscript` is not hidden, because surf parses with scripting disabled. The fallback content of
`video`, `audio` and `object` is not hidden: surf plays no media and loads no object, so the
fallback is what is shown. An `area` is not hidden for its own name: it is drawn through its
image's map. This is wider than the story's "the `hidden` attribute or `aria-hidden="true"`".

Steps not taken, because they need rendering or a part of the DOM surf does not have: "Name From
Generated Content" (`::before`, `::after`, `::marker`); hidden by author CSS (`display`,
`visibility`, `content-visibility` from a style sheet or `style` attribute); "Determine Child
Nodes" for shadow roots and slots (surf has none); the spaces accname may join with by CSS
`display` (its own note: "considering the feasibility of joining text strings with and without
spaces, depending on the CSS display value", AccName #225). Not taken either: an explicit `role`
attribute; the role of each element is the one its kind implies (item 6).

Open: the computation is never `complete`: wpt checks it in a running browser with script.

## 6. What the rows hold

**Confirmed.**

- A link's target is its `href` value as written. Resolving it against the document base URL is
  `story:agent-follow-links`'s, with `surf-url` (D10).
- A document carries the `surf.net.RequestId` of its navigation, mapped by
  `parse-on-navigation-committed` from `NavigationCommitted.navigation_id` (measured in item 2,
  variant `per-element-nav`: no new refusal).
- A `select` row holds no value; each option's selectedness is a row of `story:agent-forms-spec`
  (D11).
- The kinds are the nine of `docs/design/agent-interface.md` section 5.2. By element and by the
  `input` element's type state (HTML Standard, the `type` attribute's states; an unknown or
  missing value is the Text state):

| element, `input` type state | kind |
|---|---|
| `a` or `area` with an `href` attribute | `link` |
| `button`, any `type` | `button` |
| `input`: Submit Button, Image Button, Reset Button, Button | `button` |
| `input`: Text, Search, Telephone, URL, Email, Number, Range, Color, Date, Month, Week, Time, Local Date and Time | `text` |
| `input`: Password | `password` |
| `input`: Checkbox | `checkbox` |
| `input`: Radio Button | `radio` |
| `input`: File Upload | `file` |
| `select` | `select` |
| `textarea` | `textarea` |
| `input`: Hidden | not listed |

Number, Range, Color and the date and time states are `text` because the nine kinds have nothing
closer and each has a value that is a string. Elements inside template contents are not in the
document and are not listed.

Open: whether the answer wants kinds for those states, or for `summary`, `details` and
`contenteditable` elements.

## 7. Limits

**Confirmed**, the defaults stated like those of `crates/fetch` (`Limits`, documented defaults):

| limit | default |
|---|---|
| input bytes | 16,777,216 (16 MiB, the `surf-fetch` body limit) |
| tree depth (elements on the path from the root to any node of the tree, or from the top of a subtree out of it) | 512 |
| nodes, template contents included | 1,048,576 |
| attributes on one element | 1,024 |
| stored bytes (every string the tree holds, clones included, and 80 for each attribute) | 67,108,864 (64 MiB) |
| steps (entries tree construction visits in its scans, and nodes the walks of the depth limit visit) | 134,217,728 (2^27) |

Reaching one stops parsing as "stop parsing" does; the document holds what was built and names
the limit (a row field, item 2).

Depth, since fix rounds 2 and 3 of `story:html-script-hook`: the limit bounds the tree, in
every parse, with a script hook or without, and not the stack of open elements. Markup alone
made a tree deeper than its stack before (a random search at a limit of 16 found 18; the
`selectedcontent` copy made 2N − 4 at a limit of N), and a hook's writes can make it deeper
still; foster parenting makes the stack deeper than the tree. No node of a parsed tree is
deeper than the limit, counted as the write surface counts: the elements on its path from the
root, itself included, a template counted for the nodes of its contents; a subtree out of the
tree is counted from its own top. The parse stops with `Depth` only before a node it would
make or move would pass the limit in the tree as it is then: an element inserted under a
parent at the limit (each insertion of an element, the in-head `script`, the `template` start
tag and the `html` element included); an outer iteration of the adoption agency algorithm
whose moves would put a node past it, checked for the whole iteration before its first move, so
the iteration changes nothing and the tree holds every node where it was; an element of the
`selectedcontent` copy under a node at the limit, which leaves the copy cut there.

How the parser knows a depth (fix round 3): no node's record keeps one (`NODE_BYTES` stays
136). The parser keeps the depth of each entry of the stack of open elements, set when the
entry is pushed from the depth of its place, and reads them again from the tree after a hook's
write that moved or removed an element and after each adoption agency iteration: each entry
from its parent up to the first entry already read, most often one step. After an iteration
only the entries from the formatting element's place up are read, unless a hook's writes put
an entry under one above it (then all). An insertion place is an entry, the parent of the
foster parenting `table` or a template's contents, so a check reads a kept depth; a place that
is no entry is walked up to one. The adoption agency walks the furthest block's subtree only
when its children would end deeper than they were, for the deepest of them; a move that makes
no node deeper walks nothing of what it moves. The `selectedcontent` update counts the depth
of each `selectedcontent` on its walk down the document. Every parent or host a walk up passes
to, and every node the adoption agency's walk down visits, is a step.

The stack of open elements without a bound of its own (fix round 4): a hook's writes can keep
every element of the markup open with no parent, so the stack is as long as the node limit lets
it be (a node is on it at most once). The step limit bounds every read of it: each entry whose
depth is read again is a step, also one with no parent, which no walk up reaches; an insertion
into the stack or a removal from it counts each entry it moves and each scope boundary of the
stack's list of them it visits; the `frameset` rule counts each entry it pops. The pops of
"stop parsing" are not counted: they come once per parse pass, after its last token, and a
count there would change what the step limit checked for that token. A `meta` that changes the
encoding of `parse_document_bytes` ends the first pass and starts a second over the same bytes;
that pass takes no hook, so its stack stays inside the depth limit.

Read inside the token (fix round 5): with the stack that long, one token's work
could pass the limit by any factor (one `</b>` whose inner loop removes N entries, each removal
moving the M entries above it). The count is read after each token, before each entry of every
scan of the stack of open elements or of the list of active formatting elements (the scopes,
the stack position, popping, the implied end tags, "any other end tag", the `li` and `dd`/`dt`
scans, clearing back to a table context, resetting the insertion mode, the search for a
`template`, the place of foster parenting, the breakout of foreign content and its end tags,
Noah's Ark, the reconstruction, the list's scans and the adoption agency's scans and inner
loop, and the pass that reads the stack's depths again), and before every write of the parser
to the tree and every stop at another limit. Not read inside them, counted as the derivation
at `max_steps` says: the pass that marks the stack's depths to be read again (no step), the
moves of a removal from inside the stack or an insertion into it (counted at once), the pops
after a scan that found where to stop (the scan counted them), a walk up the tree from one
entry (each hop), a child list's scan or move and the walk under the furthest block (each
node). Past the limit the scan ends with its answer for "not found", the write is not made and
parsing stops at `Steps`, never at another limit (a page that sets the attribute, depth, node
or stored-byte limit just past what a cut scan's answer would do stops at `Steps`, fix round 6);
an adoption agency iteration whose inner loop made clones puts them back at the place of step
4.14 first, so every node the parse made stays in the tree. Below the limit no read changes a
tree or a count. A parse ends fewer than 4 ×
`max_nodes` steps past the limit: past it only the rest of the stretch between two reads and
the putting back are counted, and the longest stretches hold four pieces of work of fewer than
`max_nodes` visits each (`limits.rs`, `max_steps`, has the derivation).

Stored bytes, as `story:html-parser-limits` made them exact: the UTF-8 bytes of every string the
tree holds, element local names, attribute names, prefixes and values, text, comment data,
doctype fields, processing instruction targets and data. Names count as well as values because
the reconstruction of the active formatting elements clones attribute names, which would
otherwise grow without a bound. Since the adversarial review of the story, each attribute also
counts `limits::ATTRIBUTE_BYTES`, the size of its record (`size_of::<Attribute>()`, 80 bytes):
counted by its strings alone, an attribute with a one-byte name counted 1 while it held 80, and
the default admitted about 2.3 GB of heap. What the limit promises: a tree holds at most 2 ×
`stored_bytes()` + 3 × `limits::NODE_BYTES` × `node_count()` bytes of heap, where `NODE_BYTES`
(136) is the size of a node's record in the arena and of its entry in its parent's child list;
the factors cover lists and texts that grow by doubling, a child list's first room of 4 entries
and a text's first 8 bytes. At the defaults that is about 536 MiB (inferred from the bound).
Nodes are not counted in the stored bytes: the node limit bounds them, and at 136 bytes each
the default stored-byte limit would otherwise stop a tree long before the node limit. Steps:
every entry visited in a scan of the stack of open elements or of the list of active formatting
elements, and every child visited in a parent's child list, checked after each token (and,
since fix round 5 of `story:html-script-hook`, inside the token: "Read inside the token" above);
`Tree::steps` reads the count a parse ended with. "Is in the stack of open elements" visits no
entry (the stack keeps a count for each node), so the reconstruction before each character of a
text costs no step when there is nothing to reconstruct (before the review, text 20 elements
below an unclosed `<b>` cost 20 steps a byte and a 1 MiB page of it was cut). The five other
limits bound memory, not time: an input
inside them can still cost tokens times depth stack visits (505 nested `span`, then `</q>` to
16 MiB, ran 122.9 s in the release profile before the story). The default is twice the visits a
16 MiB page of the kind of five large documentation pages would make (at most 3.8 visits per
input byte). Where each limit stops, and what a caller gets, is the section "Limits" of
`crates/html/src/lib.rs`. The tests of `crates/html/tests/limits.rs` and
`crates/html/tests/adversary_limits.rs` assert no time (a loaded machine made 10 s bounds fail):
they assert the limit that stopped a parse, its steps, and the bytes it allocated, read by a
counting allocator of the test binary; a copy that a token can repeat without paying steps (a
formatting end tag behind a `table` copied its element's start tag token, 1,024 attributes for
4 input bytes) shows in the bytes allocated. The
standard places none of these: "The algorithm described below places no limit on the depth of
the DOM tree generated, or on the length of tag names, attribute names, attribute values, Text
nodes, etc. While implementers are encouraged to avoid arbitrary limits, it is recognized that
practical concerns will likely force user agents to impose nesting depth constraints" (13.2.6,
before 13.2.6.1). The depth limit is the kind it expects; the input, node, attribute,
stored-byte and step limits are a deliberate deviation, and so is what reaching any limit does
(parsing stops).

Measured by the story, seconds before and after it, release and test profile (a test-profile
run before it was ended at 60 s):

| input | release before | release after | test before | test after |
|---|---|---|---|---|
| 505 nested `span`, then `</q>` to 1 MiB | 27.006 | 0.427 | over 60 | 5.996 |
| Noah's Ark: 100 nested `<b>` with 1,024 attributes | 28.417 | 0.040 | over 60 | 0.592 |
| `<span>` with 1,024 attributes and `</span>` to 1 MiB (the duplicate check) | 1.653 | 0.075 | 2.779 | 0.766 |
| 50,000 `<html aK>` | 37.214 | 0.002 | 39.644 | 0.070 |

They cost no conformance case. The largest tree-construction input is 1,041 bytes
(`tests1.dat`) with at most 38 `<` followed by a letter in one case (`tests11.dat`); the largest
tokenizer input is 950 characters (`jq -s '[.[] | (.tests // .xmlViolationTests)[] | .input |
length] | max' tokenizer/*.test`); all encoding data together is 48,995 bytes.

## 8. The case tags and the section map

**Confirmed, made exact**: the story's tag rules, with the mixed files as they exist (there is no
`tests13.dat`) and the `.dat` files read from wpt (item 3). Every case has exactly one tag. The
rules, applied in this order:

1. `not-run`, with a reason, for the cases item 3 does not run: every case of
   `tokenizer/xmlViolation.test` (`xml-infoset`), of `encoding/scripted/` and of
   `resources/scripted_*.dat` (`needs-script`), and `encoding/chardet/test_big5.txt`
   (`autodetect`).
2. Tokenizer: `charref` when the input (after the `doubleEscaped` unescaping) holds `&`; else
   `markup` when the initial state is `Data state`; else `text`. A case in `CDATA section state`
   is `text`, although `story:html-tokenizer-markup` writes those states.
3. Encoding: `encoding`.
4. Tree construction, the mixed files: every case of the 30 files `tests1.dat` to `tests26.dat`
   (there is no `tests13.dat`), `webkit01.dat`, `webkit02.dat`, `tricky01.dat`,
   `html5test-com.dat` and `pending-spec-changes.dat` is `mixed`.
5. Tree construction, the 28 thematic files (every other `.dat` but `scripted_*.dat`), by the
   input lowered in ASCII case: `body`, then the last of these that applies: `formatting` when it
   holds a tag of `a b big code em font i nobr s small strike strong tt u`; `tables` for `table
   caption colgroup col tbody thead tfoot tr td th select`; `template` for `template`; `foreign`
   for `svg` or `math`, or when the case has `#document-fragment`. "Holds a tag of `x`" means the
   input contains `<x` or `</x` followed by one of tab, LF, FF, CR, space, `/` or `>`.

Per tag, the counts are the `tag` lines of `html-inventory` (item 9).

The section map. `html-sections` names the sections of the standards a mark is given for, which
data files belong to each, and per section the requirements this note leaves out and the
web-platform-tests tests that do not run. A section's mark can be `complete` only when every case
of its files passes and it has no `open` and no `wpt` line; today every section has at least one,
so every section is `incomplete` until a script engine exists, whatever its counts. Sections are
taken whole, their subsections included, except that `tree-construction` (13.2.6) leaves out
`parsing-main-inforeign` (13.2.6.5), which is a section of its own. 13.2.1, 13.2.2, 13.2.4 and
13.2.10 hold the definitions and notes the other sections use and get no mark of their own;
13.2.8 "Speculative HTML parsing" is optional ("User agents may implement an optimization") and
is listed only for its tests.

The block's format, one record per line, fields separated by one space: `section <name> <address>`
starts a section, `<name>` being the standard's anchor; `open <name> <id> <text>` is a requirement
of that section left out, `<id>` matching `[a-z0-9-]+` and `<text>` running to the end of the
line; `wpt <name> <path> <needs>` is a web-platform-tests test of that section that does not run,
`<path>` relative to the wpt root at the pinned commit, a path ending in `/` standing for every
test under that directory, and `html5lib_write.html` and `html5lib_write_single.html` standing for
their variants `?file=<f>` for each `.dat` file `<f>` of that section that the wrapper lists,
`<needs>` being `script`, `rendering` or both joined by `,`; `data <name> <path>` puts a data file
in that section, `<path>` written as in `html-inventory`. Every `<name>` after the first field is
one that a `section` line declares. Every `file` path of `html-inventory` is in exactly one `data`
line.

```html-sections
section the-input-byte-stream https://html.spec.whatwg.org/multipage/parsing.html#the-input-byte-stream
open the-input-byte-stream transport-layer-encoding The encoding sniffing algorithm's transport layer step: surf.net.MediaType carries no charset parameter.
open the-input-byte-stream encodings-without-decoder Character encodings: 35 of the Encoding Standard's 40 encodings have no decoder; they are decoded as windows-1252 and the name is recorded.
open the-input-byte-stream insertion-point Preprocessing the input stream: the insertion point, which only document.write uses.
wpt the-input-byte-stream encoding/ script
wpt the-input-byte-stream html/syntax/charset/ rendering,script
wpt the-input-byte-stream html/syntax/xmldecl/ script
wpt the-input-byte-stream html/syntax/parsing-html-fragments/the-input-byte-stream-001.html script
wpt the-input-byte-stream html/syntax/parsing-html-fragments/the-input-byte-stream-003.html script
wpt the-input-byte-stream html/syntax/parsing-html-fragments/the-input-byte-stream-004.html script
wpt the-input-byte-stream html/syntax/parsing-html-fragments/the-input-byte-stream-007.html script
wpt the-input-byte-stream html/syntax/parsing-html-fragments/the-input-byte-stream-009.html script
wpt the-input-byte-stream html/syntax/parsing-html-fragments/the-input-byte-stream-016.html script
wpt the-input-byte-stream html/syntax/parsing-html-fragments/the-input-byte-stream-018.html script
wpt the-input-byte-stream html/syntax/parsing-html-fragments/the-input-byte-stream-030.html script
wpt the-input-byte-stream html/syntax/parsing-html-fragments/the-input-byte-stream-034.html script
wpt the-input-byte-stream html/syntax/parsing-html-fragments/the-input-byte-stream-037.html script
wpt the-input-byte-stream html/syntax/parsing-html-fragments/the-input-byte-stream-038.html script
wpt the-input-byte-stream html/syntax/parsing/meta-inhead-insertion-mode.html script
wpt the-input-byte-stream html/syntax/parsing/newline-normalization-cr-then-lf.html script
wpt the-input-byte-stream html/syntax/parsing/quotes-in-meta.html script
data the-input-byte-stream html5lib-tests/encoding/chardet/test_big5.txt
data the-input-byte-stream html5lib-tests/encoding/scripted/tests1.dat
data the-input-byte-stream html5lib-tests/encoding/test-yahoo-jp.dat
data the-input-byte-stream html5lib-tests/encoding/tests1.dat
data the-input-byte-stream html5lib-tests/encoding/tests2.dat
section tokenization https://html.spec.whatwg.org/multipage/parsing.html#tokenization
wpt tokenization html/syntax/parsing/ambiguous-ampersand.html script
wpt tokenization html/syntax/parsing/doctype-system-identifier-distinction.html script
wpt tokenization html/syntax/parsing/empty-doctype-ids.html script
wpt tokenization html/syntax/parsing/no-doctype-name.html script
wpt tokenization html/syntax/parsing/truncated-markup-declaration-at-eof.html script
wpt tokenization html/syntax/parsing/zero.html script
wpt tokenization html/syntax/parsing-html-fragments/tokenizer-modes-001.html script
data tokenization html5lib-tests/tokenizer/contentModelFlags.test
data tokenization html5lib-tests/tokenizer/domjs.test
data tokenization html5lib-tests/tokenizer/entities.test
data tokenization html5lib-tests/tokenizer/escapeFlag.test
data tokenization html5lib-tests/tokenizer/numericEntities.test
data tokenization html5lib-tests/tokenizer/pendingSpecChanges.test
data tokenization html5lib-tests/tokenizer/test1.test
data tokenization html5lib-tests/tokenizer/test2.test
data tokenization html5lib-tests/tokenizer/test3.test
data tokenization html5lib-tests/tokenizer/test4.test
data tokenization html5lib-tests/tokenizer/unicodeChars.test
data tokenization html5lib-tests/tokenizer/unicodeCharsProblematic.test
section named-character-references https://html.spec.whatwg.org/multipage/named-characters.html#named-character-references
wpt named-character-references html/syntax/parsing/named-character-references.html script
data named-character-references html5lib-tests/tokenizer/namedEntities.test
section tree-construction https://html.spec.whatwg.org/multipage/parsing.html#tree-construction
open tree-construction script-processing The script end tag in the text insertion mode: the insertion point, a script nesting level above 1, the parser pause and the pending parsing-blocking script, and `document.write`.
open tree-construction custom-element-creation Create an element for a token: looking up a custom element definition, willExecuteScript, the constructor and the element reactions.
open tree-construction declarative-shadow-roots The template start tag in head with a shadowrootmode attribute: attaching a declarative shadow root.
open tree-construction template-content-patching The template start tag in head with a for attribute: content patching, which moves the template's content into the element it marks; surf keeps it in the template contents.
open tree-construction selectedcontent-update-timing The select element's queued update of its selectedcontent elements from the option and selectedcontent insertion steps: surf runs it once after the end-of-file token, for a document only.
wpt tree-construction html/syntax/parsing/html5lib_write.html script
wpt tree-construction html/syntax/parsing/html5lib_write_single.html script
wpt tree-construction html/syntax/parsing/adoption-agency-reparenting.html script
wpt tree-construction html/syntax/parsing/adoption-agency-reparenting-cycle.html script
wpt tree-construction html/syntax/parsing/adoption-agency-reparenting-document-target.html script
wpt tree-construction html/syntax/parsing/adoption-agency-reparenting-inner-loop-cycle.html script
wpt tree-construction html/syntax/parsing/adoption-agency-reparenting-pagehide.tentative.html script
wpt tree-construction html/syntax/parsing/adoption-agency-reparenting-shadow-cycle.html script
wpt tree-construction html/syntax/parsing/adoption-agency-reparenting-template-cycle.html script
wpt tree-construction html/syntax/parsing/adoption_agency_check_the_end_tag_name.html script
wpt tree-construction html/syntax/parsing/crashtests/ script
wpt tree-construction html/syntax/parsing/custom-element-reparenting.html script
wpt tree-construction html/syntax/parsing/custom-element-reparenting-shadow-cycle.html script
wpt tree-construction html/syntax/parsing/custom-element-reparenting-template-cycle.html script
wpt tree-construction html/syntax/parsing/form-element-pointer-in-template.html script
wpt tree-construction html/syntax/parsing/foster-parenting-into-document-with-element-child.html script
wpt tree-construction html/syntax/parsing/foster-parenting-into-template-element.html script
wpt tree-construction html/syntax/parsing/foster-parenting-moved-table.window.js script
wpt tree-construction html/syntax/parsing/inhead-noscript-head.html script
wpt tree-construction html/syntax/parsing/insert-into-nonempty-document.html script
wpt tree-construction html/syntax/parsing/misnested-form-in-template.html script
wpt tree-construction html/syntax/parsing/noscript-template-querySelector.html script
wpt tree-construction html/syntax/parsing/Object-prototype-names.html script
wpt tree-construction html/syntax/parsing/parser-text-node-coalescing-and-mutations.html script
wpt tree-construction html/syntax/parsing/template/ script
data tree-construction wpt/html/syntax/parsing/resources/adoption01.dat
data tree-construction wpt/html/syntax/parsing/resources/adoption02.dat
data tree-construction wpt/html/syntax/parsing/resources/blocks.dat
data tree-construction wpt/html/syntax/parsing/resources/comments01.dat
data tree-construction wpt/html/syntax/parsing/resources/doctype01.dat
data tree-construction wpt/html/syntax/parsing/resources/domjs-unsafe.dat
data tree-construction wpt/html/syntax/parsing/resources/entities01.dat
data tree-construction wpt/html/syntax/parsing/resources/entities02.dat
data tree-construction wpt/html/syntax/parsing/resources/html5test-com.dat
data tree-construction wpt/html/syntax/parsing/resources/inbody01.dat
data tree-construction wpt/html/syntax/parsing/resources/isindex.dat
data tree-construction wpt/html/syntax/parsing/resources/main-element.dat
data tree-construction wpt/html/syntax/parsing/resources/menuitem-element.dat
data tree-construction wpt/html/syntax/parsing/resources/noscript01.dat
data tree-construction wpt/html/syntax/parsing/resources/pending-spec-changes-plain-text-unsafe.dat
data tree-construction wpt/html/syntax/parsing/resources/pending-spec-changes.dat
data tree-construction wpt/html/syntax/parsing/resources/plain-text-unsafe.dat
data tree-construction wpt/html/syntax/parsing/resources/processing-instructions.dat
data tree-construction wpt/html/syntax/parsing/resources/quirks01.dat
data tree-construction wpt/html/syntax/parsing/resources/ruby.dat
data tree-construction wpt/html/syntax/parsing/resources/scriptdata01.dat
data tree-construction wpt/html/syntax/parsing/resources/scripted_adoption01.dat
data tree-construction wpt/html/syntax/parsing/resources/scripted_ark.dat
data tree-construction wpt/html/syntax/parsing/resources/scripted_foster01.dat
data tree-construction wpt/html/syntax/parsing/resources/scripted_webkit01.dat
data tree-construction wpt/html/syntax/parsing/resources/search-element.dat
data tree-construction wpt/html/syntax/parsing/resources/tables01.dat
data tree-construction wpt/html/syntax/parsing/resources/template.dat
data tree-construction wpt/html/syntax/parsing/resources/tests1.dat
data tree-construction wpt/html/syntax/parsing/resources/tests10.dat
data tree-construction wpt/html/syntax/parsing/resources/tests11.dat
data tree-construction wpt/html/syntax/parsing/resources/tests12.dat
data tree-construction wpt/html/syntax/parsing/resources/tests14.dat
data tree-construction wpt/html/syntax/parsing/resources/tests15.dat
data tree-construction wpt/html/syntax/parsing/resources/tests16.dat
data tree-construction wpt/html/syntax/parsing/resources/tests17.dat
data tree-construction wpt/html/syntax/parsing/resources/tests18.dat
data tree-construction wpt/html/syntax/parsing/resources/tests19.dat
data tree-construction wpt/html/syntax/parsing/resources/tests2.dat
data tree-construction wpt/html/syntax/parsing/resources/tests20.dat
data tree-construction wpt/html/syntax/parsing/resources/tests21.dat
data tree-construction wpt/html/syntax/parsing/resources/tests22.dat
data tree-construction wpt/html/syntax/parsing/resources/tests23.dat
data tree-construction wpt/html/syntax/parsing/resources/tests24.dat
data tree-construction wpt/html/syntax/parsing/resources/tests25.dat
data tree-construction wpt/html/syntax/parsing/resources/tests26.dat
data tree-construction wpt/html/syntax/parsing/resources/tests3.dat
data tree-construction wpt/html/syntax/parsing/resources/tests4.dat
data tree-construction wpt/html/syntax/parsing/resources/tests5.dat
data tree-construction wpt/html/syntax/parsing/resources/tests6.dat
data tree-construction wpt/html/syntax/parsing/resources/tests7.dat
data tree-construction wpt/html/syntax/parsing/resources/tests8.dat
data tree-construction wpt/html/syntax/parsing/resources/tests9.dat
data tree-construction wpt/html/syntax/parsing/resources/tricky01.dat
data tree-construction wpt/html/syntax/parsing/resources/void-in-phrasing.dat
data tree-construction wpt/html/syntax/parsing/resources/webkit01.dat
data tree-construction wpt/html/syntax/parsing/resources/webkit02.dat
section parsing-main-inforeign https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inforeign
open parsing-main-inforeign svg-script-processing The script end tag of an SVG script element in foreign content: processing the SVG script element, the script nesting level and the parser pause.
wpt parsing-main-inforeign html/syntax/parsing/html5lib_write.html script
wpt parsing-main-inforeign html/syntax/parsing/html5lib_write_single.html script
wpt parsing-main-inforeign html/syntax/parsing/Document.getElementsByTagName-foreign-01.html script
wpt parsing-main-inforeign html/syntax/parsing/Document.getElementsByTagName-foreign-02.html script
wpt parsing-main-inforeign html/syntax/parsing/Element.getElementsByTagName-foreign-01.html script
wpt parsing-main-inforeign html/syntax/parsing/Element.getElementsByTagName-foreign-02.html script
wpt parsing-main-inforeign html/syntax/parsing/cdata-in-integration-point-fragment.html script
wpt parsing-main-inforeign html/syntax/parsing/foreign_content_001.html script
wpt parsing-main-inforeign html/syntax/parsing/foreign_content_003.html script
wpt parsing-main-inforeign html/syntax/parsing/foreign_content_004.html script
wpt parsing-main-inforeign html/syntax/parsing/foreign_content_005.html script
wpt parsing-main-inforeign html/syntax/parsing/foreign_content_006.html script
wpt parsing-main-inforeign html/syntax/parsing/foreign_content_008.html script
wpt parsing-main-inforeign html/syntax/parsing/foreign_content_009.html script
wpt parsing-main-inforeign html/syntax/parsing/foreign_content_010.html script
wpt parsing-main-inforeign html/syntax/parsing/foreign_content_011.html script
wpt parsing-main-inforeign html/syntax/parsing/foreign_content_013.html script
wpt parsing-main-inforeign html/syntax/parsing/foreign_content_dom_properties.html script
wpt parsing-main-inforeign html/syntax/parsing/html-integration-point.html script
wpt parsing-main-inforeign html/syntax/parsing/html_content_in_foreign_context.html script
wpt parsing-main-inforeign html/syntax/parsing/math-parse01.html script
wpt parsing-main-inforeign html/syntax/parsing/math-parse03.html script
wpt parsing-main-inforeign html/syntax/parsing/unclosed-svg-script.html script
data parsing-main-inforeign wpt/html/syntax/parsing/resources/foreign-fragment.dat
data parsing-main-inforeign wpt/html/syntax/parsing/resources/math.dat
data parsing-main-inforeign wpt/html/syntax/parsing/resources/namespace-sensitivity.dat
data parsing-main-inforeign wpt/html/syntax/parsing/resources/svg.dat
section parsing-html-fragments https://html.spec.whatwg.org/multipage/parsing.html#parsing-html-fragments
open parsing-html-fragments sanitizer The parser sanitizer configuration, which only the Sanitizer API's callers pass.
wpt parsing-html-fragments html/syntax/parsing-html-fragments/fragment-parse-form-in-template-001.html script
wpt parsing-html-fragments html/syntax/parsing-html-fragments/fragment-parse-form-in-template-002.html script
wpt parsing-html-fragments html/syntax/parsing-html-fragments/fragment-parse-form-owner-001.html script
wpt parsing-html-fragments html/syntax/parsing-html-fragments/innerHTML-setter-default-namespace.xhtml script
data parsing-html-fragments wpt/html/syntax/parsing/resources/tests_innerHTML_1.dat
section the-end https://html.spec.whatwg.org/multipage/parsing.html#the-end
open the-end scripts-and-events Everything after popping the stack of open elements: document readiness, the scripts that will execute when the document has finished parsing, DOMContentLoaded, the scripts that will execute as soon as possible, the load event and pageshow.
wpt the-end html/syntax/parsing/the-end.html script
wpt the-end html/syntax/parsing/DOMContentLoaded-defer.html script
wpt the-end html/syntax/parsing/DOMContentLoaded-defer-task-order.tentative.html script
section speculative-html-parsing https://html.spec.whatwg.org/multipage/parsing.html#speculative-html-parsing
wpt speculative-html-parsing html/syntax/speculative-parsing/ script
wpt speculative-html-parsing html/syntax/speculative-charset/ script
section coercing-an-html-dom-into-an-infoset https://html.spec.whatwg.org/multipage/parsing.html#coercing-an-html-dom-into-an-infoset
open coercing-an-html-dom-into-an-infoset not-taken surf produces no XML infoset from an HTML document.
data coercing-an-html-dom-into-an-infoset html5lib-tests/tokenizer/xmlViolation.test
```

### The marks after story `html-tree-complete`

Every case that runs passes (`html-conformance: 9107 cases, 9095 run, 9095 passed`), and the 12
`not-run` cases stay out by their reasons. No section is `complete`: each has an `open` or a
`wpt` line of `html-sections`. The marks, with the harness's count (cases passed of the
section's cases) and the reasons:

- `the-input-byte-stream`: incomplete, 82/84, 2 not run (`needs-script` 1, `autodetect` 1); open
  `transport-layer-encoding`, `encodings-without-decoder`, `insertion-point`; 17 wpt lines.
- `tokenization`: incomplete, 2836/2836; 7 wpt lines.
- `named-character-references`: incomplete, 4210/4210; 1 wpt line.
- `tree-construction`: incomplete, 1803/1809, 6 not run (`needs-script`); open
  `script-processing`, `custom-element-creation`, `declarative-shadow-roots`; 137 wpt lines.
- `parsing-main-inforeign`: incomplete, 83/83; open `svg-script-processing`; 29 wpt lines.
- `parsing-html-fragments`: incomplete, 81/81; open `sanitizer`; 4 wpt lines.
- `the-end`: incomplete, no case; open `scripts-and-events`; 3 wpt lines.
- `speculative-html-parsing`: incomplete, no case; 2 wpt lines.
- `coercing-an-html-dom-into-an-infoset`: incomplete, 0/4, 4 not run (`xml-infoset`); open
  `not-taken`.

Two parts of the parser differ from the standard; no case of the pinned data reaches either,
and each is an `open` line of `tree-construction`. `template-content-patching`: the snapshot's
content patching (a `template` start tag with a `for` attribute) is not taken, so the result
differs: the standard patches the template's content into the element it marks, and surf keeps
it inert in the template contents. `selectedcontent-update-timing`: the update of a `select`'s
`selectedcontent` elements runs once at the end of a document's parse instead of from a queued
microtask; that differs in form only, as no script runs to observe the tree in between.

## 9. The inventory and the script hooks

**Confirmed**: every data file of item 3 is inventoried below with its case count and its count
per tag, and each place where parsing reaches a script is answered in the table at the end. The
data was fetched into empty directories under the session scratch directory and read there:

```sh
git clone --quiet --depth 1 https://github.com/html5lib/html5lib-tests.git html5lib-tests
git -C html5lib-tests fetch --quiet --unshallow
git -C html5lib-tests log -1 --format='%H %cI %s'
# c5495c539ca8a79c3c5107f5f14010bcff9a35d5 2026-11-05T05:20:07+01:00 Fix incorrect error order and remove obsolete tests
git clone --quiet --depth 1 --filter=blob:none --sparse https://github.com/web-platform-tests/wpt.git wpt
git -C wpt sparse-checkout set html/syntax/parsing
git -C wpt sparse-checkout add html/syntax/charset html/syntax/xmldecl html/syntax/parsing-html-fragments \
  html/syntax/speculative-charset html/syntax/speculative-parsing html/syntax/parsing-xml-fragments
git -C wpt log -1 --format='%H %cI %s'
# 13f8051342260922e50fd1f9d9877007ea025d2b 2026-11-20T16:54:49-08:00 SVGUseElement keeps stale external document when href changes to a data:URL
```

The block's format, one record per line, fields separated by one space:
`source <name> <address> <commit>` names a source and pins it; `file <group> <path> <cases>
<tag>=<n>...` is one data file, `<group>` being `tokenizer`, `tree-construction` or `encoding`
(the expected file `crates/html-conformance/expected/<group>.txt` it counts toward), `<path>`
relative to `crates/html-conformance/data/` (the source's name, then its path in the source),
`<cases>` its case count, and the `<tag>=<n>` pairs its count per tag, in the order of the `tag`
lines and only where `<n>` is above zero, the `not-run` tag written `not-run:<reason>=<n>`, and
the `<n>` of a line summing to its `<cases>`; `tag <name> <n>` is the count of one tag over all
files, `not-run` included; `total <n>` is the count of all cases, equal to the sum of the `tag`
lines and of the `<cases>` fields. Run: 9,107 cases exist, 12 are `not-run`, 9,095 run.

```html-inventory
source html5lib-tests https://github.com/html5lib/html5lib-tests c5495c539ca8a79c3c5107f5f14010bcff9a35d5
source wpt https://github.com/web-platform-tests/wpt 13f8051342260922e50fd1f9d9877007ea025d2b
file tokenizer html5lib-tests/tokenizer/contentModelFlags.test 24 text=20 charref=4
file tokenizer html5lib-tests/tokenizer/domjs.test 56 markup=4 text=47 charref=5
file tokenizer html5lib-tests/tokenizer/entities.test 80 charref=80
file tokenizer html5lib-tests/tokenizer/escapeFlag.test 9 text=8 charref=1
file tokenizer html5lib-tests/tokenizer/namedEntities.test 4210 charref=4210
file tokenizer html5lib-tests/tokenizer/numericEntities.test 336 charref=336
file tokenizer html5lib-tests/tokenizer/pendingSpecChanges.test 1 markup=1
file tokenizer html5lib-tests/tokenizer/test1.test 69 markup=35 text=13 charref=21
file tokenizer html5lib-tests/tokenizer/test2.test 46 markup=37 charref=9
file tokenizer html5lib-tests/tokenizer/test3.test 1802 markup=1513 text=240 charref=49
file tokenizer html5lib-tests/tokenizer/test4.test 85 markup=62 charref=23
file tokenizer html5lib-tests/tokenizer/unicodeChars.test 323 markup=322 charref=1
file tokenizer html5lib-tests/tokenizer/unicodeCharsProblematic.test 5 markup=5
file tokenizer html5lib-tests/tokenizer/xmlViolation.test 4 not-run:xml-infoset=4
file tree-construction wpt/html/syntax/parsing/resources/adoption01.dat 18 formatting=13 tables=3 foreign=2
file tree-construction wpt/html/syntax/parsing/resources/adoption02.dat 4 formatting=2 tables=2
file tree-construction wpt/html/syntax/parsing/resources/blocks.dat 48 body=48
file tree-construction wpt/html/syntax/parsing/resources/comments01.dat 16 body=16
file tree-construction wpt/html/syntax/parsing/resources/doctype01.dat 37 body=36 formatting=1
file tree-construction wpt/html/syntax/parsing/resources/domjs-unsafe.dat 49 body=32 tables=8 foreign=9
file tree-construction wpt/html/syntax/parsing/resources/entities01.dat 75 body=75
file tree-construction wpt/html/syntax/parsing/resources/entities02.dat 26 body=26
file tree-construction wpt/html/syntax/parsing/resources/foreign-fragment.dat 66 foreign=66
file tree-construction wpt/html/syntax/parsing/resources/html5test-com.dat 30 mixed=30
file tree-construction wpt/html/syntax/parsing/resources/inbody01.dat 4 body=4
file tree-construction wpt/html/syntax/parsing/resources/isindex.dat 4 body=4
file tree-construction wpt/html/syntax/parsing/resources/main-element.dat 3 body=2 foreign=1
file tree-construction wpt/html/syntax/parsing/resources/math.dat 8 foreign=8
file tree-construction wpt/html/syntax/parsing/resources/menuitem-element.dat 20 body=18 formatting=1 tables=1
file tree-construction wpt/html/syntax/parsing/resources/namespace-sensitivity.dat 1 foreign=1
file tree-construction wpt/html/syntax/parsing/resources/noscript01.dat 18 body=18
file tree-construction wpt/html/syntax/parsing/resources/pending-spec-changes-plain-text-unsafe.dat 1 tables=1
file tree-construction wpt/html/syntax/parsing/resources/pending-spec-changes.dat 3 mixed=3
file tree-construction wpt/html/syntax/parsing/resources/plain-text-unsafe.dat 53 body=15 tables=1 foreign=37
file tree-construction wpt/html/syntax/parsing/resources/processing-instructions.dat 124 body=119 formatting=1 tables=2 template=2
file tree-construction wpt/html/syntax/parsing/resources/quirks01.dat 4 tables=4
file tree-construction wpt/html/syntax/parsing/resources/ruby.dat 21 body=21
file tree-construction wpt/html/syntax/parsing/resources/scriptdata01.dat 26 body=26
file tree-construction wpt/html/syntax/parsing/resources/scripted_adoption01.dat 1 not-run:needs-script=1
file tree-construction wpt/html/syntax/parsing/resources/scripted_ark.dat 1 not-run:needs-script=1
file tree-construction wpt/html/syntax/parsing/resources/scripted_foster01.dat 2 not-run:needs-script=2
file tree-construction wpt/html/syntax/parsing/resources/scripted_webkit01.dat 2 not-run:needs-script=2
file tree-construction wpt/html/syntax/parsing/resources/search-element.dat 8 body=5 formatting=2 foreign=1
file tree-construction wpt/html/syntax/parsing/resources/svg.dat 8 foreign=8
file tree-construction wpt/html/syntax/parsing/resources/tables01.dat 19 tables=17 foreign=2
file tree-construction wpt/html/syntax/parsing/resources/template.dat 127 template=121 foreign=6
file tree-construction wpt/html/syntax/parsing/resources/tests1.dat 112 mixed=112
file tree-construction wpt/html/syntax/parsing/resources/tests10.dat 54 mixed=54
file tree-construction wpt/html/syntax/parsing/resources/tests11.dat 13 mixed=13
file tree-construction wpt/html/syntax/parsing/resources/tests12.dat 2 mixed=2
file tree-construction wpt/html/syntax/parsing/resources/tests14.dat 7 mixed=7
file tree-construction wpt/html/syntax/parsing/resources/tests15.dat 14 mixed=14
file tree-construction wpt/html/syntax/parsing/resources/tests16.dat 197 mixed=197
file tree-construction wpt/html/syntax/parsing/resources/tests17.dat 13 mixed=13
file tree-construction wpt/html/syntax/parsing/resources/tests18.dat 36 mixed=36
file tree-construction wpt/html/syntax/parsing/resources/tests19.dat 107 mixed=107
file tree-construction wpt/html/syntax/parsing/resources/tests2.dat 63 mixed=63
file tree-construction wpt/html/syntax/parsing/resources/tests20.dat 64 mixed=64
file tree-construction wpt/html/syntax/parsing/resources/tests21.dat 23 mixed=23
file tree-construction wpt/html/syntax/parsing/resources/tests22.dat 5 mixed=5
file tree-construction wpt/html/syntax/parsing/resources/tests23.dat 5 mixed=5
file tree-construction wpt/html/syntax/parsing/resources/tests24.dat 8 mixed=8
file tree-construction wpt/html/syntax/parsing/resources/tests25.dat 26 mixed=26
file tree-construction wpt/html/syntax/parsing/resources/tests26.dat 20 mixed=20
file tree-construction wpt/html/syntax/parsing/resources/tests3.dat 24 mixed=24
file tree-construction wpt/html/syntax/parsing/resources/tests4.dat 9 mixed=9
file tree-construction wpt/html/syntax/parsing/resources/tests5.dat 17 mixed=17
file tree-construction wpt/html/syntax/parsing/resources/tests6.dat 52 mixed=52
file tree-construction wpt/html/syntax/parsing/resources/tests7.dat 34 mixed=34
file tree-construction wpt/html/syntax/parsing/resources/tests8.dat 10 mixed=10
file tree-construction wpt/html/syntax/parsing/resources/tests9.dat 27 mixed=27
file tree-construction wpt/html/syntax/parsing/resources/tests_innerHTML_1.dat 81 foreign=81
file tree-construction wpt/html/syntax/parsing/resources/tricky01.dat 9 mixed=9
file tree-construction wpt/html/syntax/parsing/resources/void-in-phrasing.dat 13 body=11 formatting=2
file tree-construction wpt/html/syntax/parsing/resources/webkit01.dat 52 mixed=52
file tree-construction wpt/html/syntax/parsing/resources/webkit02.dat 49 mixed=49
file encoding html5lib-tests/encoding/chardet/test_big5.txt 1 not-run:autodetect=1
file encoding html5lib-tests/encoding/scripted/tests1.dat 1 not-run:needs-script=1
file encoding html5lib-tests/encoding/test-yahoo-jp.dat 1 encoding=1
file encoding html5lib-tests/encoding/tests1.dat 59 encoding=59
file encoding html5lib-tests/encoding/tests2.dat 22 encoding=22
tag markup 1979
tag text 328
tag charref 4739
tag body 476
tag formatting 22
tag tables 39
tag template 123
tag foreign 222
tag mixed 1085
tag encoding 82
tag not-run 12
total 9107
```

How it was counted: `inventory.sh <data>` printed the block above, `<data>` holding the two
fetched checkouts. It and the two programs it calls are given here as they ran; they were run
from the session scratch directory and are not committed.

`inventory.sh`:

```sh
#!/usr/bin/env bash
# The whole html-inventory block: source lines, file lines, tag lines, total line.
# Usage: inventory.sh <data dir holding html5lib-tests/ and wpt/>
set -euo pipefail
data="$1"
bin="$(cd "$(dirname "$0")" && pwd)"
h5="$data/html5lib-tests"
wpt="$data/wpt"
export LC_ALL=C

echo "source html5lib-tests https://github.com/html5lib/html5lib-tests $(git -C "$h5" rev-parse HEAD)"
echo "source wpt https://github.com/web-platform-tests/wpt $(git -C "$wpt" rev-parse HEAD)"

lines=$(
  "$bin/tokenizer-inventory.sh" "$h5"

  res="$wpt/html/syntax/parsing/resources"
  mixed="$(cd "$res" && ls tests[0-9]*.dat | tr '\n' ' ') webkit01.dat webkit02.dat tricky01.dat html5test-com.dat pending-spec-changes.dat"
  notrun="$(cd "$res" && ls scripted_*.dat | tr '\n' ' ')"
  (cd "$res" && awk -v group=tree-construction -v prefix=wpt/html/syntax/parsing/resources/ \
     -v mixed="$mixed" -v notrun="$notrun" -f "$bin/dat-inventory.awk" $(ls *.dat | sort))

  for f in $(cd "$h5" && find encoding -type f | sort); do
    case "$f" in
      encoding/scripted/*.dat)
        n=$(grep -c -x '#data' "$h5/$f"); echo "file encoding html5lib-tests/$f $n not-run:needs-script=$n" ;;
      encoding/*.dat)
        n=$(grep -c -x '#data' "$h5/$f"); echo "file encoding html5lib-tests/$f $n encoding=$n" ;;
      encoding/chardet/*)
        echo "file encoding html5lib-tests/$f 1 not-run:autodetect=1" ;;
      *) echo "UNEXPECTED $f" >&2; exit 1 ;;
    esac
  done
)
echo "$lines"

echo "$lines" | awk '
  { for (i = 5; i <= NF; i++) { split($i, kv, "="); t = kv[1]; sub(/:.*/, "", t); tag[t] += kv[2] } total += $4 }
  END {
    n = split("markup text charref body formatting tables template foreign mixed encoding not-run", order, " ")
    for (i = 1; i <= n; i++) printf "tag %s %d\n", order[i], tag[order[i]]
    printf "total %d\n", total
  }'
```

`tokenizer-inventory.sh`:

```sh
#!/usr/bin/env bash
# One `file` line per tokenizer data file: cases and the count per tag.
# A case is one test object in one initial state (initialStates, default ["Data state"]).
# Tag: charref when the input holds "&"; else markup when the state is "Data state"; else text.
# xmlViolation.test (top-level key xmlViolationTests): every case not-run:xml-infoset.
# Usage: tokenizer-inventory.sh <html5lib-tests checkout>
set -euo pipefail
root="$1"
cd "$root"
for f in $(ls tokenizer/*.test | LC_ALL=C sort); do
  if jq -e 'has("xmlViolationTests")' "$f" >/dev/null; then
    n=$(jq '.xmlViolationTests | length' "$f")
    echo "file tokenizer html5lib-tests/$f $n not-run:xml-infoset=$n"
    continue
  fi
  jq -r --arg path "html5lib-tests/$f" '
    [ .tests[] as $t
      | ($t.initialStates // ["Data state"])[] as $s
      | if ($t.input | contains("&")) then "charref"
        elif $s == "Data state" then "markup"
        else "text" end ] as $tags
    | ($tags | length) as $n
    | [ "markup", "text", "charref" ] as $order
    | [ $order[] as $o | ($tags | map(select(. == $o)) | length) as $c
        | select($c > 0) | "\($o)=\($c)" ] as $counts
    | "file tokenizer \($path) \($n) \($counts | join(" "))"' "$f"
done
```

`dat-inventory.awk`, rules 4 and 5 of item 8:

```awk
# One `file` line per tree-construction .dat file: cases and the count per tag.
# Run with LC_ALL=C (bytes, ASCII lower case). Variables: group, prefix, mixed (space-separated
# file names), notrun (space-separated file names, all of whose cases are not-run:needs-script).
#
# A line that starts with "#" and has more after it is a section heading (the rule of
# wpt html/syntax/parsing/resources/test.js, parseDat). "#description" starts a case; "#data"
# starts a case unless the section before it is "#description". A case's input is the lines of
# its "#data" section joined by LF, without the final LF.
function finish(   s, tag) {
  if (!open) return
  s = tolower(input)
  if (FILENAME_BASE in notrun_set) tag = "not-run:needs-script"
  else if (FILENAME_BASE in mixed_set) tag = "mixed"
  else {
    tag = "body"
    if (s ~ /<\/?(a|b|big|code|em|font|i|nobr|s|small|strike|strong|tt|u)[\t\n\f\r \/>]/) tag = "formatting"
    if (s ~ /<\/?(table|caption|colgroup|col|tbody|thead|tfoot|tr|td|th|select)[\t\n\f\r \/>]/) tag = "tables"
    if (s ~ /<\/?template[\t\n\f\r \/>]/) tag = "template"
    if (s ~ /<\/?(svg|math)[\t\n\f\r \/>]/ || fragment) tag = "foreign"
  }
  count[tag]++
  total++
  open = 0
}
function report(   order, i, n, out) {
  finish()
  n = split("body formatting tables template foreign mixed not-run:needs-script", order, " ")
  out = ""
  for (i = 1; i <= n; i++) if (count[order[i]] > 0) out = out " " order[i] "=" count[order[i]]
  printf "file %s %s%s %d%s\n", group, prefix, FILENAME_BASE, total, out
  delete count
  total = 0
}
BEGIN {
  split(mixed, m, " "); for (k in m) mixed_set[m[k]] = 1
  split(notrun, r, " "); for (k in r) notrun_set[r[k]] = 1
}
FNR == 1 {
  if (NR != 1) report()
  FILENAME_BASE = FILENAME; sub(/.*\//, "", FILENAME_BASE)
  open = 0; section = ""
}
/^#./ {
  heading = substr($0, 2)
  if (heading == "description" || (heading == "data" && section != "description")) {
    finish(); open = 1; fragment = 0; input = ""; ninput = 0
  }
  if (heading == "document-fragment") fragment = 1
  section = heading
  next
}
{
  if (section == "data") {
    input = (ninput == 0) ? $0 : input "\n" $0
    ninput++
  }
}
END { if (NR > 0) report() }
```

The checks, which read only the fetched files and this note, run from the repository root with
`data` set to the directory holding the two checkouts:

```sh
note=docs/design/html-parser.md; t=.out/html-parser-checks; mkdir -p $t   # $data holds html5lib-tests/ and wpt/
block() { awk -v b="$1" '$0 == "```" b {f = 1; next} /^```$/ {f = 0} f' "$note"; }
block html-inventory | awk '
  $1 == "file" { n++; c += $4; s = 0; for (i = 5; i <= NF; i++) { split($i, kv, "="); s += kv[2] } if (s != $4) bad++ }
  $1 == "tag" { t += $3 } $1 == "total" { total = $2 }
  END { printf "%d file lines, %d cases in them, %d in the tag lines, total %d, %d lines whose tags do not sum\n", n, c, t, total, bad }'
block html-inventory | awk '$1 == "file" { print $3, $4 }' | while read -r path cases; do
  case "$path" in
    */xmlViolation.test) n=$(jq '.xmlViolationTests | length' "$data/$path") ;;
    *.test) n=$(jq '[.tests[] | (.initialStates // ["Data state"]) | length] | add' "$data/$path") ;;
    */chardet/*) n=1 ;;
    *) n=$(grep -c -x '#data' "$data/$path") ;;
  esac
  [ "$n" = "$cases" ] || echo "differs: $path note $cases data $n"
done; echo "case counts compared with the data"
block html-inventory | awk '$1 == "file" { print $3 }' | sort > $t/inventory.paths
block html-sections | awk '$1 == "data" { print $3 }' | sort > $t/sections.paths
printf '%s data lines, %s distinct, %s inventory files in no data line, %s data lines not in the inventory\n' \
  "$(wc -l < $t/sections.paths)" "$(sort -u $t/sections.paths | wc -l)" \
  "$(comm -23 $t/inventory.paths $t/sections.paths | wc -l)" "$(comm -13 $t/inventory.paths $t/sections.paths | wc -l)"
block html-sections | awk '$1 == "section" { s[$2] = 1; n++ } $1 != "section" && !($2 in s) { bad++ }
  END { printf "%d sections, %d lines naming a section not declared before them\n", n, bad }'
(cd "$data/html5lib-tests" && find tokenizer -name '*.test' && find encoding -type f) | sed 's|^|html5lib-tests/|' > $t/fetched.paths
(cd "$data/wpt" && ls html/syntax/parsing/resources/*.dat) | sed 's|^|wpt/|' >> $t/fetched.paths
echo "$(sort $t/fetched.paths | comm -3 - $t/inventory.paths | wc -l) differences between the fetched data files and the inventory"
```

Their output on this note:

```
81 file lines, 9107 cases in them, 9107 in the tag lines, total 9107, 0 lines whose tags do not sum
case counts compared with the data
81 data lines, 81 distinct, 0 inventory files in no data line, 0 data lines not in the inventory
9 sections, 0 lines naming a section not declared before them
0 differences between the fetched data files and the inventory
```

**The script hooks.** Where the parsing algorithm reaches a script, what surf does instead, and
the `open` line of `html-sections` that lists it:

| where | what surf does | listed as |
|---|---|---|
| the `script` start tag in "in head" (`#parsing-main-inhead`): the element's parser document, "force async" and, for a fragment, "already started" | inserts the element and keeps none of these: a parse is of one document, so the parser document is that document; "force async" is read only by external and module scripts, which do not run (`docs/design/scripts.md`, Q2); `parse_fragment` calls no hook, as "already started" would stop each script it parses | (not a gap since `story:html-script-hook`) |
| the `script` end tag in the "text" insertion mode (`#parsing-main-incdata`): prepare the script element, the script nesting level, the parser pause flag, run a pending parsing-blocking script | pops the element and returns to the original insertion mode. With scripting enabled and a `ScriptHook` (`parse_document_with_hook`), after the token the steps of "prepare the script element" that `docs/design/scripts.md` Q2 takes decide whether the hook runs the element or reports it not run with a word of Q1; a `script` open when the parser stopped, at the end of the input or at a limit, is reported `eof`. The hook is called once for each script element the parser inserts. Parsing goes on over the tree the hook left. There is no insertion point, the script nesting level is never above 1, and the parser never pauses | `tree-construction script-processing` |
| the insertion point (`#insertion-point`) and `document.write()` | the input stream is the whole decoded body followed by the end of file; there is no insertion point and no `document.write` | `the-input-byte-stream insertion-point` |
| the SVG `script` end tag in foreign content (`#parsing-main-inforeign`) | pops the element; nothing is processed. With scripting enabled and a `ScriptHook`, the hook is told once that the element is not run, `svg`, however it is closed (its end tag, the self-closing flag, an end tag or a start tag that pops it, the end of the input), when it leaves the stack of open elements or the parse ends | `parsing-main-inforeign svg-script-processing` |
| the end (`#the-end`): document readiness, the scripts to run when parsing has finished, `DOMContentLoaded`, the scripts to run as soon as possible, the `load` event | pops every node off the stack of open elements and ends the parse with the ending command of item 2 | `the-end scripts-and-events` |
| custom element creation in "create an element for a token" (`#create-an-element-for-the-token`) | looks up no definition; `willExecuteScript` is always false; no constructor and no reaction runs | `tree-construction custom-element-creation` |
| `noscript` | parsed with parser scripting disabled true, so its content is markup and is in the tree | (not a gap: surf has scripting disabled) |

Open: everything in the `open` lines of `html-sections`; the order question of item 4 (the
encoding cases need the tree builder); comparing the number of tree-construction parse errors
(item 3).
