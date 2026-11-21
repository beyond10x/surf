# surf: agent instructions

surf is a web browser built from scratch. It is specified before it is implemented: the ESS
specification under `spec/` is the source, and nothing is implemented that it does not say.

## Goals

The operator, 2026-11-20: surf is "a compliant but super minimal and as secure as possible browser",
"even if that would mean we're dispatching every process in a micro VM". Following a standard is
wanted, not mandatory: "I'm not saying we have to follow a spec, but would be great".

- Compliant: where a web standard covers a behaviour, the specification follows it. The
  standards are the WHATWG HTML, Fetch, URL, DOM, MIME Sniffing and Encoding standards, the W3C
  CSS specifications, ECMA-262 for script, and RFC 9110 (HTTP) and RFC 8446 (TLS 1.3). The
  cross-browser test suite for them is web-platform-tests.
- Minimal: a feature is specified only when a slice needs it. A standard is followed for what
  surf does, not implemented whole.
- Secure: isolation wins over speed and memory. A component is an isolation boundary, and a
  design that needs two components to share memory is refused.

- Complete means complete. The operator, 2026-11-20: "It's fine if we mark some stuff as incomplete,
  but as soon as we mark it complete, it's complete as to the specs." Work goes slice by slice,
  and compliance is claimed per section of a standard. A section is marked complete only when
  every requirement in it is specified and the tests the standard's suite has for that section
  pass (web-platform-tests; test262 for ECMA-262). Everything else is marked incomplete. Where
  the mark is recorded is not decided yet.
- No runtime dependencies, as an experiment. The operator, 2026-11-20: "It's sort of a little experiment
  here, about not using any dependencies for runtime." This includes the script engine: "We
  could also spec it, implement it our own." Scope, decided by the operator on 2026-11-20 (option C):
  - No third-party crates in anything that ships. The Rust standard library and the operating
    system are allowed.
  - One exception: audited cryptographic primitives (ciphers, hashes, key exchange, signature
    verification) come from a third-party crate, because hand-written ones leak keys through
    timing. Which crate is not chosen yet. The TLS protocol code on top of them is written here.
  - Build and test tooling (`ess`, `task`, test suites) is not runtime and is not covered.
  - `conformance/` is the one place in the repository where a third-party crate is allowed: the
    `ess-conformance` runner and what it brings, in a Cargo workspace of its own. The root
    `Cargo.lock` proves the rest: it lists no package from outside this repository.

Status on 2026-11-20: `spec/` was drafted by reasoning over the page load. No standard was read
for it. Its names, lifecycle states and command boundaries are not checked against the standards
above, and the list of standards itself was written from memory.

## Rules

- A construct that follows a standard cites it with `refs:` on the command or outcome, and its
  names follow the standard's terms. A deliberate deviation says so in a comment beside it.

- Specs first. A behaviour gets its entity, command, event or binding in `spec/` and a green
  `task check` before any implementation code for it is written.
- A component holds its own rows and reads no other component's. Components are coupled only by
  the `bindings:` in `spec/components.yaml`. A value another component needs travels in an event.
- A type crosses a component boundary unconverted (`surf.url.Url`, `surf.net.RequestId`). No
  `conversions:`: an identity that is converted on the way into a binding gets no synthesized
  flow scenario (`ESS-SYNTH-010`, observed with ess 0.96.0 on 2026-11-20).
- A step that takes time is one command that starts it and one command per way it ends
  (`surf.net.Fetch`, then `DeliverResponse` or `FailRequest`). A command whose default outcome
  publishes the result at once makes the interpreter run the whole binding chain inside the first
  command, and every scenario that expects the state in between fails.
- An open design question is an `UNMAPPED:` comment at the place its answer would go. Do not fill
  one with a plausible value; `task open` lists them.
- Implementation code is generated from the specification with `ess generate synthesize --path
  spec --target rust`, never transcribed by hand. The generated Rust workspace is committed under
  `generated/rust/`. Hand-written crates supply only what the generated code leaves to ports.
- `generated/rust/` changes only through `task generate`, never by hand. A change to `spec/`
  runs `task generate` in the same change; `task generated-check` fails until it does.
