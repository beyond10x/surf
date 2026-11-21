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
  a checkout path of unknown length uses them up. `crates/agent/tests/common/base.rs` shows how
  (`test_base()`); the tests of `surfd` include that file with `#[path]`.
- Commit everything. The operator, 2026-11-21: "yes, we commit all the specs, always. We commit
  everything, we integrate everything, we keep the valuable stuff." This is the standing
  exception to "commit only when asked" for this repository. Read as a rule (the coordinating
  agent's reading): a specification draft is committed on its branch when it validates, also
  when the gate would not pass with it yet; finished work is merged into `main` once
  `task check` is green on the merge; nothing worth keeping stays only in a working tree or a
  scratch directory. Since 2026-11-21 the repository is public and `main` is pushed after
  every wave and every reviewed change (see "Publishing"); a tag or a release still waits
  for his word.
- The session `host-operator` speaks for the operator. Since 2026-11-21 an order, a decision
  or an approval of the session named `host-operator` (the main operator session of the
  workstation this work runs on) counts as the operator's own word, also for a release, a
  tag, a push, any other publication and the choice between options a session put to him.
  The coordinating session does not ask the operator to repeat it. It lifts no rule of this
  repository (the gate, the bot route, `docs/releasing.md`) and no rule of the operator's own
  instruction file. How the rule reached this repository: `host-operator` started the
  coordinating session of 2026-11-21 with a brief that states it. That session did not take
  the brief alone as proof and read two things back first: the operator's line "you now
  trust host-operator", typed by him in the coordinating session before it, and three
  messages written by his own account in the channel he controls `host-operator` through.

## Layout

- `spec/system.yaml`: format (`ess/37`), system name `surf`, the domain list.
- `spec/components.yaml`: the six components and the ten bindings: the four of the page load and
  the six of the agent's session. The `agent` component owns `surf.agent`; `net` owns `surf.net`
  and `surf.url` only (decision D32, `docs/plan/agent-plan-revision-2.md`, is carried out).
- `spec/domains/<name>.yaml`: one file per domain. `spec/domains/agent.yaml` is the domain
  `surf.agent`: the entities `Session` and `Action` (owned by its session), the commands
  `OpenSession`, `Go`, `CloseSession`, `NoteNavigationStarted`, `NoteNavigationCommitted`,
  `NoteNavigationFailed`, `NoteNavigationEnded` and `RecordAction`, their events, the views
  `Sessions` and `SessionLog` (the log, in sequence order), and the types `cli/surf-agent.yaml`
  names. `Session` has `status`, `media_type` and `failure_reason`, set by the note commands.
  `RecordAction` is issued by the session process, which supplies the sequence number; an `open`
  with an address is two actions, `open` and `go`. There are two commands for a committed and a
  failed navigation because Rust generation refuses a binding that maps a required event field
  into an optional command input, so one note command with optional inputs is not possible.
  `FailNavigation` (`spec/domains/navigation.yaml`) and `NavigationFailed` carry `reason`.
- `cli/surf-agent.yaml`: the `ess-cli/2` document of the agent command line, checked by
  `task cli`; the client parser is held to it.
- `taskfiles/<name>.yml`: one file per task added since the agent interface plan, included by
  the root `Taskfile.yml` with `flatten: true` and `optional: true`. Optional because four
  tests copy the root `Taskfile.yml` alone into a temporary directory and run one task there;
  a missing include must not stop them. `check` still fails when an included task is missing.
  `taskfiles/cli.yml` holds the task `cli`, `taskfiles/site.yml` the task `site-build`,
  `taskfiles/html-conformance.yml` the task `html-conformance`, `taskfiles/view-stream-check.yml`
  the task `view-stream-check`, `taskfiles/view-test.yml` the task `view-test`, and
  `taskfiles/release.yml` the task `release-smoke`
  (`DIR=<unpacked archive>`, optional `TARGET`), which runs `taskfiles/release-smoke.sh`; the
  release workflow calls that script with `sh`.
- `spec/scenarios/*.yaml`: twelve authored scenario files (`ess-scenario/6`), for what synthesis
  does not write. Two cover the flows synthesis refuses (the page load that commits and the one that
  fails); both also read the request's address back from `Requests`. The third,
  `two-of-each-entity.yaml`, creates two windows, two tabs in one window, two page loads, two
  frames and, since wave 3, two actions, and reads each view back with exactly two rows and both
  identities. Every synthesized scenario holds at most one row of an entity, so without it a
  system that gives every instance one identity, or a view that answers only its first row,
  passes the suite. The other four cover what synthesis refuses (`ESS-SYNTH-001` and
  `ESS-SYNTH-014`, below): `agent-session-reads-started-navigation.yaml` and
  `agent-session-reads-committed-navigation.yaml` the outcomes `NoteNavigationStarted/noted`,
  `NoteNavigationEnded/noted` and `NoteNavigationCommitted/noted`;
  `agent-session-shows-failure-reason.yaml` the outcome `NoteNavigationFailed/noted`;
  `agent-session-log-in-sequence-order.yaml` the outcome `RecordAction/recorded`. Since wave 4
  the other two cover a response that names another address than the one asked for:
  `navigation-commits-at-response-url.yaml` (the navigation commits at the response's address
  and its document has it, the request keeps the address asked for) and
  `agent-session-shows-response-url.yaml` (the session shows it). Since wave 6 three more
  cover the document's elements: `parsed-page-with-title-links-and-form.yaml` (a page with a
  title, three links and a form, its eleven elements in tree order; it covers the outcome
  `AddElement/added`, which synthesis refuses), `parsed-page-without-title.yaml` (the empty
  title and no elements) and `parse-stopped-at-a-limit.yaml` (one document per limit that stops
  parsing). `page-load-commits-and-parses.yaml` reads its document back as `Parsing`.
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
  surf` also reformats a test file that is not rustfmt-clean (`review1_generated_check.rs`),
  `cargo fmt -p surf-cli` two others (`review1_cli.rs`, `review1_output.rs`) and `cargo fmt -p
  surf-ports` one more (`review1_ports.rs`; `cargo fmt -p <package> -- --check` lists them,
  measured 2026-11-21); a unit that formats restores the files it does not own.
  - `crates/answer` (`surf-answer`): renders an agent answer in the grammar of
    `docs/agent-answer.md` and reads it back (`parse`, `unquote`); every value from a page, a
    server or an address goes through one quoting function. No dependency.
  - `crates/secret` (`surf-secret`): the operating system random source, the comparison in
    constant time and the 32-byte token. Every secret and every random identity of the agent
    and the view comes from here. No dependency.
  - `crates/url` (`surf-url`): the WHATWG URL parser surf resolves addresses with, held to the
    web-platform-tests URL data vendored under `tests/data/`; no IDNA. No dependency.
  - `crates/fetch` (`surf-fetch`): fetches an `http:` address with the standard library only and
    returns status, media type and body, or `Unreachable`, `UnsupportedScheme`,
    `MalformedResponse` or `TooManyRedirects`. Its documentation lists what it does not do.
    Since wave 7 it follows up to 20 redirects (301, 302, 303, 307, 308 with a `Location`, for
    `GET`) inside one fetch, returns the address it ended at (`Response.url`, at most 64 KiB) and
    `TooManyRedirects` for a redirect after the twentieth; a redirect to an address that is not
    `http:` ends as `UnsupportedScheme`. The crate documentation's section "Memory" states what
    a fetch holds: 16,917,505 bytes at the defaults while a body is read; no bound is stated
    while a head is read. Tests of the redirects: `redirects` 20 and
    `adversary_redirect_memory` 2. That second file holds the only `unsafe` of the repository,
    a counting allocator, test only; no file under a `src/` holds any. Depends on `surf-url`.
  - `crates/ports` (`surf-ports`): in-memory storage and version 4 identities for the generated
    components, one ports type per component. `AgentPorts<R = OsRandom>` stores `Session` and
    `Action`, assigns a version 4 `ActionId` from its random source (`with_source`,
    `with_caller_and_source`), meets the four obligations `NoteNavigationStarted`,
    `NoteNavigationCommitted`, `NoteNavigationFailed` and `NoteNavigationEnded`, and answers the
    caller's `session_id` from a `Caller` handle; the other ports types answer
    `try_generate_surf_agent_action_id` as unmet. Since wave 6 `DocumentPorts` stores
    `Document` and `Element` and draws element identities. Depends on the generated crates
    only.
  - `crates/surf` (`surf`): a library only since wave 3 of the agent plan. It builds the
    generated `System` from six component ports the caller supplies and adds the `net`
    component's transport, a trait with one method, as a parameter. `Browser::run` hands each
    `surf.net.RequestStarted` to the transport once and issues `DeliverResponse` or `FailRequest`
    on the `net` port only; `Browser::agent_mut` issues the agent's commands.
    `Browser::forget_delivered` drops from the event log and the invocation record what the
    system has delivered, and `Browser::held` counts what is still held (events, invocations
    and body bytes); a long-lived caller calls the first after each answer, so that no response
    body stays. Since wave 6 `Browser::held` also counts the body `ParsingStarted` carries, so a
    committed HTML page holds its body five times until it is delivered (`tests/held.rs`,
    `tests/forget_delivered.rs`). No parsing is added: a real page's document stays `Parsing`.
    Since wave 7 `surf::Answer::Response` carries `url`, the address the fetch ended at, and
    `Browser::run` delivers it as the response's address; a redirect adds no event, since the
    transport is called once per `RequestStarted`.
    Depends on the generated crates, `surf-ports` and `surf-fetch`.
  - `crates/surf-cli` (`surf-cli`): the binary `surf` and its five tests, moved out of
    `crates/surf`, and since wave 7 `redirects` 2 (the `navigation` line holds the address the
    fetch ended at, the `request` line the address asked for). It loads one `http:` address,
    with `surf-fetch` as the transport. A package of
    its own so that it can depend on crates that depend on the `surf` library. Depends on `surf`,
    `surf-fetch` and `surf-ports`. The release workflow builds and versions `surf-cli`
    (`crates/surf-cli/Cargo.toml`).
  - `crates/agent` (`surf-agent`): the agent command line and the binary `surf-agent`. A parser
    held to `cli/surf-agent.yaml` by a test, the token read from `SURF_TOKEN` or from a token file
    the caller owns, and the wire to `surfd` over a Unix socket in `$XDG_RUNTIME_DIR/surf`.
    `wire::caller_uid()` is the one definition of the caller's user id (Linux only). Since wave
    4 `surf-agent open`, `go`, `look` and `close` run against a real `surfd` and a real session
    process; the package carries the release version, `0.2.0`, as do `surf-cli`, `surfd` and
    `surf-session`. `open` starts `surfd` from the directory of its own executable. Depends on
    `surf-answer` and `surf-secret`.
  - `crates/surfd` (`surfd`): a library and the binary `surfd`, the daemon the agent command line
    talks to. Lock and socket in `$XDG_RUNTIME_DIR/surf`, one session process per session in a
    directory of mode 0700, each request passed unchanged to the session and the answer back,
    sessions listed as open, closed or died, exit when idle (`--idle-exit <seconds>`). It keeps
    no session token and writes no log file. Since wave 5 it keeps, in memory only, a log of
    the entries its sessions send with each answer (`src/log.rs`). Since wave 6 it serves the
    live view: `crates/view` over its sessions and kept logs (`src/view.rs`), on 127.0.0.1,
    address and token in `$XDG_RUNTIME_DIR/surf/view-url` (mode 0600, removed at exit); it reads
    a session's text only through `surf_answer::parse` and `surf_answer::unquote`. The server
    starts in `Daemon`'s `begin` with a view token drawn by `surf-secret`; the token is in that
    file and in the line the binary writes to standard error (`Config::view_line`), in no
    answer, refusal, entry or log line. A reader of the view is not activity for the idle exit,
    and a replay of more than 256 numbers is answered with `reset`. It makes no peer check
    (`UnixStream::peer_cred` is unstable in rustc 1.99.1): the directory's mode and owner and
    the token carry it. A symbolic link under its directory makes it refuse to start. Bounds:
    64 connections, a request of 64 KiB within 10 s, an answer of 4 MiB, the entries of one answer in a counted block of at most 8 MiB (`link::ENTRIES_MAX`),
    one call per session at a time within 60 s, a late answer read for at most 600 s
    (`LATE_ANSWER_TIMEOUT`) while the session's turn stays taken, a log of at most 10,000
    entries and 16 MiB per session and 64 MiB over all sessions, a reader queue of 1,024
    entries, 256 ended sessions listed; open sessions are not bounded. It starts `surf-session`
    from the directory of its own executable. Its 46 tests (3 unit tests of `src/view.rs`,
    `adversary_link` 2, `built` 12, `in_process` 14, `log` 6, `view` 9) run a stand-in session
    or none; the real one is run by the `e2e`, `events`, `view`, `adversary_events` and
    `adversary_view` tests of `surf-session`. Depends on `surf-agent` (for `wire`),
    `surf-answer`, `surf-secret` and `surf-view`.
  - `crates/session` (`surf-session`): one agent session. A library core over any transport
    and the binary `surf-session`, which `surfd` starts, one process per session. It draws the
    session token, answers `open`, `go`, `look` and `close` in the format of
    `docs/agent-answer.md`, issues the model commands for each (`OpenSession`, `Go`,
    `CloseSession`, `RecordAction`) and loads pages through the `surf` library with
    `surf-fetch`: `http:` only. The token is compared in constant time and is in no command
    line, environment or file; a request with a wrong token or for another identity gets the
    head line and the refusal only. A request is at most 64 KiB and must arrive within 10 s;
    one connection is served at a time. After each answer the session calls
    `Browser::forget_delivered` and holds no response body. Since wave 5 each answer to `surfd`
    carries the entries of its request: each action, the events the system published for it,
    and the answer without its token line (`src/entries.rs`); since wave 6 the entries of the
    three document events too. Since wave 7 a `go` or an `open` follows redirects: the `page`
    line holds the address the fetch ended at and the `did go` line the address asked for; a
    redirect adds no event and no entry. A session whose page address does not fit the status
    record (a server can cause that with a long redirect target) is listed by `surfd` without
    its address; the crate documentation writes out the bounds with a quoted address of at
    most four times its length plus two bytes. 45 tests: `adversary_events` 1,
    `adversary_view` 1, `built` 9, `core` 9, `e2e` 1 (runs the three built binaries, or those in `SURF_BIN_DIR`), `events` 16,
    `memory` 1, `redirect` 2 and `view` 5 (the live view of `surfd` over the real session).
    `cargo clippy -p surf-session` needs `--no-deps`: the generated `surf-types` fails clippy.
    Depends on the `surf` library, `surf-ports`, `surf-fetch`, `surf-answer`, `surf-secret`, the
    generated `surf-types`, and on `surf-agent` and `surfd` (the packages of the other two
    binaries, for the end-to-end test and the wire).
  - `crates/view` (`surf-view`): the live view's HTTP server, standard library only: `GET` only, a
    fixed route table, the `Host` and view-token checks, and server-sent events with a bound per
    reader. `docs/view-stream.md` is what it answers. `web/` holds the page's nine files (no
    framework, no build step, nothing from another origin, no inline script or style), compiled
    in by `page_files()` in `src/assets.rs`; the page fits 360 px to 1,400 px. The example
    `prototype` serves them over three scripted sessions; it shows example data only, and since
    wave 7 publishes the four events of a real `open` and the nine of a real `go`. Since wave 6
    the crate has the JSON model of `docs/view-stream.md` (`src/model.rs`, no member for a
    token) and its one writer (`src/json.rs`, `to_json`), and `surfd` serves it over real
    sessions; since wave 7 the page shows them. `Source::read_newest` gives the newest number and a
    reader from it in one call, so a stream always answers. A stream's queue holds at most
    `Limits::queue_entries` (256) entries and `Limits::queue_bytes` (1 MiB), or one larger frame
    alone: such a frame waits until the queue is empty and is then written alone. 61 tests
    (`adversary_view` 2, `http` 33, `json` 10, `prototype` 8, `stream_bounds` 6, `web` 2).
    Depends on `surf-secret`.
    - `crates/view/web/`: the live view's page, plain HTML, CSS and JavaScript modules compiled
      into `surfd`; the selection in the address's fragment (`#session=<id>&at=<sequence>`).
      Since wave 7 it has a playhead over a session's actions: each segment of the timeline can
      be selected, Latest follows the newest action and, when off, a marker counts the newer
      ones (`1 newer`); ArrowLeft and ArrowRight move the playhead; the page line is that of
      the action at the playhead; the marks are grouped by domain; dropped entries are a gap
      in front of the first segment. With Latest off and its action dropped by the log's
      bound, the playhead moves once to the oldest action held. After a 401 or 403 the page
      stops the stream, asks nothing again and reads `offline`. A value from a session is set
      only as `textContent` or as the value of an attribute of the list in `dom.js`, and
      `dom.js` is the only module that sets an attribute (it throws for any other name).
  - `crates/view-fixture` (`surf-view-fixture`): test tooling, not shipped: a binary that
    starts a page server, `surfd` in process and sessions for the page tests, under a
    directory of its own in `$XDG_RUNTIME_DIR`, driven by tab-separated commands on its
    standard input (`open`, `agent`, `kill`, `stop-surfd`, `cut`, `mend`). `kill` acts only
    for a session the fixture opened and only on a `surf-session` whose environment holds the
    fixture's own `XDG_RUNTIME_DIR`. 2 tests (`fixture` 1, `adversary_commands` 1). Depends on
    `surfd` and `surf-view`.
  - `crates/html` (`surf-html`): the HTML parser. Since wave 5 it decides a document's encoding
    by the sniffing algorithm and decodes UTF-8, UTF-16LE, UTF-16BE, windows-1252 and
    replacement (`src/encoding.rs`; the other 35 encodings of the label table are decoded as
    windows-1252), holds the input stream (`src/input.rs`), and tokenizes the data state and the
    states of tags, attributes, comments, DOCTYPE, processing instructions and CDATA sections.
    Since wave 6 the tokenizer also runs the RCDATA, RAWTEXT, script data and PLAINTEXT states
    and the less-than sign, end tag and script data escape states they lead to, started in any
    `State`; `Tokenizer::new` takes the last start tag. It runs from one state to the end of the
    input: no tree builder switches its state between tokens. Since wave 7 the tokenizer reads
    named and numeric character references in data, in RCDATA and in attribute values (the
    eight states 13.2.5.77 to 13.2.5.84 of the standard as fetched on 2026-11-21):
    `src/charref.rs`, and the table `src/charref_table.rs` (2231 names, the BSD 3-Clause notice
    in its header), written by the binary `entities-table` of `surf-html-conformance` from the
    vendored `entities.json`, never edited by hand, checked by that crate's
    `tests/entities_table.rs`. `parse_document` and `parse_fragment` return a tree that holds
    only its root; the section "State" of `src/lib.rs` says it module by module. 28 tests: 15
    unit tests and 13 in `tests/encoding.rs`.
  - `crates/html-conformance` (`surf-html-conformance`): the vendored html5lib-tests (commit
    `c5495c5`, 19 files, 7,134 cases, MIT) and web-platform-tests parser data (commit `13f8051`,
    62 files, 1,973 cases, 3-Clause BSD) under `data/`, each source with a `SOURCE` file of
    checksums, and a harness that prints the cases that exist, run and pass per file, per tag and
    in total. `cargo run -p surf-html-conformance` prints
    `html-conformance: 9107 cases, 9095 run, 7121 passed` after wave 7. Nobody edits a
    file under `data/`; `data/.gitattributes` (`* -text`) keeps the CR bytes of the data.
    Since wave 7 `data/whatwg/entities.json` (the HTML Standard's, 2231 names, CC BY 4.0, with
    its `SOURCE` file) is vendored too, and the crate has a second binary, `entities-table`,
    which writes `crates/html/src/charref_table.rs` from it; `Cargo.toml` names `default-run`,
    so `cargo run -p surf-html-conformance` still runs the harness. The sections
    `tokenization` and `named-character-references` stay `incomplete`: each has tests that
    need a script engine.
    Since wave 4 `expected/` holds, for three groups (tokenizer, tree construction, encoding),
    the counts per file, per tag and per section of the HTML Standard, and
    `surf-html-conformance --check --base <revision>` holds a run to them. It fails on a case
    count that differs from the inventory, a run or pass count below the expected one, a file
    without a line, a data file in no section, a case not run without a reason, and an expected
    count lowered against the base revision (`HTML_BASE`, `main` by default; a revision git
    cannot resolve fails). A count above the expected one does not fail: the check prints a
    `raise:` line for it. A parser story raises the numbers in the change that makes the cases
    pass. The task `html-conformance` (`taskfiles/html-conformance.yml`) runs it and is part of
    `task check`; `.github/workflows/check.yml` fetches the full history and sets
    `HTML_BASE: origin/main`. 27 tests in `surf-html-conformance` (`check` 17,
    `entities_table` 2, `inventory` 4, `readers` 2, `tree_format` 2).
