---
format: aep.planning-md/3
id: review-result:agent-plan-critic-acceptance-runner-round-1
kind: review-result
status: active
title: Plan critic (acceptance), round 1, epic e2e-scenario-runner
relations:
- reviews: epic:e2e-scenario-runner
revision: 1
---
needs-revision

Each acceptance in this set is one checkable command, except one story that has two sentences. Two of the determinism checks the brief asked for are not met: nothing shows that the replayed network opens no socket, and the grammar documents are missing from the formats story's acceptance.

**What I read:** 12 artifacts, the epic and all 11 `story:runner-*`, using `aep plan artifact show` for each one in full. I also ran `aep plan artifact kinds`, `lifecycle story`, `relations` and `list --kind story --status draft`, grepped the acceptance sections under `.engineering/planning/`, and checked `Taskfile.yml:171` (`.out/test.log`) and `crates/surf/src/lib.rs:88`.

**Brief checks that pass:**
- **Decision story:** `story:runner-decisions` says where the decisions go (`docs/design/scenario-runner.md`). A later story can tell they were taken from an `approval-record` that `decides` the story, which shows under `aep plan artifact show`. `decides` is a relation the store knows.
- **File modes** are asserted for the recording (`runner-record-option:53`) and for the scenario (`runner-scenario-from:52`).
- **Secrets** are checked against the recording file's bytes (`runner-record-session:67`). The scenario is held byte for byte against a golden file that contains `${PASSWORD_1}`.
- **The failing step** is named, with expected and actual lines (`runner-e2e:54`).

**Findings**

1. story:runner-replay-transport — the acceptance shows replay works when nothing is listening, but not that it opens no socket. A transport that tries the network, gets refused and then falls back to the recording passes. On this shared machine another process can also take the freed port. — `.engineering/planning/story/runner-replay-transport.md:59` "passes with its server's listener dropped before the replay starts"
   - Fix: keep the listener bound and non-blocking during the replay, and assert afterwards that `accept()` returns `WouldBlock`, meaning no connection was attempted.

2. story:runner-formats — the acceptance doesn't mention `docs/formats/surf-recording.md`, `docs/formats/surf-scenario.md`, or that the golden files cover every block and line kind those documents define. A crate with one trivial golden file and no grammar documents passes this "specify" story. — `.engineering/planning/story/runner-formats.md:69` "`cargo test -p surf-scenario` passes with one round-trip test per golden file"
   - Fix: the acceptance names both documents and says the golden recording and scenario each contain every block and line kind those documents define.

3. story:runner-run — "the same result every time" is only checked as two runs in the same environment being identical, not against a fixed golden output. A time printed to the minute, or any other value fixed per machine, passes two back-to-back runs. — `.engineering/planning/story/runner-run.md:71` "the run twice with byte-identical output". Line 48 already promises golden outputs.
   - Fix: each run case's stdout equals its golden output under `crates/surf-cli/tests/golden/run/` byte for byte. That fixes by construction what is excluded (time, session identity, absolute paths).

4. story:runner-run — the epic's "one process, with no socket and no session process" is not in the acceptance. A `surf run` that starts `surf-agent` and talks to it over the Unix socket passes all nine cases. — `.engineering/planning/epic/e2e-scenario-runner.md:35` against `.engineering/planning/story/runner-run.md:71`
   - Fix: one case runs `surf run` with no `surf-agent` reachable (for example `PATH` emptied) and still passes.

5. story:runner-e2e — the failure case requires `expected` and `actual` lines but not what they contain. Empty or wrong lines pass. — `.engineering/planning/story/runner-e2e.md:54` "a `fail` line for the `press` step and its `expected` and `actual` lines"
   - Fix: the `expected` line holds `"Sign in"` and the `actual` line holds `"Log in"`.

6. story:runner-record-session — the acceptance names no password whose form-encoded spelling differs from the typed one. With an ordinary password the two spellings are the same and the encoded-form replacement is never tested. — `.engineering/planning/story/runner-record-session.md:67` "does not contain the typed password in either spelling". The same gap is in the secret case of `runner-replay-transport`.
   - Fix: the test password contains characters that form-encoding changes, such as a space and `&`, and both spellings are asserted absent.

7. story:runner-detail-option — the acceptance doesn't give the text of the `note` line. Two stories fix two different texts for it, so either text, or both, passes. — `.engineering/planning/story/runner-detail-option.md:54` "carries the `note` line about what the detailed mode does not see yet". The two texts: "detail: attributes, classes and text only" (`runner-detail-option.md:39`) and `note detail: computed style not available` (`runner-detailed-changes.md:42`).
   - Fix: the acceptance quotes the one note line.

8. story:runner-detail-option — the acceptance is two sentences with two independent outcomes (the named session cases, and the drift test). — `.engineering/planning/story/runner-detail-option.md:55` "The drift test between `cli/surf-agent.yaml` and the parser passes."
   - Fix: make it one sentence, or drop the drift test because the parser's own test suite already covers it.

**What I could not establish**
- The crate that holds the agent's in-process core. It decides whether `runner-surf-cli-package` exists at all and what finding 4 can test. That belongs to `epic:agent-cli-target`.
- Out of my lane (design): the two different `note` texts in finding 7 are also a design conflict.
- Out of my lane (parallel safety): the e2e test finds `surf-agent` "beside" the `surf` binary, which assumes both are built when the test runs.

```findings
- file: .engineering/planning/story/runner-replay-transport.md
  line: 59
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance shows replay works when nothing is listening, but not that it opens no socket, so a transport that tries the network, gets refused and falls back to the recording passes"
- file: .engineering/planning/story/runner-formats.md
  line: 69
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance does not name the two grammar documents under docs/formats/ or require the golden files to cover every block and line kind they define, so a crate with one trivial golden file and no grammar passes"
- file: .engineering/planning/story/runner-run.md
  line: 71
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "determinism is only checked as two runs in the same environment being identical, not against a golden output, so a time printed to the minute or any other value fixed per machine passes"
- file: .engineering/planning/story/runner-run.md
  line: 71
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance does not show the epic's in-process run with no socket and no session process, so a surf run that drives surf-agent over the Unix socket passes all nine cases"
- file: .engineering/planning/story/runner-e2e.md
  line: 54
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the failure case requires expected and actual lines but not their content, so empty or wrong lines pass where the expected line should hold \"Sign in\" and the actual line \"Log in\""
- file: .engineering/planning/story/runner-record-session.md
  line: 67
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names no password whose form-encoded spelling differs from the typed one, so the two spellings can be the same and the encoded-form replacement is never tested"
- file: .engineering/planning/story/runner-detail-option.md
  line: 54
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance does not give the text of the note line, while runner-detail-option and runner-detailed-changes fix two different texts, so either text or both passes"
- file: .engineering/planning/story/runner-detail-option.md
  line: 55
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance is two sentences with two independent outcomes, the named session cases and the drift test"
```