- Generated output never goes under `spec/`. Scratch output goes to `.out/`, which git ignores.
- No Python. The operator, 2026-11-20: "Why do we have a Python server? What's that for? I hate Python."
  Read as a rule (the coordinating agent's reading of that sentence): no Python file in the
  repository, no Python in a gate, a test, a script or a manual check, and no Python in a command
  suggested to a person. Tests start their own servers in Rust (`std::net::TcpListener`). A
  manual run needs no server: `surf http://example.org/` loads a page over plain HTTP.
- rustc 1.99.1 marks `AtomicUsize::fetch_update` as deprecated (renamed `try_update`), which
  fails under `-D warnings`. Use `try_update`.
- A test that binds a Unix socket puts its directory under `$XDG_RUNTIME_DIR` when that is
  usable, never under `/tmp` and never under the checkout: a socket path may have 107 bytes, and
  a checkout path of unknown length uses them up. `crates/agent/tests/common/mod.rs` shows how.
- Commit everything. The operator, 2026-11-21: "yes, we commit all the specs, always. We commit
  everything, we integrate everything, we keep the valuable stuff." This is the standing
  exception to "commit only when asked" for this repository. Read as a rule (the coordinating
  agent's reading): a specification draft is committed on its branch when it validates, also
  when the gate would not pass with it yet; finished work is merged into `main` once
  `task check` is green on the merge; nothing worth keeping stays only in a working tree or a
  scratch directory. Since 2026-11-21 the repository is public and `main` is pushed after
  every wave and every reviewed change (see "Publishing"); a tag or a release still waits
  for his word.

## Layout

- `spec/system.yaml`: format (`ess/37`), system name `surf`, the domain list.
- `spec/components.yaml`: the six components and the ten bindings: the four of the page load and
  the six of the agent's session. The `agent` component owns `surf.agent`; `net` owns `surf.net`
  and `surf.url` only (decision D32, `docs/plan/agent-plan-revision-2.md`, is carried out).
- `spec/domains/<name>.yaml`: one file per domain. `spec/domains/agent.yaml` is the domain
  `surf.agent`: the entity `Session`, the commands `OpenSession`, `Go`, `CloseSession`,
  `NoteNavigationStarted` and `NoteNavigationEnded`, their events, the view `Sessions`, and the
  types `cli/surf-agent.yaml` names.
- `cli/surf-agent.yaml`: the `ess-cli/2` document of the agent command line, checked by
  `task cli`; the client parser is held to it.
- `taskfiles/<name>.yml`: one file per task added since the agent interface plan, included by
  the root `Taskfile.yml` with `flatten: true` and `optional: true`. Optional because four
  tests copy the root `Taskfile.yml` alone into a temporary directory and run one task there;
  a missing include must not stop them. `check` still fails when an included task is missing.
  `taskfiles/cli.yml` holds the task `cli`.
- `spec/scenarios/*.yaml`: authored scenarios (`ess-scenario/6`), for what synthesis does not
  write. Two cover the flows synthesis refuses (the page load that commits and the one that
  fails); both also read the request's address back from `Requests`. The third,
  `two-of-each-entity.yaml`, creates two windows, two tabs in one window, two page loads and two
  frames, and reads each view back with exactly two rows and both identities. Every synthesized
  scenario holds at most one row of an entity, so without it a system that gives every instance
  one identity, or a view that answers only its first row, passes the suite. The last two,
  `agent-session-reads-started-navigation.yaml` and
  `agent-session-reads-committed-navigation.yaml`, cover the outcomes `NoteNavigationStarted/noted`
  and `NoteNavigationEnded/noted`, which synthesis refuses (`ESS-SYNTH-001`, below).
- `spec/ess-inputs.yaml`: the list of every file above, and the pin `requires: ess 0.96.0`. A new
  file under `spec/` is not read until it is listed there.
- `generated/rust/`: the workspace `ess generate synthesize` writes, its own `Cargo.toml`
  included, with the record `.ess-output/state.json`. Eight crates under `crates/`: `surf-types`,
  `agent`, `shell`, `navigator`, `net`, `document`, `renderer`, `surf-system`. Its `agent` is
  the component's port; the hand-written command line is `crates/agent` (`surf-agent`).
- `Cargo.toml`: the root workspace. Its members are the globs `crates/*` and
  `generated/rust/crates/*`, so a crate joins by creating its directory. `Cargo.lock` is
  committed. `task build` and `task test` pass `--locked`, so a new crate fails them until
  `cargo generate-lockfile --offline` has run.
- `crates/<name>/`: one hand-written crate each. One that uses a generated crate depends on it
  by path, `../../generated/rust/crates/<name>`. cargo 1.99.1 refuses a member glob that matches
  nothing (measured 2026-11-20), so `crates/` always holds at least one crate. Format one crate
  with `cargo fmt -p <package>`: `cargo fmt --all` rewrites `generated/rust/`. `cargo fmt -p
  surf` also reformats four test files that are not rustfmt-clean (`review1_cli.rs`,
  `review1_output.rs`, `review1_generated_check.rs`, `review1_ports.rs`); a unit that formats
  restores the files it does not own.
  - `crates/answer` (`surf-answer`): renders an agent answer in the grammar of
    `docs/agent-answer.md` and reads it back (`parse`, `unquote`); every value from a page, a
    server or an address goes through one quoting function. No dependency.
  - `crates/secret` (`surf-secret`): the operating system random source, the comparison in
    constant time and the 32-byte token. Every secret and every random identity of the agent
    and the view comes from here. No dependency.
  - `crates/url` (`surf-url`): the WHATWG URL parser surf resolves addresses with, held to the
    web-platform-tests URL data vendored under `tests/data/`; no IDNA. No dependency.
  - `crates/fetch` (`surf-fetch`): fetches an `http:` address with the standard library only and
    returns status, media type and body, or `Unreachable`, `UnsupportedScheme` or
    `MalformedResponse`. No dependency. Its documentation lists what it does not do.
  - `crates/ports` (`surf-ports`): in-memory storage and version 4 identities for the generated
    components, one ports type per component. `AgentPorts` also meets the obligations
    `NoteNavigationStarted` and `NoteNavigationEnded`, and answers the caller's `session_id` from
    a `Caller` handle. Depends on the generated crates only.
  - `crates/surf` (`surf`): a library and the binary `surf`. The library builds the generated
    `System` from six component ports the caller supplies and adds the `net` component's
    transport, a trait with one method, as a parameter. `Browser::run` hands each
    `surf.net.RequestStarted` to the transport once and issues `DeliverResponse` or `FailRequest`
    on the `net` port only; `Browser::agent_mut` issues the agent's commands. The binary uses
    `surf-fetch` as the transport and still loads one `http:` address. Depends on the
    generated crates, `surf-ports` and `surf-fetch`.
  - `crates/agent` (`surf-agent`): the agent command line and the binary `surf-agent`. A parser
    held to `cli/surf-agent.yaml` by a test, the token read from `SURF_TOKEN` or from a token file
    the caller owns, and the wire to `surfd` over a Unix socket in `$XDG_RUNTIME_DIR/surf`. There
    is no `surfd` yet (wave 3), so it cannot open a session; its tests run against a stand-in.
    Depends on `surf-answer` and `surf-secret`.
  - `crates/view` (`surf-view`): the live view's HTTP server, standard library only: `GET` only, a
    fixed route table, the `Host` and view-token checks, and server-sent events with a bound per
    reader. `docs/view-stream.md` is what it answers. It serves no page yet. Depends on
    `surf-secret`.
  - `crates/html` (`surf-html`): the HTML parser's interface. Every entry point returns an empty
    result until the parser stories fill one module each.
  - `crates/html-conformance` (`surf-html-conformance`): the vendored html5lib-tests (commit
    `c5495c5`, 19 files, 7,134 cases, MIT) and web-platform-tests parser data (commit `13f8051`,
    62 files, 1,973 cases, 3-Clause BSD) under `data/`, each source with a `SOURCE` file of
    checksums, and a harness that prints the cases that exist, run and pass per file, per tag and
    in total. `cargo run -p surf-html-conformance` prints
    `html-conformance: 9107 cases, 9095 run, 5 passed` while the parser is empty. Nobody edits a
    file under `data/`; `data/.gitattributes` (`* -text`) keeps the CR bytes of the data.
- `conformance/` (`surf-conformance`): runs `.out/suite.json` against the Rust system in process,
  with the runner of `ess-conformance`. A Cargo workspace of its own, outside the root one, with
  its own committed `Cargo.lock` and its own `target/`. It depends on `surf`, `surf-ports` and the
  generated crates by path, and on `ess-conformance`, `ess-primitives` and `ess-domain` from
  `https://github.com/beyond10x/ess` at tag `0.96.0`; its lock pins the third-party packages
  those bring to the versions ESS's own lock holds at that tag. Nothing in the root workspace
  depends on it.
  - `src/target.rs`: `SurfTarget`, the hand-written adapter between the runner's named values
    and the generated typed commands, outcomes, view rows, events and binding invocations. A
    fresh `Browser` per scenario; a command is issued on its component's port and `Browser::run`
    delivers what it published before the answer returns. The transport never answers: the
    suite ends each request itself with `DeliverResponse` or `FailRequest`. A scenario's `setup:`
    of a `surf.agent.Session` is established by assembling the system again over
    `Setup::agent_ports`, and a command is sent as the caller the suite names. A command's direct
    events are read from what `System::published` gained while it ran, up to the first binding
    delivery, and the scenario errors where they differ from the event the typed outcome
    carries.
  - `src/main.rs`: the binary `conform-rust` runs. `tests/planted_defect.rs`: the same suite over
    a `Navigation` storage that drops every `put` must fail at least one scenario.
    `tests/two_of_each_entity.rs`: for each entity alone, one identity for every instance, and
    a view cut to its first row, must each fail at least one scenario.

## Components

| component | domain | holds | accepts |
|---|---|---|---|
| `agent` | `surf.agent` | `Session` | `OpenSession`, `Go`, `CloseSession`, `NoteNavigationStarted`, `NoteNavigationEnded` |
| `shell` | `surf.shell` | `Window`, `Tab` | `OpenWindow`, `OpenTab`, `CloseTab` |
| `navigator` | `surf.navigation` | `Navigation` | `Navigate`, `CommitNavigation`, `FailNavigation`, `CancelNavigation` |
| `net` | `surf.net`, `surf.url` | `Request` | `Fetch`, `DeliverResponse`, `FailRequest` |
| `document` | `surf.document` | `Document` | `ParseDocument` |
| `renderer` | `surf.render` | `Frame` | `PaintFrame` |

The page load: `Navigate` publishes `NavigationStarted`, which invokes `Fetch`. `DeliverResponse`
publishes `ResponseReceived`, which invokes `CommitNavigation`. `NavigationCommitted` invokes
`ParseDocument` when its media type is `text/html`. `FailRequest` publishes `FetchFailed`, which
invokes `FailNavigation`. Nothing invokes `PaintFrame` yet.

The agent's session: `Go` publishes `NavigationRequested`, which invokes `Navigate`.
`NavigationStarted` also invokes `NoteNavigationStarted`; `NavigationCommitted`,
`NavigationFailed` and `NavigationCancelled` each invoke `NoteNavigationEnded` with that state.
`SessionClosed` invokes `CloseTab`. The `Sessions` view shows each session's navigation.

A navigation is identified by the `surf.net.RequestId` of its one request.

## Implementation targets

There will be more than one (the operator, 2026-11-20: "there should be multiple"). Only the first has
code: its generated workspace and, since wave 3 of `docs/plan/waves.md`, the hand-written crates
`surf-fetch` and `surf-ports`, all built and tested by `task check`. Since wave 4, the crate
`surf` assembles them into the `surf` command line. Since wave 2 of
`docs/plan/agent-waves.md`, it assembles six component ports, and the agent command line
(`surf-agent`), the live view's server (`surf-view`) and the HTML parser's interface and
conformance harness (`surf-html`, `surf-html-conformance`) exist beside it.

- First target, chosen by the operator on 2026-11-20: Rust, all six components in one process (five
  until wave 2 added `agent`), a headless command line. No isolation in this target.
- Isolation target, after it: the same Rust code, one operating-system process per component.
  Blocked on beyond10x/ess#792 (https://github.com/beyond10x/ess/issues/792). With every
  component `reached_by: network`, ess 0.96.0 generates one `<component>-server` binary per
  component, and each binary constructs the whole system and delivers bindings inside its own
  process. The server crate also adds the third-party crates `clap`, `uuid` and `time`, and
  `validate` refuses both authored scenarios with `ESS-AUTHOR-040`, because no actor is granted
  `surf.net.DeliverResponse` or `surf.net.FailRequest`.
- After that: a micro VM per component. Nothing is explored for it.
- Agent target, required by the operator on 2026-11-20: "one implementation which is sort of a fully CLI
  based CLI-based variant used by AI agents". An agent opens a session and gets "sort of a token,
  an ID"; each later call names that session. A call answers with "a like super dense text
  representation of what can be done on this page": how to navigate, which buttons can be
  pressed, "which links are technically available, which ones are visible". "Spec first, also
  CLIs can be specified": the agent's action surface is specified and "mapped to core operations
  of that browser". Planned as `epic:agent-cli-target` (draft). ESS has a document for it:
  `ess generate cli --binding <ess-cli/1 or ess-cli/2 document>` writes "a parser and process
  adapter from a typed CLI presentation binding" (`ess generate cli --help`, ess 0.96.0; not run
  here yet). Measured since: both ESS command-line generators write code that needs third-party
  crates, so the parser is written here from the `ess-cli/2` document
  (`docs/plan/agent-cli-draft-1.md`).
- What an answer of that command line holds, added by the operator on 2026-11-21: "they not only see the
  current snapshot [...] it also shows them what led to this action, like where they have been
  before and what actually in a structural way, like semantically, has actually changed on the
  page." A "detailed mode [...] more for recording end-to-end sessions" shows each change
  itself: "a font has now changed, or a button has now gained a certain class active".
- A purpose of surf, same day: "one purpose of surf would also be to have a super fast
  end-to-end scenario runner, like test browser." An agent is sent over a site ("test that new
  feature we just implemented"), explores, clicks, fills and submits, "and then basically from
  that transcript just build a deterministically executable end-to-end test." Planned as
  `epic:e2e-scenario-runner` (draft), which needs the agent target first.
- The order for the agent target and the scenario runner, the operator, 2026-11-21: "this probably isn't
  something we just implement straight away. It's probably something requiring a design first,
  then an epic, and then a breakdown to stories." The design is `docs/design/agent-interface.md`.
  The two draft epics record what was asked and are rewritten from the accepted design.
- The daemon and the live view, the operator, 2026-11-21: "What if there is a surfd which is the surf
  daemon which is what owns or maintains the active browser sessions and [the] CLI [...] would
  just be some thin client over it and there would also be some web UI showing all the active
  sessions on that daemon and we're using real-time events [of] ESS to basically push to a nice
  UI what's currently going on semantically, not like on a HTML tag basis." The design has it in
  sections 3 and 12; the view is `epic:live-view`, oriented on the SDC run inspector (`sdc/ui`).
- The plan is accepted for implementation, same day: "I'm just accepting these new designs,
  accept all of them, put it into stories or epics, whatever, and implement it. And as soon as
  there is a first UI, a prototype of it or something, just open that UI in my Brave browser"
  (`approval-record:agent-plan-accepted-2026-11-21`). So: when a first version of the live view
  runs, the coordinating agent opens it with `/usr/bin/brave <address>`.
- A wish, stated as beside the point for now: "Ideally it would also just support this protocol
  from Google". Read as the Chrome DevTools Protocol (the coordinating agent's reading; he did
  not name it). Nothing is explored for it.
