---
format: aep.planning-md/3
id: story:agent-cli-spec
kind: story
status: implemented
title: The agent types, the command-line document and the task that checks it
summary: surf.agent with the types the command line names, owned by net as surf.url is (D32), cli/surf-agent.yaml and task cli, so the client can start before the session model lands.
relations:
- decomposes: epic:agent-cli-target
scope:
- confidence: cited
  path: cli/surf-agent.yaml
- confidence: cited
  path: generated/rust
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/system.yaml
- confidence: cited
  path: taskfiles/cli.yml
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-11-21T01:25:58Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Decision D5 of `docs/plan/agent-plan-revision-1.md` (design critic, round 1, finding 8): the
command-line document and the task that checks it are a story of their own, so the client
(`agent-cli-client`) does not wait for `agent-session-spec`, the hardest story of the plan.

`cli/surf-agent.yaml` (`ess-cli/2`) names the types its callables take and answer, and
`ess specify cli` checks it against the specification: the types must be in `spec/`. They need
no entity, no command and no component of their own. Measured on 2026-11-21 on a scratch copy
under `.out/agent-probe/` (ess 0.96.0): `spec/` plus a domain `surf.agent` holding only the types
of the draft on branch `spec/agent-session` (`b48bc87`: `SessionId`, `SessionToken`,
`SessionCall`, `GoCall`, `OpenedSession`, `SessionReadout`, with the session state as text
because `Session.State` comes with the entity) validates, and the draft binding validates
against it (`valid CLI presentation binding`).

Decision D32 of `docs/plan/agent-plan-revision-2.md` (found in wave 1): this story was first to
add a component `agent` that owns `surf.agent` and accepts nothing, and the sixth port with it.
ess 0.96.0 generates for every component a port with a field `behaviors` that only command and
view handlers read (`ess-synth/src/rust/port.rs:158`), so a component with neither does not
build with warnings denied (`error: field 'behaviors' is never read`,
`generated/rust/crates/agent/src/lib.rs:28`). So the domain `surf.agent` lands here with types
only, owned by the component `net`, as `net` owns `surf.url` today (`spec/components.yaml:46-51`).
No component is added and the generated `System` keeps its five ports, so nothing under
`crates/` or `conformance/` is written. The component `agent`, the move of `surf.agent` to it and
the sixth port belong to `story:agent-session-spec`, which gives the component its commands.

The first attempt is parked on branch `park/agent-sixth-port` (`9e43dcf`). Its
`spec/domains/agent.yaml`, `cli/surf-agent.yaml` and `taskfiles/cli.yml` may be read with
`git show 9e43dcf -- <path>` and taken; its `spec/components.yaml`, `generated/rust/`,
`crates/`, `conformance/` and lock files are not (no merge or cherry-pick). The generated tree
of this shape was measured by the wave-1 unit in a probe on 2026-11-21: 28 artifacts, the 27 of
`main` and `crates/surf-types/src/agent.rs`, and no crate `generated/rust/crates/agent` (the
parked shape wrote 30, the two more being that crate).

The decisions of `docs/plan/agent-cli-draft-1.md` and design sections 3 and 4 that shape the
document: the commands `open [<url>]`, `go <url>`, `look`, `close`; the session from
`--session <id>` or `SURF_SESSION`; the token from `SURF_TOKEN` or `--token-file <path>`, never
an argument word; `--token-stdin` and `--token-prompt` dropped; the answer is the text format of
`agent-answer-format`, not the JSON of `ess-cli/2` (draft marker 11, settled by design section 6).

## Change

- `spec/domains/agent.yaml` (new): domain `surf.agent` with the types above, revised: session as
  text (the binding refuses `Uuid`, draft report, part 5, limit 6), `SessionToken` documented as
  64 lower-case hexadecimal digits, and a struct `surf.agent.OpenCall` (`url:
  Optional<surf.url.Url>`) that the `open` callable takes, so that `runner-record-option` adds
  `record: Optional<String>` to it under the same name. `spec/components.yaml`: `surf.agent` in
  the `owns: domains:` of the component `net`, beside `surf.url`, with a comment that it moves to
  a component `agent` once that component has commands (`agent-session-spec`, D32); no component
  is added. `spec/system.yaml`, `spec/ess-inputs.yaml`.
- `cli/surf-agent.yaml` (new), the commands and sources above. Where `ess-cli/2` has no
  environment source, an `UNMAPPED:` marker says so and names what the client reads instead.