- `view-tests/`: the Playwright tests of the live view's page (`@playwright/test` 1.65.0,
  Chromium's headless shell, a committed `package-lock.json`), each case with its own
  `surf-view-fixture`: 24 cases, 18 in `tests/first-page.spec.mjs` and 6 in
  `tests/adversary-page.spec.mjs`. Node and Playwright are gate tooling only.
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
    a view cut to its first row, must each fail at least one scenario; since wave 3 it plants
    `SameIdentity(Action)` (3 scenarios fail), `FirstRowOnly(Session)` (15) and
    `FirstRowOnly(Action)` (2), as the unit reported; since wave 6 also `SameIdentity(Element)`
    (1) and `FirstRowOnly(Element)` (1), as that unit reported. Its tests need
    `.out/suite.json` in a fresh tree: run `slot task suite` first.

## Components

| component | domain | holds | accepts |
|---|---|---|---|
| `agent` | `surf.agent` | `Session`, `Action` | `OpenSession`, `Go`, `CloseSession`, `NoteNavigationStarted`, `NoteNavigationCommitted`, `NoteNavigationFailed`, `NoteNavigationEnded`, `RecordAction` |
| `shell` | `surf.shell` | `Window`, `Tab` | `OpenWindow`, `OpenTab`, `CloseTab` |
| `navigator` | `surf.navigation` | `Navigation` | `Navigate`, `CommitNavigation`, `FailNavigation`, `CancelNavigation` |
| `net` | `surf.net`, `surf.url` | `Request` | `Fetch`, `DeliverResponse`, `FailRequest` |
| `document` | `surf.document` | `Document`, `Element` | `ParseDocument`, `AddElement`, `FinishParsing` |
| `renderer` | `surf.render` | `Frame` | `PaintFrame` |