- What comes after the agent interface plan, 2026-11-21: "of course we do need SSL soon. We
  also need ways to take screenshots, which I think requires a CSS rendering engine, which is
  quite a task, I would say, but this is first principles. Yeah. Same with JavaScript engine.
  The JavaScript engine actually could also be just, I guess, its own new project. [...] It
  should be separate, I guess, from Surf itself, and we probably can just implement that
  completely independently. Requirements are for sure influenced by Surf and its needs".
  So: TLS is the next epic after this plan and may run beside it (it touches `crates/fetch`
  only); screenshots mean CSS, layout, painting, text rendering and an image encoder, each from
  first principles, design first; the script engine is a repository of its own that surf
  embeds, with its requirements written from surf's needs. None of the three has an epic yet.
- The repository becomes public, 2026-11-21: "we should start pushing the repo, make it a
  public repo, employ the beyond10x gates, and [...] add a documentation just in the style as
  all the other public ones, a public website documentation [...]. Do not integrate it into the
  beyond10x super documentation". He also asked for a logo (`docs/logo.svg`) and called the
  README good.

## Publishing

The repository is public at `https://github.com/beyond10x/surf` since 2026-11-21. The
operator chose a fresh first commit of a cleaned tree: `main` starts at `9a858e0`, made by
the bot. The history before it exists only in the local repository this work started in,
as the branch `archive/pre-public`; it is never pushed. The branch `park/agent-sixth-port`
is on that old history too, and stays local.