- `task generate`: `generated/rust/` gains the module `surf-types/src/agent.rs` and no crate
  `agent`. The generated `System` still takes five ports, so `crates/ports`, `crates/surf` and
  `conformance/` are not written. `surf <url>` prints what it prints today.
- `taskfiles/cli.yml` (new, decision D16): a task `cli` that runs `ess specify cli --path spec
  --binding cli/surf-agent.yaml` and writes what `ess generate cli` makes of the document into
  `.out/cli/` (`binding.json`, `help.txt`; never built), its `dir:` the repository root so it
  runs the same alone and included. The root `Taskfile.yml` is not written
  here (decision D30): the unit report hands the coordinator its `includes:` entry for
  `taskfiles/cli.yml`, `- task: cli` in `check` and `deps: [cli]` on `test`.

Surfaces written: `spec/domains/agent.yaml`, `spec/components.yaml`, `spec/system.yaml`,
`spec/ess-inputs.yaml`, `cli/surf-agent.yaml`, `generated/rust/`, `taskfiles/cli.yml`. Not
written (D32): `crates/ports/`, anything under `crates/surf/` (its `Cargo.toml` included, so this
story no longer meets `runner-surf-cli-package` there), `conformance/`, the root `Cargo.lock`.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Layout": "`cli/surf-agent.yaml`: the `ess-cli/2` document of the agent command line, checked
  by `task cli`; the client parser is held to it." and "`spec/domains/agent.yaml`: the types the
  agent command line names (`surf.agent`), owned by `net` until a component `agent` has commands
  (D32)."
- "Commands": "`task cli`: check `cli/surf-agent.yaml` against `spec/` and write what `ess`
  reads from it into `.out/cli/`."
- "Components": the `net` row's domain column becomes `surf.net`, `surf.url`, `surf.agent`.

## Acceptance

`slot task --taskfile taskfiles/cli.yml cli` prints `cli/surf-agent.yaml — valid CLI presentation
binding` and leaves `.out/cli/binding.json`; `slot task check REFUSALS_MAX=<n>` exits 0 with `<n>`
equal to the `refused:` count, still 10, its `generated-check` step printing `generated/rust
matches spec/ (28 artifact(s) and .ess-output)` with no `generated/rust/crates/agent`, and the
scenario total unchanged from its base (51 on `main` since `1c979b4`) passed on both runners.

## Out of scope

The component `agent`, the move of `surf.agent` to it and the sixth port (`AgentPorts`,
`Browser` over six components, the six-port system in `conformance/`): `agent-session-spec`,
decision D32. The session entity, its commands and bindings (`agent-session-spec`); the parser
(`agent-cli-client`); commands of later slices, which each story adds to the document together
with the parser.

## Scope

Derived 2026-11-21 by `story-scoper`; revised after D32 on 2026-11-21 by `decomposer`. The
assembly files of the first scoping (everything under `crates/` and `conformance/`) moved to
`story:agent-session-spec` with the component.

- **Primary surface:** `spec/` plus `cli/surf-agent.yaml` (new) — cited (story, Change). The rest is generated from them or is the task that checks the document. `cli/` and `taskfiles/` do not exist on this branch.
- **Files written:**
  - `spec/domains/agent.yaml` (new): domain `surf.agent` with the six session types and `surf.agent.OpenCall` — cited (story, Change).
  - `spec/components.yaml`: `surf.agent` in the `owns: domains:` of `net`, beside `surf.url` (`spec/components.yaml:46-51`); no new component — cited (story, Change; D32).
  - `spec/system.yaml`: `surf.agent` in `domains:` — cited (story, Change).
  - `spec/ess-inputs.yaml`: `domains/agent.yaml` under `specification:` — cited (story, Change).
  - `cli/surf-agent.yaml` (new): the `ess-cli/2` document, with `UNMAPPED:` markers where `ess-cli/2` has no environment source — cited (story, Change).
  - `taskfiles/cli.yml` (new): task `cli`, `dir:` the repository root — cited (story, Change; D16 at `docs/plan/agent-plan-revision-1.md:134-136`; D30 at `docs/plan/agent-plan-revision-2.md:97-102`).
  - `generated/rust/` by `task generate` — cited (story, Change; `AGENTS.md`, "Rules"). Within it, `crates/surf-types/src/agent.rs` (new) and the files `ess` rewrites with it (`crates/surf-types/src/lib.rs`, `PLAN.md`, `plan.json`, `.ess-output/state.json`, the per-crate headers), and no `crates/agent` — inferred from the parked diff (`git show 9e43dcf --stat`) less its `agent` crate, and from the probe count of 28 artifacts (story, Context).
