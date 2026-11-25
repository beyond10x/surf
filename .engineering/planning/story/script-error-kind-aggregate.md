---
format: aep.planning-md/3
id: story:script-error-kind-aggregate
kind: story
status: draft
title: The specification records a script that threw an AggregateError by its own word
summary: surf.document.ScriptErrorKind gains AggregateError; the scenario of every error kind records nine kinds; the generated types and the conformance target follow, with no new planted defect; the synthesis of the suite is timed against its bound.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-spec
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: conformance/src/target.rs
- confidence: cited
  path: conformance/tests/adversary_2.rs
- confidence: cited
  path: conformance/tests/noticed.txt
- confidence: cited
  path: generated/rust
- confidence: cited
  path: spec/domains/document.yaml
- confidence: cited
  path: spec/scenarios/document-records-every-error-kind.yaml
- confidence: cited
  path: website/docs/how-it-is-built.md
revision: 10
---
## Context

swell `0.9.0` adds the error kind `AggregateError`: "New: `ErrorKind::AggregateError`. A thrown
`AggregateError` object now reports this kind" (`CHANGELOG.md:181-183 @ 0.9.0`;
`crates/swell/src/host/mod.rs:46-55`, `:69-81 @ 0.9.0`, word `"AggregateError"`).
`story:swell-0-9-0` (decision 4) maps it to a kind of `surf-script`'s own `ScriptErrorKind`
(`crates/script/src/runner.rs:209-237`). The note decided that the tag change "adds the word
`AggregateError` to this list and to `ScriptErrorKind`" (`docs/design/scripts.md:451-454`).

The specification's list is a separate enum, `surf.document.ScriptErrorKind`
(`spec/domains/document.yaml:186-201` on `main` at `9934c89`): seven kinds and `Value`, with the
comment "the constructor that made the error object, read through swell's
`Agent::error_summary`" (`:186-188`). The record writer of `story:scripts-document-port` maps
`surf-script`'s kind to it; without a variant the writer has no word to store for an
`AggregateError`.

What reads the specification's enum on `main`:

- `generated/rust/crates/surf-types/src/document.rs:171-174`, written by `task generate`
  (`Taskfile.yml:293`) and compared by `task generated-check` (`Taskfile.yml:311`).
- `conformance/src/target.rs:1231-1258`: `script_error_kind` (one arm per variant, no wildcard)
  and `script_error_kind_named` (a list of the eight).
- `conformance/tests/adversary_2.rs:98-106`: `const NOT_SYNTAX: [ScriptErrorKind; 7]`, and
  `NOT_SYNTAX[..6]` at `:140` (the kinds before `Value`).
- `conformance/tests/adversary.rs:114`, `:192`, `:456`: the planted defect `UriErrorStoredAsError`,
  the one defect that stores a kind as another.
- `crates/ports/src/script.rs:119`, `:122`: compares with `Value` and `SyntaxError` only.
- `spec/scenarios/document-records-every-error-kind.yaml` (155 lines): summary "eight error kinds"
  (`:8`), eight `AddScriptEnd` steps, `counts: {at_least: 8, at_most: 8}` (`:131`), and one
  `contains` per position (`:132-155`); its head comment (`:1-4`) says "the seven `ErrorKind`
  words and `Value`".
- `conformance/tests/noticed.txt:9` lists that scenario under `FirstRowOnly(Script)`; the file's
  first line is the digest of its sources, which `task conform-rust` compares
  (`Taskfile.yml:122`). Its sources are `.out/suite.json`, every file under `conformance/src` and
  `conformance/tests` but the list itself, `conformance/Cargo.toml` and `conformance/Cargo.lock`
  (`taskfiles/conform-noticed.sh:10-14`), so a change of the scenario changes that first line.

The number of planted defects, 31, is written in `README.md:262`, `:592`, `:649`,
`AGENTS.md:1830`, `Taskfile.yml:226`, `website/docs/how-it-is-built.md:47`, `:105` and
`website/docs/principles.md:35`. The gate block holds the digest of the list's sources
(`README.md:592`, `website/docs/how-it-is-built.md:47`) and the suite's digest (`README.md:596`,
`website/docs/how-it-is-built.md:51`).

The synthesis of the suite has a bound: a refusal added to `AddScriptEnd` took it "from about 3 s
to 8 to 9 s, against a bound of 4.65 s, in every form tried", and was left out
(`.engineering/planning/story/scripts-spec.md:294-296`, branch `plan/round-2`). `task suite`
runs `ess verify conform synthesize --path spec --scenarios spec --out .out/suite.json` and holds
the refusals at `REFUSALS_MAX: 57` (`Taskfile.yml:80`, `:93-100`). A variant of an enum adds no
refusal and no guard; whether it changes the synthesis time is measured, not assumed.

## Decisions taken

1. `surf.document.ScriptErrorKind` gains `{name: AggregateError, summary: "AggregateError"}`
   after `UriError` and before `Value`. The enum's comment names the kinds as swell `0.9.0`'s
   eight `ErrorKind` words and `Value`.
2. The scenario `document-records-every-error-kind` gains a position 9 with `error_kind:
   AggregateError`, `message: "a AggregateError"`, in the pattern of positions 1 to 7 (line 9,
   column 19), before the `Value` position or after it as the unit finds the file reads best; its
   counts become `{at_least: 9, at_most: 9}`, its summary "nine error kinds", and its head
   comment "the eight `ErrorKind` words and `Value`". No new scenario file.