- Nothing personal goes into the tree: no person's name (write "the operator"), no home
  directory path (write `$HOME`), no machine name. The gates scan for secrets, private
  identifiers, home paths, unpinned workflow actions and workflows without `permissions`,
  and refuse without printing the match.
- The repository is enrolled in the common gates (`beyond10x/gates`); its hooks are
  installed in the repository (`b10x-gates ... install`). To publish `main`:

      b10x-gates --repo . --repository beyond10x/surf --policy ../gates-policy/policy.json \
        --gitleaks "$XDG_CONFIG_HOME/b10x/gates/bin/gitleaks" check --receipt <receipt.json>
      b10x-gates ... verify --receipt <receipt.json>
      b10x-gates ... publish --receipt <receipt.json> --remote-ref refs/heads/main

  The receipt file lives in the session's scratch directory. `main` is published after
  `task check` is green on it, never before.
- `crates/url/tests/data/urltestdata.json` is vendored test data whose inputs hold
  a `file:` address with a home directory path; the policy carries six exceptions for it. Replacing the file with a
  newer copy needs the exceptions made again (`b10x-gates ... policy except`).
- Every GitHub write (a release, an issue, a setting) goes through `b10x-gates api` or
  `b10x-gates gh --`, as the bot. Never raw `gh`.