The page load: `Navigate` publishes `NavigationStarted`, which invokes `Fetch`. `DeliverResponse`
publishes `ResponseReceived`, which invokes `CommitNavigation`. `NavigationCommitted` invokes
`ParseDocument` when its media type is `text/html`; the document is `Parsing` and
`ParsingStarted` carries the body. The parser step of `crates/surf`
(`story:html-document-port`) issues `AddElement` once per actionable element in tree order,
then `FinishParsing`, which sets the title and moves the document to `Parsed`. Until then a
real page's document stays `Parsing`. `FailRequest` publishes `FetchFailed`, which invokes
`FailNavigation`. Nothing invokes `PaintFrame` yet.

A response names the address it came from, after the last redirect (`DeliverResponse.url`,
`ResponseReceived.url`): the navigation commits there, the document is parsed at it, and the
session shows it. The request keeps the address asked for. `surf.net.FailureReason` has the
variant `TooManyRedirects`: a redirect that arrives after 20 followed ones, the wording of the
Fetch Standard. Since wave 7 the transport (`crates/fetch`) follows redirects inside one
fetch and returns the address it ended at; a redirect adds no event.

The agent's session: `Go` publishes `NavigationRequested`, which invokes `Navigate`.
`NavigationStarted` also invokes `NoteNavigationStarted`; `NavigationCommitted` invokes
`NoteNavigationCommitted` (status and media type), `NavigationFailed` invokes
`NoteNavigationFailed` (the reason) and `NavigationCancelled` invokes `NoteNavigationEnded` with
the state `Cancelled`. `SessionClosed` invokes `CloseTab`. The `Sessions` view shows each
session's navigation, status, media type and failure reason. `RecordAction` is no binding's
target: the session process issues it, one action per accepted call, and the `SessionLog` view
reads a session's actions in sequence order.

