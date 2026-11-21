---
format: aep.planning-md/3
id: review-result:agent-plan-critic-acceptance-html-round-1
kind: review-result
status: active
title: Plan critic (acceptance), round 1, epic html-parser
relations:
- reviews: epic:html-parser
revision: 1
---
needs-revision

The conformance counts are what this epic is judged by, and several acceptances can be met without them: the gate only checks the test half of the completeness rule, nothing the task prints shows the per-tag counts, one story's acceptance requires a `complete` mark in advance, and the limits have no time or memory bound.

**What I read:** 19 artifacts in full (`epic:html-parser` and all 18 `story:html-*`). Commands: `aep plan artifact show` for each, `aep plan artifact kinds`, `aep plan artifact lifecycle story`, `aep plan artifact list --kind story --status draft`. I also read `AGENTS.md` "Goals" and "The gate's numbers", the `check` task in `Taskfile.yml`, and `docs/design/agent-interface.md:220-240`, at `c6a66a2`.

## Findings

1. **`epic:html-parser`** — the acceptance names no pass count, and "every case of the sections this epic marks complete passes" is true by default when no section is marked complete, so a parser that passes few cases meets it — `.engineering/planning/epic/html-parser.md:71`
   - Settle: name the closing state, which is pass = run for `tokenizer/`, `tree-construction/` and the `encoding` cases, and every section printed `complete n/n` or `incomplete` with a reason that is not a failing case.

2. **`story:html-conformance-gate`** — the check enforces only the test half of the rule in `AGENTS.md:21-26`, so a section can print `complete n/n` while the note leaves one of its requirements out or lists a web-platform-tests test for it that does not run. Examples of a left-out requirement are the insertion point (`html-tokenizer-text.md:44`) and the script steps — `.engineering/planning/story/html-conformance-gate.md:31-34`
   - Settle: the expected file records, per section, the open requirements and the web-platform-tests tests that do not run, both taken from the note. `--check` refuses `complete` while either list is non-empty, with a test for each.

3. **`story:html-conformance-gate`** — which files belong to a section is a list in the expected file that any parser story may edit, so a section can earn `complete` by dropping a failing file from its list — `.engineering/planning/story/html-conformance-gate.md:33-34`
   - Settle: the note fixes the section-to-file map, `--check` compares against it, and `--check` fails when a data file or a case that is not run belongs to no section.

4. **`story:html-conformance-gate`** — `--check` prints "one line per section … and the total line", and neither that output nor the expected files hold a per-tag count. So the "every case tagged X passing" in seven later acceptances is printed by nothing they run and protected by nothing after them — `.engineering/planning/story/html-conformance-gate.md:35-37`
   - Settle: `--check` prints `tag <name> <pass>/<run>/<exist>` lines, and the expected files hold per-tag counts that the check also refuses to see drop.

5. **`story:html-conformance-gate`** — the acceptance says the task exits 0 "printing one line per section", but the Change says the task "prints its last line" — `.engineering/planning/story/html-conformance-gate.md:53` against `:38-39`
   - Settle: either the task prints the section and tag lines, or the acceptance reads them from `.out/html-conformance.log`.

6. **`story:html-conformance-gate`** — the check compares against expected files that each parser story writes itself, so a regression passes by lowering the expected count, and no acceptance forbids a count below its value at the story's base — `.engineering/planning/story/html-conformance-gate.md:31-34`
   - Settle: `--check` also fails when an expected count is lower than in `git show <base>:crates/html-conformance/expected/<dir>.txt`, or each parser acceptance states that no expected count dropped.

7. **`story:html-conformance-harness`** — the acceptance and the test `the_case_count_per_file_equals_the_inventory` compare per-file counts and the total only, so a tag rule that moves failing cases to a later story's tag passes every check — `.engineering/planning/story/html-conformance-harness.md:53`, `:61-62`
   - Settle: add a test `the_case_count_per_tag_equals_the_inventory` and name it in the acceptance.