- The documentation website is the repository's own (`website/`, published at
  `https://beyond10x.github.io/surf/`). It is not part of the organization's shared
  documentation: no `b10x.docs.yaml`, no shared bundle or pages workflow, no generated
  `b10x-docs:` block. The operator, 2026-11-21: "Do not integrate it into the beyond10x
  super documentation".
- Pre-built binaries, the operator, 2026-11-21: "Make sure surf comes basically from the
  start with pre-built binaries for Linux and Mac, maybe, and Windows, I don't know."
  `story:release-binaries` is the release workflow for Linux and macOS. Windows waits: the
  code reads `/dev/urandom` and the agent command line uses a Unix socket. The first release,
  `0.1.0`, exists (2026-11-21, https://github.com/beyond10x/surf/releases/tag/0.1.0);
  `docs/releasing.md` is the procedure.
- Examples in `README.md`, `docs/surf-command.md` and the website show the installed binary
  (`surf http://…`), never a build command or a path under `target/`. The operator,
  2026-11-21: "make sure that examples in the docs [...] do not contain cargo run stuff.
  Instead they should just reference the [surf] binary, which now should then also come with
  just the pre-built". Building from source is described once, for people who change surf.
- If the workflow "Documentation site" fails at "Resolve the build run that holds the
  artifact" right after a push, the build run was not listed yet (seen on 2026-11-21, 11
  seconds after the build ended): start it again as the bot,
  `b10x-gates ... gh -- workflow run b10x-docs-site.yml --ref main --repo beyond10x/surf`.
