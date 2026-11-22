---
format: aep.planning-md/3
id: story:runner-record-option
kind: story
status: active
title: Specify surf-agent open --record and create the recording safely
summary: The ess-cli/2 document gains --record; open creates the file 0600, never overwriting, with its header.
relations:
- decomposes: epic:e2e-scenario-runner
- depends_on: story:runner-formats
- depends_on: story:agent-change-and-log
- depends_on: story:agent-cli-spec
- depends_on: story:agent-cli-client
- depends_on: story:agent-surfd-supervisor
- depends_on: story:agent-session-process
- depends_on: story:agent-session-events
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 5}
---
## Context

`surf-agent open --record <file>` writes the session as it runs (epic, Outcome). The command line
is specified as an `ess-cli/2` document, `cli/surf-agent.yaml` (story `agent-cli-spec`), that the
hand-written parser in `crates/agent` is held to by a drift test (story `agent-cli-client`), so
the option is specified before any code records anything. This story is the specification step
plus the thinnest slice that makes the option true under `surfd`: the path travels from the
client to the session process, the file is created safely and holds the recording header, and a
path that cannot be recorded to is refused.

**The route under `surfd`** (`docs/plan/agent-plan-revision-2.md`, D28: "The client sends an
absolute path. The refusal of a path that cannot be recorded to is the session process's answer
to `open`, with exit 2, and `surfd` lists no session for it."). `surf-agent` is a client of
`surfd`, whose working directory is not the client's, and `surfd` starts the session process
(story `agent-surfd-supervisor`). So:

- **client** (`crates/agent`, story `agent-cli-client`): makes the `--record` value absolute
  against its own working directory (`std::path::absolute`, from memory stable since Rust 1.79,
  to be confirmed against rustc 1.99.1), refuses a value that is not valid UTF-8 with exit 2
  before connecting, and sends it as the argument `record` of the `open` request of the wire
  (`surf_agent::wire`: "the request names the session (not for `open`), the command, its
  arguments and the token"), quoted with `surf-answer`'s quoting function so that the wire and
  the link stay printable ASCII (D26);
- **`surfd`** (`crates/surfd`, story `agent-surfd-supervisor`): passes the request of the wire on
  the link unchanged, as it does every request ("per request the request of the wire passed on
  unchanged"), and reads nothing of `record`;
- **session process** (`crates/session`, story `agent-session-process`): reads `record` with
  `surf_answer::unquote`, refuses a relative path, creates the file before it issues any command
  of the system, and on failure answers `open` with exit 2, the head line `surf 1 session <id>
  unknown`, `did refused usage` (a reason on the fixed list of `docs/agent-answer.md`, so the
  answer grammar does not change) and, as the wire's reason for standard error, `record
  "<path>": <the operating system's error>`; it then exits with status 2 without having opened a
  session;
- **`surfd` again**: an `open` whose answer has a non-zero exit status leaves no entry in the
  session list: `surfd` relays the answer, waits for the session process to end and removes the
  session directory, as for a session that never reported ready (`agent-surfd-supervisor`, case
  9). This is the one change this story makes in `crates/surfd`, in its supervisor code, not in
  `crates/surfd/src/view.rs`, `main.rs` or `Cargo.toml`, which the `view-*` stories write.

**Where the field lives, and D10.** `story:runner-decisions`, D10, keeps recordings out of the
model: no entity, command input or event. Every callable of `cli/surf-agent.yaml` is `local` and
names as its input a struct declared in `surf.agent` that holds what the call carries
(`agent-cli-spec`: the types "need no entity and no command"). The `record` field goes on the
struct the `open` callable names, and nowhere else: `surf.agent.OpenSession`'s input, the entity
`surf.agent.Session`, its events and views are unchanged. So the field is a command-line shape,
not an ESS command input, which is what D10 allows. If the `open` callable names no struct
(`input: null`, as on branch `spec/agent-session`), this story declares `surf.agent.OpenCall`
with `url: Optional<surf.url.Url>` and `record: Optional<String>` and points the callable at it.
The generated crate `surf-types` gains the struct or field; no command, outcome or view changes,
so no synthesized scenario changes and `conformance/src/target.rs` needs nothing.

## Change

- `cli/surf-agent.yaml`: an option `--record <file>` on `open`, mapped to the field `record`.
- `spec/domains/agent.yaml`: the field, or the struct with it, as above. `task generate`
  regenerates `generated/rust/` in the same change. The gate runs as `slot task check
  REFUSALS_MAX=<n>` (D30) with `<n>` the refusal count on the branch; no refusal is expected to
  change, and the unit reports the number.
- `crates/agent`: the parser accepts `--record <file>`; the client makes it absolute and sends it
  as above.
- `crates/session`: the record argument read, the file created with mode 0600 and without
  overwriting (`create_new`), the `surf-recording 1` header written with the writer of
  `crates/scenario` and the file kept open; the refusal as above.
- `crates/surfd`: no list entry for an `open` answered with a non-zero exit status, as above.
- Tests, numbered, the five cases of the acceptance:
  1. `record_creates_a_user_only_file_with_the_header` (`crates/session/tests/`, built
     `surf-agent`, `surfd`, `surf-session`): after `open --record <dir>/s.recording <url>` the
     file has mode 0600 and the first line `surf-recording 1`;
  2. `relative_record_path_is_resolved_by_the_client` (same binaries): `surfd` started by an
     `open` run from directory A; then `open --record r.recording` run from directory B; the file
     is `B/r.recording`, and `A/r.recording` does not exist;
  3. `record_onto_an_existing_file_is_refused` (same binaries): exit 2, the answer
     `surf 1 session <id> unknown` and `did refused usage`, standard error naming the path, the
     file's bytes unchanged;
  4. `record_into_an_unwritable_directory_is_refused` (same binaries): a directory of mode 0500;
     exit 2, `did refused usage`, no file created;
  5. `refused_record_leaves_no_session_listed` (`crates/surfd/tests/record.rs`, `surfd` run in
     the test process through its library, D29, with the built `surf-session`): after the refused
     `open` of case 3, `surfd::Sessions` holds no entry for it and its session directory is gone.

Surfaces: `cli/surf-agent.yaml`, `spec/domains/agent.yaml`, `generated/rust/`, `crates/agent/`,
`crates/session/`, `crates/surfd/` (supervisor code, `tests/record.rs` and its `mod` line in `tests/main.rs` only). Every agent
story that writes these files precedes this one: `agent-cli-spec`, `agent-cli-client`,
`agent-surfd-supervisor`, `agent-session-process`, `agent-session-events` and
`agent-change-and-log`, each a direct edge. `runner-record-session` follows it on
`crates/session`. The `view-*` stories write other files of `crates/surfd` (`src/view.rs`,
`src/main.rs`, `Cargo.toml`, `tests/view.rs`) and need no order against it.

## For README.md

In the section "Driving surf from an agent": "`surf-agent open --record <file> [<url>]` records
the session into `<file>`, a path taken relative to where you run `surf-agent`, which it creates
readable by you only and never overwrites; a file that exists or cannot be created refuses the
`open` with exit 2. A recording holds the pages the session saw and what was typed, passwords
excepted."

## Acceptance

`slot task check REFUSALS_MAX=<n>` exits 0 with the `cli/surf-agent.yaml` drift test passing over
a document that holds `--record`, `OpenSession`'s input unchanged in `spec/domains/agent.yaml`,
and the five numbered tests passing: `record_creates_a_user_only_file_with_the_header`,
`relative_record_path_is_resolved_by_the_client`, `record_onto_an_existing_file_is_refused`,
`record_into_an_unwritable_directory_is_refused` and `refused_record_leaves_no_session_listed`.

## Out of scope

Writing actions and exchanges into the recording (`runner-record-session`).

## Domain relations

Session to recording, one to zero or one, the file outliving the session: decided by
`story:runner-decisions` (Q1, D7, D10), not typed in `spec/` by design. No `ess/1` relation is
added.