3. `generated/rust` is what `task generate` writes; nothing in it is edited by hand.
4. `conformance/src/target.rs`: both functions name `AggregateError`, the first with one arm, so
   the match stays exhaustive over the generated enum.
5. `conformance/tests/adversary_2.rs`: `NOT_SYNTAX` holds eight kinds, `AggregateError` before
   `Value`, and `NOT_SYNTAX[..6]` becomes `NOT_SYNTAX[..7]`, so the kinds before `Value` keep
   their meaning there.
6. No planted defect is added (the coordinating agent's decision of 2026-11-25):
   `UriErrorStoredAsError` stays the one defect that stores a kind as another, and the
   scenario's position 9 holds the new word. So the number of planted defects, 31, does not
   change, and `conformance/tests/adversary.rs` does not change. `task conform-slow WRITE=1`
   writes `conformance/tests/noticed.txt` again; it changes in its first line only, the digest of
   its sources.
7. The synthesis time is the gate of this story: the bound is 4.65 s, the number
   `story:scripts-spec` set. Over it, the unit stops before merging and reports the ten times;
   no other form of the change is tried without the coordinating agent.

## Change

- `spec/domains/document.yaml`, `spec/scenarios/document-records-every-error-kind.yaml`: decisions 1
  and 2.
- `generated/rust`: `task generate`.
- `conformance/src/target.rs`, `conformance/tests/adversary_2.rs`: decisions 4 and 5;
  `conformance/tests/noticed.txt`: its first line, decision 6.
- In the last commit, the public surfaces of acceptance 5: the gate block of `README.md` and of
  `website/docs/how-it-is-built.md`.

## Acceptance

1. `slot task validate` exits 0 with no `warning` line; `slot task generated-check` exits 0;
   `git grep -n AggregateError -- generated/rust/crates/surf-types/src/document.rs` prints the
   variant.
2. `slot task suite` exits 0, its refusals at most 57; `slot task conform` and `slot task
   conform-rust` exit 0, and each log lists `document-records-every-error-kind` as passed;
   `git grep -c -F 'command: surf.document.AddScriptEnd' --
   spec/scenarios/document-records-every-error-kind.yaml` prints 9 (8 on `main`);
   `git grep -c -F 'error_kind: AggregateError' --
   spec/scenarios/document-records-every-error-kind.yaml` prints 2 (the step and its `contains`).
3. `slot task conform-slow` (without `WRITE`) exits 0 against the committed
   `conformance/tests/noticed.txt`; `git diff --numstat <base> -- conformance/tests/noticed.txt`
   prints 1 line added and 1 deleted (its first line, the digest); `git diff --stat <base> -- conformance/tests/adversary.rs`
   prints nothing; `git grep -n -F '31 planted defects' -- README.md` prints the lines it prints on
   the base.
4. The synthesis is timed: `ess verify conform synthesize --path spec --scenarios spec --out
   .out/suite.json`, five runs on the story's base and five on its tip, each with `/usr/bin/time
   -f %e`, without `slot` (a measurement); the report lists all ten times; the largest time on the
   tip is at most 4.65 s.
5. `slot task check` exits 0; `git diff --stat` names only the files of the Change and the public
   surfaces. What changes there, in the story's last commit after merging `main`, each value from
   the `slot task check` log of its tip: the digest of the list's sources (`README.md:592`,
   `website/docs/how-it-is-built.md:47` on `main`) and the suite's digest (`README.md:596`,
   `website/docs/how-it-is-built.md:51` on `main`), and nothing else unless that log shows another
   number changed (the number of scenarios, 221 on `main`, among them); the old strings are read
   by `git grep -n` on the base and listed in the report with their new values.
6. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
   call and no network request in a test; every build and test runs through `slot`, and a job in
   the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes;
   the timing of acceptance 4 starts without `slot`; no Python; `generated/rust` stays the output
   of `task generate`, compared by `task generated-check`.

## Out of scope

The mapping from `surf-script`'s kind to this enum and the record writer
(`story:scripts-document-port`); the record of a page whose script throws an `AggregateError`,
checked end to end by `story:scripts-document-port` (its acceptance 7); the swell tag and
`surf-script`'s own `ScriptErrorKind` (`story:swell-0-9-0`); a planted defect that stores
`AggregateError` as another kind (decision 6); a refusal or guard on `AddScriptEnd`.

## Placement

Beside `story:swell-0-9-0`: the two share no file outside the public surfaces (`README.md`,
`website/docs/how-it-is-built.md`), and neither needs the other to build; whichever merges second
merges `main` first and takes its numbers from its own log. Before `story:scripts-document-port`,
which writes `conformance/src/target.rs` and `conformance/tests/noticed.txt` and maps the kind; that
story depends on this one. Before `story:conformance-lock-and-lints` (draft, `epic:headless-rust-target`),
which depends on this story: both write `conformance/src/target.rs` and the first line of
`conformance/tests/noticed.txt`, whose sources include `conformance/Cargo.lock`
(`taskfiles/conform-noticed.sh:10-14` on `main`). The specification is changed before the code
that writes the record, as `story:scripts-spec` was.

## Why `epic:scripts-in-a-page`

The epic records each script's end with its document; a page that throws an `AggregateError`
under swell `0.9.0` needs a word in that record.