- The licence is Apache-2.0 (`LICENSE`, `NOTICE`), chosen by the coordinating agent on
  2026-11-21 at the operator's word: "choose whatever license makes sense here. I'm not caring
  about IP. We're open source. I want to share stuff. People can use it. They should mention
  me or us." Apache-2.0 because it lets anyone use and change surf, keeps the attribution in
  `NOTICE` with every copy, and is what the organization's other open repositories use. Every
  hand-written crate declares it; vendored test data keeps its own licence beside it.

An issue for ESS is filed as the organization bot, from the `ess` checkout, with the body scanned
first. `b10x-gates gh` has no `issue` command, so it goes through `api`:

    b10x-gates --policy ../gates-policy/policy.json --repository beyond10x/ess scan-text <body.md>
    b10x-gates --policy ../gates-policy/policy.json --repository beyond10x/ess api --method POST \
      --path /repos/beyond10x/ess/issues --input <issue.json> --output <response.json>

`<issue.json>` holds `title`, `body` and `labels`. The issue text names no person and no private
project; it carries a reproduction written for it.

Measured on 2026-11-20 with `ess generate synthesize --path spec --target <t>` on
ess 0.96.0. The `rust` output is committed under `generated/rust/` and compiled by `task check`;
the others were written to a scratch directory and not compiled:

- `rust`: 7 crates (8 since wave 2 added `agent`), 4,288 lines of Rust (then), path dependencies
  only, no third-party crate. It builds
  and tests with warnings denied (`RUSTFLAGS=-D warnings`, rustc 1.99.1). The
  generated crates hold no test. With the hand-written crates, `task test` ran 37 suites and
  114 tests on 2026-11-20, after review 1 (62 in `surf-fetch`, 16 in `surf-ports`, 36 in `surf`).