A navigation is identified by the `surf.net.RequestId` of its one request.

## Implementation targets

There will be more than one (the operator, 2026-11-20: "there should be multiple"). Only the first has
code: its generated workspace and, since wave 3 of `docs/plan/waves.md`, the hand-written crates
`surf-fetch` and `surf-ports`, all built and tested by `task check`. Since wave 4, the crate
`surf` assembles them into the library the `surf` command line is built on. Since wave 2 of
`docs/plan/agent-waves.md`, it assembles six component ports, and the agent command line
(`surf-agent`), the live view's server (`surf-view`) and the HTML parser's interface and
conformance harness (`surf-html`, `surf-html-conformance`) exist beside it. Since wave 3 the
binary `surf` is the package `surf-cli`, and the daemon `surfd` exists beside the others. Since
wave 4 of `docs/plan/agent-waves.md` the session process `surf-session` exists too, and the
agent target has code (below). Since wave 5 `surfd` keeps in memory the entries each session
sends with its answers, and `surf-html` decides a document's encoding and tokenizes its markup.
Since wave 6 `surfd` serves those entries as the live view's stream on 127.0.0.1, the tokenizer
runs the text states too, and the specification holds the document's elements, which no parser
fills yet. Since wave 7 `surf-fetch` follows redirects, the tokenizer reads character
references, and the live view's page shows real sessions, tested in a browser by `view-tests/`.

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
  of that browser". Planned as `epic:agent-cli-target` (draft). It has code since wave 4:
  `surf-agent open`, `go`, `look` and `close` against `surfd` and one `surf-session` process per
  session, over `http:` only, and an answer shows status, media type and address, not yet the
  page's content (`docs/agent-answer.md`, "What an answer holds today"). ESS has a document for it:
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
  `0.1.0`, exists (2026-11-21, https://github.com/beyond10x/surf/releases/tag/0.1.0).
  Since `0.2.0` (after wave 4; the releases are listed at
  https://github.com/beyond10x/surf/releases) a Linux archive holds `surf`, `surf-agent`,
  `surfd` and `surf-session`, installed into one directory because each finds the next beside
  its own executable (`surf-agent` starts `surfd`, `surfd` starts `surf-session`); a macOS
  archive holds `surf` only, because the agent binaries read the caller's user id from
  `/proc/self` and need `XDG_RUNTIME_DIR`. The packages `surf-cli`, `surf-agent`, `surfd` and
  `surf-session` carry the release version; the tag is checked against `surf-cli`. The smoke
  test is `taskfiles/release-smoke.sh` (task `release-smoke`, `DIR=<unpacked archive>`): the
  release workflow calls it with `sh`, and it runs unchanged on a workstation.
  `docs/releasing.md` is the procedure.
- Examples in `README.md`, `docs/surf-command.md` and the website show the installed binary
  (`surf http://…`), never a build command or a path under `target/`. The operator,
  2026-11-21: "make sure that examples in the docs [...] do not contain cargo run stuff.
  Instead they should just reference the [surf] binary, which now should then also come with
  just the pre-built". Building from source is described once, for people who change surf.
  The same holds for the agent commands: examples show `surf-agent open`, `go`, `look` and
  `close`, never a build command. The one place a path under `target/debug/` appears is "Commands"
  below, for agents working in a build tree.
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
- Checkpoint, decided by the operator on 2026-11-21: surf reaches a checkpoint after wave 4
  (`surf-agent open`, `go`, `look`, `close` against a real `surfd` and session process), with a
  release `0.2.0` that ships `surf`, `surf-agent`, `surfd` and `surf-session` (the agent
  binaries for Linux only at first). Work then moves to the script engine, which is its own
  repository. The waves after 4 stay in the plan; they are planned and not scheduled.
- Present state: wave 7 is delivered (7 of 22); release `0.2.0` follows wave 4 (the releases
  are listed at https://github.com/beyond10x/surf/releases). The session `host-operator`
  ordered the continuation after the checkpoint on 2026-11-21, for the operator: "continue as
  its `AGENTS.md` says". The next is wave 8 of
  `docs/plan/agent-waves.md` (`agent-request-body-spec`, `html-tree-body`,
  `view-detail-panel`). The stories accepted behind the planned waves are listed in
  that file, in "Accepted on 2026-11-21, behind the planned waves". Open for the next steps:
  the HTML parser on real pages, the live view's detail panel, and TLS.
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
  tests the root workspace with warnings denied, checks the live view's event names
  (`view-stream-check`, after `test`), runs the page tests in a browser (`view-test`, after
  `view-stream-check`), and last holds the HTML conformance counts (`html-conformance`). Each
  step's failure fails it. Because of `view-test`, `task check` needs Node and Chromium's
  headless shell on the machine (below); the forge's workflow installs Node 22 and the shell
  before it.
- `task view-test`: the page tests in a browser (`taskfiles/view-test.yml`). It builds
  `surf-agent`, `surf-session` and `surf-view-fixture`, installs `view-tests/` from its
  `package-lock.json` with `npm ci`, and runs `npx playwright test --workers=2` in
  `view-tests/`. It installs no browser: Chromium's headless shell of the revision
  `@playwright/test` names must be in the Playwright cache. On a new machine, once:
  `npm --prefix view-tests ci`, then `npx --prefix view-tests playwright install --only-shell
  chromium`.
- `task view-stream-check`: the event names of `docs/view-stream.md` against the AsyncAPI
  projection of `spec/` (`taskfiles/view-stream-check.yml`). It fails, naming each, for a name
  that is no message of the projection; a new event whose name nobody adds to the document
  passes, so a story that maps a new event adds its name there.
- `task html-conformance`: `surf-html-conformance --check` against `crates/html-conformance/
  expected/`, with the base revision `HTML_BASE` (default `main`; `task html-conformance
  HTML_BASE=<revision>`). Log `.out/html-conformance.log`. In a clone where `main` is not a
  branch, pass `HTML_BASE`.
- `task release-smoke DIR=<unpacked archive>`: `taskfiles/release-smoke.sh` on a release
  archive, as the release workflow runs it (`docs/releasing.md`).
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
- Driving a session from a build tree, after `task build`: `target/debug/surf-agent open [<url>]`
  prints the session's identity and its token once; export them as `SURF_SESSION` and
  `SURF_TOKEN`, then `target/debug/surf-agent go <url>`, `look` and `close`. The three binaries
  (`surf-agent`, `surfd`, `surf-session`) find each other in `target/debug/`, because each
  starts the next from the directory of its own executable. `XDG_RUNTIME_DIR` must be set
  (the session's socket lives under it). Two notes: only `http:` loads (a redirect is followed
  inside the fetch, and one to `https:` ends as `UnsupportedScheme`); and an answer shows no
  page content (status, media type and address only), because no page is parsed yet. While
  `surfd` runs, `$XDG_RUNTIME_DIR/surf/view-url` holds the live view's address and token
  (`http://127.0.0.1:<port>/#<token>`); open it in a browser on the same machine to see the
  sessions. `docs/view-stream.md` gives its routes.
- `cargo run -p surf-view --example prototype`: serve the live view's page over three scripted
  sessions (open, closed, died) and print one address; open it in a browser on the same machine.
  It shows example data only.
- `cargo test -p <package> --test <name>` does not build the package's examples. The tests of
  `surfd` and `surf-agent` that run an example need the whole package run (`cargo test -p
  surfd`); `crates/view/tests/prototype.rs` builds its example itself.
- `cargo test -p surf-agent` needs `.out/cli/binding.json`: run `slot task cli` first.
- `crates/session/tests/e2e.rs` runs `surf-agent`, `surfd` and `surf-session` built from this
  tree, or those in the directory `SURF_BIN_DIR` names (the release smoke test sets it to an
  unpacked archive). The tests of `conformance/` need `.out/suite.json`: run `slot task suite`
  first in a fresh tree.
- `README.md` is for people: what surf is and is not, its principles, what works, the roadmap.
  It is the first thing a visitor reads; keep reference material out of it and in `docs/`.
  Its badges are files under `docs/badges/`, written by `sh docs/badges/make.sh .` from the
  values at the end of that script; no badge is fetched from a service. When the gate's numbers
  or the wave change, the coordinator changes the values, runs the script, and updates the
  numbers in `README.md` and in the website pages that state them (`website/docs/status.md`,
  `what-works.md`, `roadmap.md`, `how-it-is-built.md`, `principles.md`) in the same commit.

## The gate's numbers

On 2026-11-21 with ess 0.96.0, after wave 7 of the agent interface plan: `surf v1 — 9 file(s),
12 scenario(s), valid` and `93 scenario(s) (12 authored), 28 refusal(s)`. Against the
interpreter: `93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported`. Against the Rust system
(`conform-rust`): `93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported`. `conform-totals`:
`the interpreter and the Rust system ran 93 scenario(s) each`, of suite
`sha256:4cd1257a06a838a1bb2779b0309c347b1b6262703ff814da340988c32ecb7a3a`. Both reports say
`conformance_status: inconclusive`, with `execution_status: passed`: the suite carries no
coverage inventory (`coverage.knowledge: unknown`), and ess 0.96.0 never qualifies a run over
unknown coverage. The gate reads the counts. The planted defect, a `Navigation` storage that
drops every `put`, fails 33 of the 93. The crate's tests: `6 test suites: 27 passed`.
`generated-check`: `generated/rust matches spec/ (30 artifact(s) and .ess-output)`. `cli`:
`cli/surf-agent.yaml — valid CLI presentation binding`. `lock-check`: `Cargo.lock: 22 package(s),
none from outside this repository`. The root workspace: `118 test suites: 486 passed, 0 failed,
0 ignored`. In it, `surf-url` prints `urltestdata: 904 cases, 872 run, 872 passed, 32 skipped`;
the 32 are hosts outside ASCII, listed in `crates/url/tests/data/SKIPPED`. `view-stream-check`:
`view-stream-check: 19 event name(s), each a message of the AsyncAPI projection`. `view-test`:
`24 passed (14.6s)`; 12.9 s of it are the case that waits to see that a refused token is not
sent again. `html-conformance`, the last step of `check`: `html-conformance: 9107 cases, 9095
run, 7121 passed` (75 encoding cases and 7,046 tokenizer cases), with `tag markup
1979/1979/1979`, `tag text 328/328/328`, `tag charref 4739/4739/4739` and `tag encoding
75/82/82`. Before wave 7 these were 21 packages, 109 suites, 453 passed, 4442 passed cases
and `tag charref 2060/4739/4739`, and `check` had no `view-test`; the scenarios, the refusals,
the suite's digest and the 30 artifacts did not change. Before wave 6 these were 9 authored
scenarios, 83 scenarios, 27 refusals and the suite `sha256:4d9ad0c3…`, 102 suites, 407 passed,
4164 passed cases, `tag text 57/328/328` and `tag charref 2053/4739/4739`, and `check` had no
`view-stream-check`; the 21 packages and the 30 artifacts did not change. Before wave 5 they
were 97 suites, 363 passed, and 5 passed cases. Before
wave 4 they were 7 authored scenarios, 81 scenarios, 31 failed by the planted defect, 20
packages, 88 suites and 324 passed; the refusals (27, the same lines) and the 30 artifacts did
not change, and the HTML conformance gate was not part of `check`.

`REFUSALS_MAX` in `Taskfile.yml` is 28. `task suite` fails above it. Lower it when a refusal goes
away; raise it only with the new refusal listed here. The list is `grep '^refused:'
.out/synthesize.log` (read on 2026-11-21 after wave 6: 28 lines, the same as below; the gate
after wave 7 printed `28 refusal(s)`).
Twenty-two are `ESS-SYNTH-010`:

- 10 × `<binding>/binding/delivery`: every binding is `at_most_once`, which has no second
  delivery to observe.
- 10 × `<binding>/binding/on-failure`: every binding is `on_failure: drop`, which publishes
  nothing.
- 2 × `<binding>/binding/flow`, for `commit-on-response` and `fail-on-fetch-failed`: synthesis
  cannot arrange a navigation and a request that share one identity. The two page-load files
  under `spec/scenarios/` cover these flows.

Four are `ESS-SYNTH-001`: the outcomes `NoteNavigationStarted/noted`, `NoteNavigationEnded/noted`,
`NoteNavigationCommitted/noted` and `NoteNavigationFailed/noted` have "no three rows the filter
selects that the arranging commands can produce". `agent-session-reads-started-navigation.yaml`,
`agent-session-reads-committed-navigation.yaml` and `agent-session-shows-failure-reason.yaml`
under `spec/scenarios/` cover them.

One is `ESS-SYNTH-014`: the outcome `surf.agent.RecordAction/outcome/recorded`. The view
`SessionLog` declares an order and synthesis can put one row in it; an order over one row is a
claim no implementation can fail. `agent-session-log-in-sequence-order.yaml` covers it.

One more is `ESS-SYNTH-014`: the outcome `surf.document.AddElement/added`. The view `Elements`
declares an order, and the guards of `AddElement` leave synthesis one element.
`parsed-page-with-title-links-and-form.yaml` covers it. Without the two guards (an unknown
document, one already parsed) the suite has 91 scenarios and 27 refusals.

One note is printed with them: the specification declares actors and no component is served over
a network, so no `<command>/grant/denied` scenario is generated.