8. **`story:html-character-references`** — the acceptance fixes the mark in advance ("the tokenizer section printed `complete n/n`"), while its own Change allows it to stay `incomplete` (`:37`). `html-tokenizer-text.md:44` says the insertion point stays unsupported. So the story can only close by marking the section complete — `.engineering/planning/story/html-character-references.md:45-46`
   - Settle: the acceptance reads pass = run for `tokenizer/`, and the section printed `complete n/n` or `incomplete` with a reason that is not a failing case.

9. **`story:html-tree-foreign`** — its acceptance ("every tree-construction case of every tag passing except those the note does not run") is the pass condition of `html-tree-complete`. That story exists because cases are expected to still fail after `html-tree-foreign` (`html-tree-complete.md:15-19`), so one of the two cannot be told done — `.engineering/planning/story/html-tree-foreign.md:44-45`
   - Settle: `html-tree-foreign` claims the tag lines `body`, `formatting`, `tables`, `template` and `foreign` at `n/n`, and leaves the remaining cases of the mixed files to `html-tree-complete`.

10. **`story:html-parser-limits`** — the node-count and attributes-per-element limits have no number, here (`:23`) or in `html-decisions` item 7 (`html-decisions.md:95`). "The tests of `tests/limits.rs` named in Change" refers to no named test (`:38`). So neither the limits nor the test count can be checked — `.engineering/planning/story/html-parser-limits.md:52`
   - Settle: give a default for every limit, and one named test per limit that asserts the named limit at that number.

11. **`story:html-parser-limits`** — the acceptance checks only "no panic", with no time or memory bound. A quadratic run of the adoption agency algorithm over 100,000 misnested `a`, or an unbounded allocation, passes, although the epic's constraint names hangs and unbounded allocation — `.engineering/planning/story/html-parser-limits.md:52-53`
   - Settle: each adversarial test and the seeded run asserts a stated time bound and a node count no larger than the node limit.

12. **`story:html-tokenizer-markup`** — "every tokenizer case tagged `markup` passing" names no number and is read from a command whose exit status does not depend on it — `.engineering/planning/story/html-tokenizer-markup.md:42`
    - Settle: name the line `tag markup n/n`, with n the inventory count for `markup`.

13. **`story:html-tokenizer-text`** — "every tokenizer case tagged `markup` or `text` passing" has the same problem: no number, and the exit status does not depend on it — `.engineering/planning/story/html-tokenizer-text.md:36`
    - Settle: name the `markup` and `text` tag lines at `n/n`, with the inventory counts.

14. **`story:html-input-stream`** — "every case tagged `encoding` that the note runs passing" names no number. The printed `incomplete p/n` does not say whether p must equal n — `.engineering/planning/story/html-input-stream.md:47-49`
    - Settle: require `p/n` with p = n = the inventory's run count for `encoding`.

15. **`story:html-tree-body`** — "every tree-construction case tagged `body` passing" names no number and is not printed by the task — `.engineering/planning/story/html-tree-body.md:45`
    - Settle: name `tag body n/n`, with the inventory count.

16. **`story:html-tree-formatting`** — "every tree-construction case tagged `body` or `formatting` passing" names no number and is not printed by the task — `.engineering/planning/story/html-tree-formatting.md:36`
    - Settle: name both tag lines at `n/n`.

17. **`story:html-tree-tables`** — "every tree-construction case tagged `body`, `formatting` or `tables` passing" names no number and is not printed by the task — `.engineering/planning/story/html-tree-tables.md:42`
    - Settle: name the three tag lines at `n/n`.

18. **`story:html-tree-template`** — "every tree-construction case tagged `body`, `formatting`, `tables` or `template` passing" names no number and is not printed by the task — `.engineering/planning/story/html-tree-template.md:39`
    - Settle: name the four tag lines at `n/n`.

19. **`story:html-accessible-name`** — "one per name source" has no fixed count. The Change does not count its sources ("by `for` and by nesting", "a button's value or its default `Submit` and `Reset`") and names one test only — `.engineering/planning/story/html-accessible-name.md:42` against `:33`
    - Settle: list the test names.

## What I could not establish