- `go`: 4,901 lines of Go, a `go.mod` with no `require`. Its `TARGET.md` lists 5 guarantees the
  Rust output holds and Go cannot (exhaustive matches, no zero values, and three more).
- `web`: a `wasm32-unknown-unknown` crate with `index.html` and `bridge.js`, run inside a page.
- Each reports `99 capabilities: 97 generated, 0 obligation(s), 2 refused`. The two refusals are
  the actor grants of `surf.shell.User` and `surf.navigation.User`: generated as data, enforced
  by the caller.
- `clap` is the fourth value of `--target`; it was not run.

## Planning and waves

- Work is planned in the AEP store under `.engineering/` (`aep.project/5`). Every store change
  goes through `aep plan artifact`; no file under `.engineering/planning/` is edited by hand.
  Draft bodies for `--from` go in `.engineering/drafts/`, which git ignores.
- Accepted stories are implemented in waves. The plan is `docs/plan/waves.md`; each wave has a
  page beside it. A unit's brief starts from `docs/plan/unit-invariants.md`.
- Branches: `wave/<n>-<name>` for a wave, `impl/<story-slug>` for a unit. Every checkout is a
  managed worktree (`worktree create`); nothing is edited in the primary checkout during a wave.
- An implementor leaves its changes uncommitted; the coordinator commits, merges and owns every
  store write. `Cargo.lock` is the coordinator's too: regenerated on the wave branch.
- A commit has a body and no attribution line. Every commit is the organization bot's:
  `b10x-gates --repo <tree> --repository beyond10x/surf --policy ../gates-policy/policy.json
  bot -- commit -F <file>`. The gates refuse a pushed commit with another author or
  committer, so a plain `git commit` is never used, also not for a merge:
  `... bot -- merge --no-ff <branch> -F <file>`.
- The model of a sub-agent is named in every dispatch and is never the coordinating session's
  own. The operator, 2026-11-21: "subagents should not be fabled, they should be Opus, Sonnet, or Haiku,
  depending on the task". Read as (the coordinating agent's reading): `model: opus` for an
  implementor, a reviewer, an adversary, a decomposer, a specification author and a plan critic;
  `model: sonnet` for work that follows a procedure over known files (scoping a story,
  inventories, mechanical edits with a check); `model: haiku` for a lookup. `subagent_type:
  "fork"` is not used: a fork runs on the coordinating session's model.
- The coordinating session does not implement. The operator, 2026-11-21: "not implement with Fable in
  the main session, only to the" (the sentence ends there; read as: implementation goes only to
  sub-agents). The coordinating session plans, briefs, reads diffs, runs the gate, commits,
  merges and writes plan and design documents. Code, tests and specification files are written
  by sub-agents on Opus, Sonnet or Haiku.

## Commands

Run everything through the build pool: `slot task <name>`.

- `task check`: the gate. Validates (a warning fails it), synthesizes the suite and runs it
  against the interpreter built into `ess`, then against the Rust system (`conform-rust`),
  compares the two runs' suite and total (`conform-totals`), checks `generated/rust/` against
  `spec/`, checks the root `Cargo.lock` for third-party packages (`lock-check`), then builds and
  tests the root workspace with warnings denied. Each step's failure fails it.
- `task validate`, `task suite`, `task conform`: the specification steps on their own.
- `task conform-rust`: the crate `conformance/`, warnings denied. Its tests, the planted-defect
  test included, then the suite against the Rust system: log `.out/conform-rust.log`, report
  `.out/conform-rust.json` (`ess-conformance-report/2`). Fails when any scenario failed, errored
  or was unsupported. cargo fetches `github.com/beyond10x/ess` at tag `0.96.0` when its cache
  does not hold it. `task --summary conform-rust` says more.
- `task conform-totals`: fail, printing both, when `.out/report.json` (interpreter) and
  `.out/conform-rust.json` (Rust) are of different suites by digest or differ in
  `counts.total`. Needs both runs first.
- `task lock-check`: fail, naming each as `Cargo.lock: <name> <version> from <source>`, when the
  root `Cargo.lock` holds a package with a `source` line, that is one from a registry or a git
  repository. It does not read `conformance/Cargo.lock`. A path dependency outside the
  repository has no `source` line and is not caught.
- `task generate`: rewrite `generated/rust/` from `spec/`, in place. `ess` refuses, naming the
  file, when a file it wrote was edited since; delete that file or restore it from git, then run
  it again.