- **Files read, not written:**
  - `docs/plan/agent-cli-draft-1.md` and commit `b48bc87` on `spec/agent-session` — cited.
  - `docs/plan/agent-plan-revision-1.md` (D5, D7, D16) and `docs/plan/agent-plan-revision-2.md` (D30, D32) — cited.
  - `docs/design/agent-interface.md` sections 3, 4 and 6 — cited by the story's Context.
  - `spec/domains/url.yaml` — cited. The type-only domain owned by `net`; the pattern to copy.
  - Commit `9e43dcf` on `park/agent-sixth-port`: its `spec/domains/agent.yaml`, `cli/surf-agent.yaml` and `taskfiles/cli.yml`, read with `git show` — cited (story, Context; D32).
  - `generated/rust/crates/surf-system/src/lib.rs:218`: `System::new` over five ports, unchanged — inferred, because no component is added.
- **Handed to the coordinator** (D7, D16, D30, `AGENTS.md`, "Planning and waves"):
  - Root `Taskfile.yml` — cited: an `includes:` entry for `taskfiles/cli.yml`; `- task: cli` in `check`, after `generated-check`; `deps: [cli]` on `test`.
  - `REFUSALS_MAX` stays 10 — cited (story, Acceptance; D30). The unit runs `slot task check REFUSALS_MAX=10` and reports the number.
  - `AGENTS.md`, "Layout", "Commands" and "Components" — cited, the lines of the story's "For AGENTS.md".
  - Root `Cargo.lock`: no change expected — inferred. No crate is added; the parked diff changes it only for the crate `agent`.
- **Symbols:**
  - `surf.agent.OpenCall` (`url: Optional<surf.url.Url>`), `SessionId`, `SessionToken`, `SessionCall`, `GoCall`, `OpenedSession`, `SessionReadout` (ess spec types, domain `surf.agent`) — cited (story, Context and Change).
  - The generated module `surf_types::agent` — inferred from the parked `generated/rust/crates/surf-types/src/agent.rs`.
  - No hand-written symbol is added or changed; `surf_system::System::new` keeps five ports — inferred (D32).
- **Would collide with:**
  - `story:agent-session-spec` writes `spec/domains/agent.yaml`, `spec/components.yaml`, `spec/ess-inputs.yaml` and `generated/rust/` — cited (its Change). It has `depends_on story:agent-cli-spec`, so the order is fixed.
  - `story:runner-record-option` writes `cli/surf-agent.yaml`, `spec/domains/agent.yaml` and `generated/rust/` — cited. It has `depends_on story:agent-cli-spec`.
  - `story:html-document-spec`, `agent-history-spec`, `agent-links-spec`, `agent-forms-spec`, `agent-redirect-spec`, `agent-request-body-spec` and `agent-page-path-spec` write `spec/components.yaml`, `spec/ess-inputs.yaml` and `generated/rust/` — cited (their Surfaces). They come after through chains not traced here.
  - `story:agent-back-forward`, `agent-change-and-log`, `agent-fill-pick`, `agent-follow-links`, `agent-press-submit` and `runner-detail-option` write `cli/surf-agent.yaml` — cited. Their order against this story is not verified.
  - `story:runner-surf-cli-package` no longer collides: this story writes nothing under `crates/surf/` — cited (D32).
  - The wave-1 siblings `agent-answer-format`, `agent-secret-primitives`, `agent-url-parser`, `html-decisions`, `runner-decisions` and `view-decisions` share no path — cited (their typed scopes; `docs/plan/agent-waves.md:24`).
  - `story:agent-cli-client` only reads `cli/surf-agent.yaml` and `.out/cli/binding.json` and writes `crates/agent/` — cited. It has `depends_on story:agent-cli-spec`.
- **Not yet existing when it starts:**
  - Nothing from another story: it has no `depends_on` edge.
  - The types are in `b48bc87` on `spec/agent-session` and in `9e43dcf` on `park/agent-sixth-port`, not on `main`.
- **Confidence:** high — the files are the story's Change list and the generated shape was measured in a probe (28 artifacts). Which generated files `ess` rewrites besides `surf-types/src/agent.rs` is inferred.