- Whether the HTML Standard puts the insertion point inside the section the gate calls "tokenizer". I relied on the story's own sentence at `html-tokenizer-text.md:44`. Settling it is `html-decisions`' job (the section-to-file map).
- Whether web-platform-tests has parser tests that are not generated from html5lib. Also `html-decisions`' job, item 3.
- Out of my lane, for design or scope: `html-document-port` parses real pages with a tree builder that is only up to `html-tree-formatting`, before tables, templates or foreign content. And `html-conformance-gate` decides where the completeness mark lives but does not update `AGENTS.md` "Goals", which still says that is not decided.

```findings
- file: .engineering/planning/epic/html-parser.md
  line: 71
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names no pass count, and 'every case of the sections this epic marks complete passes' is true by default when no section is marked complete, so a parser that passes few cases meets it"
- file: .engineering/planning/story/html-conformance-gate.md
  line: 31
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the check enforces only the test half of the AGENTS.md completeness rule, so a section can print complete n/n while the note leaves one of its requirements out or lists a web-platform-tests test for it that does not run"
- file: .engineering/planning/story/html-conformance-gate.md
  line: 33
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "which files belong to a section is a list in the expected file that any parser story may edit, so a section can earn complete by dropping a failing file from its list"
- file: .engineering/planning/story/html-conformance-gate.md
  line: 35
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "neither --check output nor the expected files hold a per-tag count, so the 'every case tagged X passing' in seven later acceptances is printed by nothing they run and protected by nothing after them"
- file: .engineering/planning/story/html-conformance-gate.md
  line: 53
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says the task exits 0 printing one line per section, but the Change says the task prints its last line"
- file: .engineering/planning/story/html-conformance-gate.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the check compares against expected files that each parser story writes itself, so a regression passes by lowering the expected count, and no acceptance forbids a count below its value at the story's base"
- file: .engineering/planning/story/html-conformance-harness.md
  line: 53
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance and the_case_count_per_file_equals_the_inventory compare per-file counts and the total only, so a tag rule that moves failing cases to a later story's tag passes every check"
- file: .engineering/planning/story/html-character-references.md
  line: 45
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance fixes the mark in advance as complete n/n while its own Change allows it to stay incomplete and html-tokenizer-text leaves the insertion point unsupported, so the story can only close by marking the section complete"
- file: .engineering/planning/story/html-tree-foreign.md
  line: 44
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance is the pass condition of html-tree-complete, a story that exists because cases are expected to still fail after html-tree-foreign, so one of the two cannot be told done"
- file: .engineering/planning/story/html-parser-limits.md
  line: 52
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the node-count and attributes-per-element limits have no number here or in html-decisions item 7, and the acceptance refers to tests the Change does not name, so neither the limits nor the test count can be checked"
- file: .engineering/planning/story/html-parser-limits.md
  line: 53
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance checks only no panic, with no time or memory bound, so a quadratic adoption agency run or an unbounded allocation on the adversarial inputs passes, although the epic's constraint names hangs and unbounded allocation"
- file: .engineering/planning/story/html-tokenizer-markup.md
  line: 42
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'every tokenizer case tagged markup passing' names no number and is read from a command whose exit status does not depend on it"
- file: .engineering/planning/story/html-tokenizer-text.md
  line: 36
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'every tokenizer case tagged markup or text passing' names no number and is read from a command whose exit status does not depend on it"
- file: .engineering/planning/story/html-input-stream.md
  line: 47
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'every case tagged encoding that the note runs passing' names no number, and the printed incomplete p/n does not say whether p must equal n"
- file: .engineering/planning/story/html-tree-body.md
  line: 45
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'every tree-construction case tagged body passing' names no number and is not printed by the task"
- file: .engineering/planning/story/html-tree-formatting.md
  line: 36
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'every tree-construction case tagged body or formatting passing' names no number and is not printed by the task"
- file: .engineering/planning/story/html-tree-tables.md
  line: 42
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'every tree-construction case tagged body, formatting or tables passing' names no number and is not printed by the task"
- file: .engineering/planning/story/html-tree-template.md
  line: 39
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'every tree-construction case tagged body, formatting, tables or template passing' names no number and is not printed by the task"
- file: .engineering/planning/story/html-accessible-name.md
  line: 42
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'one per name source' has no fixed count, because the Change does not count its sources and names one test only"
```