- `task generated-check`: generate into `.out/generated-check/` and fail with one line per path
  that differs from `generated/rust/`, in its bytes or its executable bit. `task --summary generated-check` names what it leaves out
  and why.
- `task build`, `task test`: `cargo build` and `cargo test` of the root workspace with
  `--locked` and `RUSTFLAGS=-D warnings`.
- `task open`: every `UNMAPPED:` marker, in `spec/` and `cli/`.
- `task cli`: check `cli/surf-agent.yaml` against `spec/` and write what `ess` reads from it into
  `.out/cli/` (`binding.json`, `help.txt`). `task test` depends on it.
- Documents: `docs/agent-answer.md` (the agent answer's grammar), `docs/view-stream.md` (the live
  view's routes and event stream), `docs/design/` (accepted designs and decision notes:
  `agent-interface.md`, `scenario-runner.md`, `live-view.md`, `html-parser.md`).
- `target/debug/surf <url>`, after `task build`: load one address and print one line per row.
  `docs/surf-command.md` gives the output format and the exit statuses.
- `README.md` is for people: what surf is and is not, its principles, what works, the roadmap.
  It is the first thing a visitor reads; keep reference material out of it and in `docs/`.
  Its badges are files under `docs/badges/`, written by `sh docs/badges/make.sh .` from the
  values at the end of that script; no badge is fetched from a service. When the gate's numbers
  or the wave change, the coordinator changes the values, runs the script, and updates the
  numbers in `README.md` and in the website pages that state them (`website/docs/status.md`,
  `what-works.md`, `roadmap.md`, `how-it-is-built.md`, `principles.md`) in the same commit.

## The gate's numbers

On 2026-11-21 with ess 0.96.0, after wave 2 of the agent interface plan: `surf v1 — 9 file(s), 5
scenario(s), valid` and `77 scenario(s) (5 authored), 24 refusal(s)`. Against the interpreter:
`77 scenarios: 77 passed, 0 failed, 0 error, 0 unsupported`. Against the Rust system
(`conform-rust`): `77 scenarios: 77 passed, 0 failed, 0 error, 0 unsupported`. `conform-totals`:
`the interpreter and the Rust system ran 77 scenario(s) each`. Both reports say
`conformance_status: inconclusive`, with `execution_status: passed`: the suite carries no
coverage inventory (`coverage.knowledge: unknown`), and ess 0.96.0 never qualifies a run over
unknown coverage. The gate reads the counts. The planted defect, a `Navigation` storage that
drops every `put`, fails 30 of the 77. The crate's tests: `6 test suites: 27 passed`.
`generated-check`: `generated/rust matches spec/ (30 artifact(s) and .ess-output)`. `cli`:
`cli/surf-agent.yaml — valid CLI presentation binding`. `lock-check`: `Cargo.lock: 18 package(s),
none from outside this repository`. The root workspace: `81 test suites: 280 passed, 0 failed, 0
ignored`. In it, `surf-url` prints `urltestdata: 904 cases, 872 run, 872 passed, 32 skipped`; the
32 are hosts outside ASCII, listed in `crates/url/tests/data/SKIPPED`. Before wave 2 these were
3 authored scenarios, 51 scenarios, 10 refusals, 22 failed by the planted defect, 28
artifacts, 13 packages, 59 suites and 201 passed.

`REFUSALS_MAX` in `Taskfile.yml` is 24. `task suite` fails above it. Lower it when a refusal goes
away; raise it only with the new refusal listed here. The list is `grep '^refused:'
.out/synthesize.log`. Twenty-two are `ESS-SYNTH-010`:

- 10 × `<binding>/binding/delivery`: every binding is `at_most_once`, which has no second
  delivery to observe.
- 10 × `<binding>/binding/on-failure`: every binding is `on_failure: drop`, which publishes
  nothing.
- 2 × `<binding>/binding/flow`, for `commit-on-response` and `fail-on-fetch-failed`: synthesis
  cannot arrange a navigation and a request that share one identity. The two page-load files
  under `spec/scenarios/` cover these flows.

Two are `ESS-SYNTH-001`: the outcomes `NoteNavigationStarted/noted` and
`NoteNavigationEnded/noted` have "no three rows the filter selects that the arranging commands
can produce". The two `agent-session-reads-*` files under `spec/scenarios/` cover them.

One note is printed with them: the specification declares actors and no component is served over
a network, so no `<command>/grant/denied` scenario is generated.
