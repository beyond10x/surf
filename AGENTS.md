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
    timing. The crate is ring 0.17.14 (the operator, 2026-11-22: "A: ring 0.17.14"), used by
    `surf-tls` alone and allowed by `taskfiles/lock-allow.txt`. The TLS protocol code on top of
    it is written here.
  - Build and test tooling (`ess`, `task`, test suites) is not runtime and is not covered.
  - `conformance/` is the one place in the repository where a third-party crate is allowed: the
    `ess-conformance` runner and what it brings, in a Cargo workspace of its own. The root
    `Cargo.lock` proves the rest: every package in it from outside this repository is one of
    the 19 packages of ring on `taskfiles/lock-allow.txt`, matched on name, version, source and
    checksum, and no crate of this repository but `surf-tls` depends on one (`task
    lock-check`).

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
- `spec/components.yaml`: the six components and the fifteen bindings: the four of the page load
  and `navigate-on-form-submitted`, and the ten of the agent's session (the six of before and,
  since round 1, `set-value-on-fill-requested`, `check-on-pick-requested`,
  `select-on-pick-requested` and `submit-on-press-requested`). The `agent` component owns `surf.agent`; `net` owns `surf.net`
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
  Since round 1 it also has the commands `Follow`, `Fill`, `Pick` and `Press`, the events
  `FillRequested`, `PickRequested` and `PressRequested`, the types `surf.agent.PageForm` and
  `surf.agent.LookCall` (the input of `look`), `form` and `from_line` on `OpenCall` and
  `GoCall`, and the event `surf.agent.EventsLeftOut` (no command publishes it; the session
  writes it).
- `cli/surf-agent.yaml`: the `ess-cli/2` document of the agent command line, checked by
  `task cli`; the client parser is held to it.
- `taskfiles/<name>.yml`: one file per task added since the agent interface plan, included by
  the root `Taskfile.yml` with `flatten: true` and `optional: true`. Optional because four
  tests copy the root `Taskfile.yml` alone into a temporary directory and run one task there;
  a missing include must not stop them. `check` still fails when an included task is missing.
  `taskfiles/cli.yml` holds the task `cli`, `taskfiles/site.yml` the task `site-build`,
  `taskfiles/html-conformance.yml` the task `html-conformance`,
  `taskfiles/tls-x509-conformance.yml` the task `tls-x509-conformance`,
  `taskfiles/view-stream-check.yml`
  the task `view-stream-check`, `taskfiles/view-test.yml` the task `view-test`, and
  `taskfiles/release.yml` the task `release-smoke`
  (`DIR=<unpacked archive>`, optional `TARGET`), which runs `taskfiles/release-smoke.sh`; the
  release workflow calls that script with `sh`. `taskfiles/check-runner.yml` holds the task
  `check-runner`, which runs `taskfiles/check-runner.sh`; `taskfiles/forge-watch.yml` the tasks
  `forge-watch` and `forge-watch-test` over `taskfiles/forge-watch.sh` and the recorded runs
  under `taskfiles/forge-watch-test/` (`runs-61fb7a9.json` and `runs-e47b1d1.json` as the forge
  listed them, four written by hand); `taskfiles/planning-only.yml` the task
  `planning-only-test` over `taskfiles/planning-only.sh`, the one list of planning and prose
  paths, which `.github/workflows/check.yml` calls; `taskfiles/test-layout.yml` the tasks
  `test-layout` and `test-layout-test` over `taskfiles/test-layout.sh`;
  `taskfiles/agent-tasks.yml` the tasks `agent-tasks-report`, `agent-tasks-archive` and
  `agent-tasks-model` (over `taskfiles/agent-tasks-model.sh`); `taskfiles/sh-echo.yml` the tasks
  `sh-echo` and `sh-echo-test` over `taskfiles/sh-echo.sh`;
  `taskfiles/tls-anchors.yml` the tasks `anchors-refresh` (fetches curl's `cacert.pem` and
  its `.sha256` with curl, or takes `FILE=<pem> SHA256=<file>`, refuses a sha256 that
  differs, writes `cacert.pem` and `SOURCE`; it reaches the network and is never a step of
  `check`) and `anchors-age` (`DATE`, default today; fails when the list is older than 90
  days) over `taskfiles/tls-anchors.sh`, POSIX sh; `taskfiles/tls-certs.yml` the tasks
  `tls-certs-check` (over `taskfiles/tls-certs-check.sh`, a step of `check`) and
  `tls-certs-slow` (a step of `slow`: makes the test certificates again into
  `.out/tls-certs/` and compares every file with the committed ones; `WRITE=1` writes them);
  `taskfiles/tls-flights.yml` the tasks
  `tls-flights-check` (over `taskfiles/tls-flights-check.sh`, a step of `check`) and
  `tls-flights-slow` (a step of `slow`: records the flights again and replays them; `WRITE=1`
  writes them). `taskfiles/slowest-tests.sh` is no task's own: `task test` calls it.
- Shell scripts, committed or written by a test: a value is printed with `printf '%s'`, never
  with `echo`. The forge's `sh` is dash, whose `echo` turns `\n`, `\\` and the other backslash
  sequences into control characters; this host's `/bin/sh` is bash, whose `echo` does not, so
  such a script passes here and fails there (0.4.0: the model lane wrote its wrapper's awk
  program with `echo` and counted no call on the forge). In the gate `task sh-echo` runs the
  tests that start a script with the `sh` of `PATH` under an `sh` whose `echo` does as dash's.
- `rust-toolchain.toml`: the one pinned rustc, `1.99.1`, with the components `rustfmt` and
  `clippy` (the gate uses neither; units run `cargo fmt -p` and `cargo clippy`), profile
  `minimal`. rustup reads it in every directory of the checkout, so `rustc --version` there
  prints that release; `.github/workflows/check.yml` and `release.yml` read the channel and the
  components from it and name no release of their own. A `RUSTUP_TOOLCHAIN` in the environment
  wins over the file; `task check-runner` unsets it.
- `spec/scenarios/*.yaml`: twenty-four authored scenario files (`ess-scenario/6`), for what synthesis
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
  parsing). `page-load-commits-and-parses.yaml` reads its document back as `Parsing`. Since
  wave 8 two more cover a request's method and body: `navigation-post-reaches-request.yaml` (a
  `Navigate` with `Post`, a body and its media type starts a request with the same method and
  media type) and `navigation-get-has-no-body.yaml` (a `Navigate` with `Get` and no body starts a
  request with `Get` and no media type); both read the request back from `Requests`, which
  stores no body. Every `Navigate` step of the other files names `method: Get`. Since
  `story:security-review-fixes`, `page-load-fails-at-a-forbidden-destination.yaml` sends
  `FailRequest` with `ForbiddenDestination` and reads the failed request and navigation back:
  synthesis sends only the first variant of a reason. Since round 1 nine more:
  `agent-follow-navigates.yaml` and `agent-follow-records-handle-and-label.yaml` for `Follow`,
  and the seven that cover what the form stories' refusals leave (below, "The gate's numbers"):
  `agent-fill-sets-value.yaml`, `agent-pick-unchecks-radio-group.yaml`,
  `agent-pick-selects-option.yaml`, `document-refuses-disabled-control.yaml`,
  `document-refuses-hidden-control.yaml`, `agent-press-submits-post.yaml` and
  `agent-press-submission-ends-without-body.yaml`.
- `spec/ess-inputs.yaml`: the list of every file above, and the pin `requires: ess 0.97.3`. A new
  file under `spec/` is not read until it is listed there.
- `generated/rust/`: the workspace `ess generate synthesize` writes, its own `Cargo.toml`
  included, with the record `.ess-output/state.json`. Eight crates under `crates/`: `surf-types`,
  `agent`, `shell`, `navigator`, `net`, `document`, `renderer`, `surf-system`. Its `agent` is
  the component's port; the hand-written command line is `crates/agent` (`surf-agent`).
- `Cargo.toml`: the root workspace. Its members are the globs `crates/*` and
  `generated/rust/crates/*`, so a crate joins by creating its directory. `Cargo.lock` is
  committed. `task build` and `task test` pass `--locked`, so a new crate fails them until
  `cargo generate-lockfile --offline` has run.
- `.config/nextest.toml`: the configuration of cargo-nextest, which `task test` runs: at least
  0.9.143, a run that does not stop at the first failure, a test ended after 60 s, and the
  JUnit report the task reads the ten slowest tests from.
- `.cargo/config.toml`: `debug = false` for the profiles `dev` and `test`, so no build of this
  repository carries debug info (the folder's rule: no debug info in test builds). A setting in
  `Cargo.toml` would not do: a cargo config file wins over the manifest, and one in a user's
  home may set `debug`. cargo reads it from the directory it runs in and its parents, so it
  holds for the root workspace, `conformance/` (run from the root with `--manifest-path`) and
  `generated/rust/` alike.
- Integration tests: each crate with a `tests/` directory, and `conformance/`, builds them into
  one test binary (`story:one-test-binary-per-crate`). Its `Cargo.toml` has `autotests = false`
  under `[package]` and one entry `[[test]]` with `name = "main"` and `path = "tests/main.rs"`;
  `tests/main.rs` holds a head comment, `#![forbid(unsafe_code)]` (not in `crates/fetch`,
  `crates/html` and `crates/tls`, whose counting allocators are `unsafe`; each stays in its
  module file, and `crates/tls/tests/main.rs` holds `#![deny(unsafe_code)]`) and one
  sorted `mod <file>;` line per file of `tests/`, `mod common;` included. A file reaches the
  shared helpers with `use crate::common;`. A new test file gets its `mod` line in
  `tests/main.rs`: cargo builds no file that is not declared there. A module file may also lie
  one directory down, `tests/<dir>/<stem>.rs`, declared in `tests/main.rs` by the line
  `#[path = "<dir>/<stem>.rs"]` and the line `mod <stem>;` (`common/` and `data/` are not such
  directories). A new crate starts with `autotests = false`, `[[test]] name = "main"` and
  `tests/main.rs`. A test is named `<file>::<name>` (`e2e::e2e_open_go_look_close`)
  in the binary `<package>::main`. One file's tests are selected with `--test main <file>::`;
  the filter matches a substring of the name, so where another module's name ends in `<file>`
  the selection adds `-- --skip <other>::` (`cargo test -p surf-cli --test main cli:: --
  --skip review1_cli::`). The selections of the repository are listed in
  `taskfiles/test-layout.sh` (`SELECTIONS`), and a stem added to the skipped list of a selection
  gets its `--skip <stem>::` in that selection. `task test-layout` holds this form.
- `crates/<name>/`: one hand-written crate each. One that uses a generated crate depends on it
  by path, `../../generated/rust/crates/<name>`. cargo 1.99.1 refuses a member glob that matches
  nothing (measured 2026-11-20), so `crates/` always holds at least one crate. Format one crate
  with `cargo fmt -p <package>`: `cargo fmt --all` rewrites `generated/rust/`. `cargo fmt -p
  surf` also reformats a test file that is not rustfmt-clean (`review1_generated_check.rs`),
  `cargo fmt -p surf-cli` two others (`review1_cli.rs`, `review1_output.rs`) and `cargo fmt -p
  surf-ports` one more (`review1_ports.rs`), and `cargo fmt -p surf-fetch` five (`fetch.rs`,
  `redirects.rs`, `review1_fetch.rs`, `review1_security_fetch.rs`, `review2_fetch_limits.rs`;
  `cargo fmt -p <package> -- --check` lists them, measured 2026-11-21); a unit that formats
  restores the files it does not own.
  - `crates/answer` (`surf-answer`): renders an agent answer in the grammar of
    `docs/agent-answer.md` and reads it back (`parse`, `unquote`); every value from a page, a
    server or an address goes through one quoting function. Since round 1
    (`story:page-text-answer-block`) it also renders and parses the `text` block of page text,
    every body line behind `>`, its characters checked by `escaped_in_text`
    (`escaped_at_run_start`, `quote_text`, `quote_text_cut`, `text_page`, `render_text_whole`;
    at most 200 body lines and 32,768 bytes, then a `more` line), and the notes `note text:
    no-css`, `no-document` and `deep <n>` (`TextNote`). No dependency.
  - `crates/secret` (`surf-secret`): the operating system random source, the comparison in
    constant time and the 32-byte token. Every secret and every random identity of the agent
    and the view comes from here. No dependency.
  - `crates/url` (`surf-url`): the WHATWG URL parser surf resolves addresses with, held to the
    web-platform-tests URL data vendored under `tests/data/`; no IDNA. No dependency.
  - `crates/fetch` (`surf-fetch`): fetches an `http:` address with the standard library only and
    returns status, media type and body, or `Unreachable`, `UnsupportedScheme`,
    `MalformedResponse`, `TooManyRedirects` or `ForbiddenDestination`. Its documentation lists what
    it does not do.
    Since wave 7 it follows up to 20 redirects (301, 302, 303, 307, 308 with a `Location`, for
    `GET`) inside one fetch, returns the address it ended at (`Response.url`, at most 64 KiB) and
    `TooManyRedirects` for a redirect after the twentieth; a redirect to an address that is not
    `http:` ends as `UnsupportedScheme`. Since wave 9 `surf_fetch::send(&Request, &Limits)`
    stands beside `fetch(address, limits)`, which stays the `Get` without a body: a `Request`
    holds the address, the method (`Method`: `Get`, `Post`), an optional body and an optional
    media type, borrowed from the caller. A `Post` sends `Content-Length` always and
    `Content-Type` when it has a media type. After a `Post` a 301, 302 or 303 continues as `Get`
    without the body, for the rest of the chain; a 307 or 308 repeats method, body and media
    type. A body over `Limits::max_request_body_bytes` (16,777,216 by default), a media type
    that is not `type/subtype` of tokens, and a `Get` with a body or a media type end
    `Unreachable` before the host is resolved. `Debug` of `Request` prints the body's length,
    not the body. The crate documentation's section "Memory" states what a fetch holds:
    16,917,505 bytes at the defaults while a response body is read; the request body is
    borrowed, not copied, and while it is written the fetch holds at most the header limit and
    one read buffer (73,728 bytes at the defaults for the whole fetch of a 4 MiB body sent twice
    through a 308, with short addresses and a 2-byte response); no bound is stated while a head
    is read. Since `story:security-review-fixes` it has the destination rule of its section
    "Destinations": each request of a chain resolves its host once, and those addresses are
    checked and dialed; after a first request to a public address a redirect to a loopback,
    private, link-local, unique-local, shared or unspecified address (or an IPv6 address that
    embeds one: IPv4-mapped, IPv4-compatible, NAT64 `64:ff9b::/96`, 6to4 `2002::/16`)
    ends `ForbiddenDestination` before any connection. `send_through(&Request, &Limits, &dyn
    Network)` takes the resolver and the transport (`Network::resolve`, `Network::connect`);
    `send` and `fetch` use `OsNetwork`. 117 tests: 14 unit tests, `adversary_redirect_memory` 4,
    `fetch` 24, `fetch_bounds` 7, `post` 16, `redirects` 27 (cases 21 to 26 and the review's
    case of other spellings over a `TestNetwork` of the test, no real name resolved), `review1_fetch` 5,
    `review1_security_fetch` 13 and
    `review2_fetch_limits` 7. `adversary_redirect_memory` holds an `unsafe`, a counting
    allocator, test only; since wave 10 `crates/html/tests/common/mod.rs` holds another, and
    since `story:tls-lock-allow` `crates/tls/tests/tls/alloc.rs` the third, each a counting
    allocator of tests; no file under a `src/` holds any. Depends on
    `surf-url`.
  - `crates/ports` (`surf-ports`): in-memory storage and version 4 identities for the generated
    components, one ports type per component. `AgentPorts<R = OsRandom>` stores `Session` and
    `Action`, assigns a version 4 `ActionId` from its random source (`with_source`,
    `with_caller_and_source`), meets the four obligations `NoteNavigationStarted`,
    `NoteNavigationCommitted`, `NoteNavigationFailed` and `NoteNavigationEnded`, and answers the
    caller's `session_id` from a `Caller` handle; the other ports types answer
    `try_generate_surf_agent_action_id` as unmet. Since wave 6 `DocumentPorts` stores
    `Document` and `Element` and draws element identities. Each table keeps an index from key
    to row (`Rows::index`), so a put or a read by key is one hash lookup; 10,001 links of one
    page are stored in under a second (`crates/session/tests/adversary_parse.rs`). Depends on
    the generated crates only.
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
    `tests/forget_delivered.rs`). No parsing was added then: a real page's document stayed
    `Parsing` until round 1.
    Since wave 7 `surf::Answer::Response` carries `url`, the address the fetch ended at, and
    `Browser::run` delivers it as the response's address; a redirect adds no event, since the
    transport is called once per `RequestStarted`.
    Since wave 8 the transport takes the started request,
    `Transport::fetch(&mut self, request: &RequestStarted)` (`surf::RequestStarted` and
    `surf::Method` are re-exported), and `surf::fetch_http` is the transport of `surf-cli` and
    `surf-session`: it sends a `Get` without body and media type through `surf-fetch`. Since
    wave 9 it sends a `Post` with its body and media type through `surf_fetch::send`, and still
    ends a `Get` with a body or a media type with `UnsupportedScheme` before any connection. No
    command sends a `Post` yet: `go` and `open` navigate with `Get`, and the form stories bring
    the first `Post`. `Browser::held` also counts the request bodies of
    `NavigationStarted`, `RequestStarted` and the `Fetch` invocation. `Browser::start` and
    `Browser::load` keep their signatures. `tests/request_method.rs` 5.
    The library also runs the `document` component's parser step, a parameter of `Browser` like
    the transport: it decodes and parses each committed HTML body with `surf-html`, computes the
    title and the actionable elements with `surf-page`, and issues one command per actionable
    element and then the ending command on the `document` port. `Browser` keeps each document's
    tree, read only through `Browser::document_tree`; no component's port receives it
    (`story:html-document-port`). The parser step is the type parameter `P` of `Browser`
    (default `HtmlParser`, replaced with `Browser::with_parser`); `Browser::element_nodes` gives
    each element row's identity with its node; `surf::parse_page` is the default's work.
    `tests/parse.rs` 9.
    Depends on the generated crates, `surf-ports`, `surf-fetch`, `surf-html` and `surf-page`.
  - `crates/surf-cli` (`surf-cli`): the binary `surf` and its five tests, moved out of
    `crates/surf`, and since wave 7 `redirects` 2 (the `navigation` line holds the address the
    fetch ended at, the `request` line the address asked for). It loads one `http:` address,
    with `surf-fetch` as the transport (since wave 8 through `surf::fetch_http`). A package of
    its own so that it can depend on crates that depend on the `surf` library. Depends on `surf`,
    `surf-fetch` and `surf-ports`. The release workflow builds and versions `surf-cli`
    (`crates/surf-cli/Cargo.toml`).
  - `crates/agent` (`surf-agent`): the agent command line and the binary `surf-agent`. A parser
    held to `cli/surf-agent.yaml` by a test, the token read from `SURF_TOKEN` or from a token file
    the caller owns, and the wire to `surfd` over a Unix socket in `$XDG_RUNTIME_DIR/surf`.
    `--as` and `--from-line` go to the session as `arg form` and `arg line` (a wire argument name
    is `[a-z]+`); the client and `surfd` pass UTF-8 above U+007F only in a `>` body line of a
    `text` block, and only characters `surf_answer::escaped_in_text` does not name.
    `wire::caller_uid()` is the one definition of the caller's user id (Linux only). Since
    `story:security-review-fixes` every command checks, before it connects and again after it
    started `surfd` (`wire::check_socket_path`), that the runtime directory and `surf/` are
    directories of the caller, not symbolic links, with no access for group and others, and
    that `surfd.sock` is a socket of the caller; a failed check exits 3 naming the path, with
    nothing connected or written. It checks no peer: `UnixStream::peer_cred` is unstable in
    rustc 1.99.1. The runtime directory is checked in one plain spelling (no trailing `/`,
    doubled `/` or `.` component), so a symbolic link is refused however it is written; a `..`
    component and a relative path are refused. `tests/start.rs` has the six tests of the checks,
    each against a listener of the test. Since wave
    4 `surf-agent open`, `go`, `look` and `close` run against a real `surfd` and a real session
    process; the package carries the release version, `0.4.0`, as do `surf-cli`, `surfd` and
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
    file and, only with `surfd --print-view-url`, in the line the binary writes to standard
    error (`Config::view_line`), in no answer, refusal, entry or log line. Without the option the
    binary writes `surfd: the live view's address is in <path of view-url>` there
    (`story:security-review-fixes`). A link answer whose record state is `unknown`
    (`link::SessionState::Unknown`, a refusal the session did not act on) changes nothing on the
    list. A reader of the view is not activity for the idle exit,
    and a replay of more than 256 numbers is answered with `reset`. It makes no peer check
    (`UnixStream::peer_cred` is unstable in rustc 1.99.1): the directory's mode and owner and
    the token carry it. A symbolic link under its directory makes it refuse to start. Bounds:
    64 connections, a request of 64 KiB within 10 s, an answer of 4 MiB, the entries of one answer in a counted block of at most 8 MiB (`link::ENTRIES_MAX`),
    one call per session at a time within 60 s, a late answer read for at most 600 s
    (`LATE_ANSWER_TIMEOUT`) while the session's turn stays taken, a log of at most 10,000
    entries and 16 MiB per session and 64 MiB over all sessions, a reader queue of 1,024
    entries, 256 ended sessions listed; open sessions are not bounded. It starts `surf-session`
    from the directory of its own executable. Its 49 tests (3 unit tests of `src/view.rs`,
    `adversary_link` 3, `built` 14, `in_process` 14, `log` 6, `view` 9) run a stand-in session
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
    head line and the refusal only, and since `story:security-review-fixes` that head line says
    `unknown` and the status record holds the state `unknown` with no address and no title, for
    an unreadable request too. A request is at most 64 KiB and must arrive within 10 s;
    one connection is served at a time. After each answer the session calls
    `Browser::forget_delivered` and holds no response body. Since wave 5 each answer to `surfd`
    carries the entries of its request: each action, the events the system published for it,
    and the answer without its token line (`src/entries.rs`); since wave 6 the entries of the
    three document events too. Since wave 7 a `go` or an `open` follows redirects: the `page`
    line holds the address the fetch ended at and the `did go` line the address asked for; a
    redirect adds no event and no entry. A session whose page address does not fit the status
    record (a server can cause that with a long redirect target) is listed by `surfd` without
    its address; the crate documentation writes out the bounds with a quoted address of at
    most four times its length plus two bytes. Since wave 8 the entries of
    `NavigationStarted` and `RequestStarted` carry `method`, `body` as `<n> bytes` and
    `body_type`, an absent one as the field with no value; its transport is `surf::fetch_http`,
    which since wave 9 sends a `Post` (no command of the session sends one yet). 73 tests:
    `adversary_document_text` 7, `adversary_events` 1, `adversary_parse` 4, `adversary_view`
    1, `built` 11, `core` 22, `e2e` 2 (runs the three built binaries, or those in
    `SURF_BIN_DIR`), `events` 17, `memory` 1, `redirect` 2 and `view` 5 (the live view of
    `surfd` over the real session).
    `open <url>`, `go <url>` and `look` take `--as tree|markdown [--from-line <n>]` and print the
    page as an outline or as Markdown in a `text` block, computed at each call from the tree
    `surf::Browser::document_tree` keeps for the document of the session's current navigation
    (nothing fetched again); with no HTML document `note text: no-document` and no block; the
    call records the same action, events and path as without `--as`.
    `cargo clippy -p surf-session` needs `--no-deps`: the generated `surf-types` fails clippy.
    Depends on the `surf` library, `surf-ports`, `surf-fetch`, `surf-answer`,
    `surf-page-text`, `surf-secret`, the
    generated `surf-types`, and on `surf-agent` and `surfd` (the packages of the other two
    binaries, for the end-to-end test and the wire).
  - `crates/view` (`surf-view`): the live view's HTTP server, standard library only: `GET` only, a
    fixed route table, the `Host` and view-token checks, and server-sent events with a bound per
    reader. `docs/view-stream.md` is what it answers. `web/` holds the page's ten files, with
    `detail.js` since wave 8 (no
    framework, no build step, nothing from another origin, no inline script or style), compiled
    in by `page_files()` in `src/assets.rs`; the page fits 360 px to 1,400 px. The example
    `prototype` serves them over three scripted sessions; it shows example data only, and since
    wave 7 publishes the four events of a real `open` and the nine of a real `go` (since wave 8
    `NavigationStarted` and `RequestStarted` with `method` `Get` and `body` and `body_type`
    `null`, as the example of `docs/view-stream.md` has them). Since wave 6
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
      stops the stream, asks nothing again and reads `offline`. Since wave 8 the right column
      (`detail.js`) shows the action at the playhead or, after a click on a mark, that event
      with its fields, and the session's requests, one row per request; the selected event is
      kept in the address (`#session=<id>&at=<sequence>&event=<number>`); the requests list
      and the selection scroll inside themselves. A mark holds a button for the keyboard, and a
      click anywhere on the mark selects its event. A value from a session is set
      only as `textContent` or as the value of an attribute of the list in `dom.js`, and
      `dom.js` is the only module that sets an attribute (it throws for any other name); its
      nine attribute names are unchanged. The static attributes of `index.html` (`tabindex`,
      `role`, `aria-labelledby` on the two scrolling regions) are not set by a script.
  - `crates/view-fixture` (`surf-view-fixture`): test tooling, not shipped: a binary that
    starts a page server, `surfd` in process and sessions for the page tests, under a
    directory of its own in `$XDG_RUNTIME_DIR`, driven by tab-separated commands on its
    standard input (`open`, `agent`, `kill`, `stop-surfd`, `cut`, `mend`). `kill` acts only
    for a session the fixture opened and only on a `surf-session` whose environment holds the
    fixture's own `XDG_RUNTIME_DIR`. Whether a process has ended is decided by `surf-test-process`
    (`alive`, in the binary and in `tests/fixture.rs`, as in the tests of `surf-agent`, `surfd`
    and `surf-session`): it counts the `/proc/<pid>/stat` states `Z`, `X` and `x` as ended: a
    killed session reads `X` while `surfd` reaps it, before it leaves `/proc`. A second binary,
    `surf-view-foreign`, waits until it is killed: `tests/fixture.rs` starts it under the name
    `surf-session` as a process that is not the fixture's and that `kill` must leave alive. A
    test starts no program of the system under another name: on the forge's runner `tail`
    started as `surf-session` exited at once. 2 tests (`fixture` 1, `adversary_commands` 1).
    Depends on `surfd`, `surf-view` and `surf-test-process`.
  - `crates/test-process` (`surf-test-process`): test tooling, not shipped and not published: no
    binary that ships depends on it. Whether another process has ended, decided once from
    `/proc/<pid>/stat`: `runs(stat)` for the text of a stat file, `alive(pid)` from one read of
    it, and `ended_within(pid, bound)`, which waits at most `bound` and returns the time it
    took, or an error that holds the last stat text read. `alive` and `runs` are no longer
    copied into the test helpers. The example `sleeper` is the child its tests start. Standard
    library only. 8 tests in one binary (`tests/main.rs` over `tests/lines.rs` and
    `tests/children.rs`). A development dependency of `surf-agent`, `surfd` and
    `surf-session`; a dependency of `surf-view-fixture`.
  - `crates/html` (`surf-html`): the HTML parser. Since wave 5 it decides a document's encoding
    by the sniffing algorithm and decodes UTF-8, UTF-16LE, UTF-16BE, windows-1252 and
    replacement (`src/encoding.rs`; the other 35 encodings of the label table are decoded as
    windows-1252), holds the input stream (`src/input.rs`), and tokenizes the data state and the
    states of tags, attributes, comments, DOCTYPE, processing instructions and CDATA sections.
    Since wave 6 the tokenizer also runs the RCDATA, RAWTEXT, script data and PLAINTEXT states
    and the less-than sign, end tag and script data escape states they lead to, started in any
    `State`; `Tokenizer::new` takes the last start tag. Since wave 8 the tokenizer gives one
    token at a time and the tree builder sets its state between tokens; one tokenizer serves a
    whole parse. Since wave 7 the tokenizer reads
    named and numeric character references in data, in RCDATA and in attribute values (the
    eight states 13.2.5.77 to 13.2.5.84 of the standard as fetched on 2026-11-21):
    `src/charref.rs`, and the table `src/charref_table.rs` (2231 names, the BSD 3-Clause notice
    in its header), written by the binary `entities-table` of `surf-html-conformance` from the
    vendored `entities.json`, never edited by hand, checked by that crate's
    `tests/entities_table.rs`. Since wave 8 `parse_document` and `parse_document_bytes` build a
    document with the insertion modes outside tables, templates and foreign content
    (`src/tree_builder.rs`, `src/tree_builder/modes.rs`): the stack of open elements and its
    scopes, implied end tags, the list of active formatting elements and its reconstruction,
    and the quirks decision. A token whose rule a later story owns goes to in body's "any other
    start tag" or "any other end tag". A `meta` met while the confidence is tentative re-parses
    the bytes once with the new encoding. Since wave 9 the tree builder runs the adoption agency
    algorithm (`src/tree_builder/formatting.rs`, with the step numbers of the snapshot the
    design note names) for the end tags of the fourteen formatting elements and for the `a` and
    `nobr` start tags, and Noah's Ark when a formatting element enters the list of active
    formatting elements. Since round 1 the tree builder runs every insertion mode and the
    rules for foreign content, `parse_fragment` runs the fragment parsing algorithm, and
    `NOT_YET_PARSED` is empty. `Tree::parent` and
    `Tree::descendants` read the tree without recursion. Parse errors of tree construction are
    not collected. The section "State" of `src/lib.rs` says it module by module. Since wave 10
    a parse is bounded (`src/limits.rs`, `ParseOptions::limits`, `story:html-parser-limits`):
    `limits::Limits` has the public fields and defaults `max_input_bytes` 16,777,216 (16 MiB,
    the body limit of `surf-fetch`; a longer input is cut at a character boundary, bytes before
    decoding), `max_depth` 512 (elements on the stack of open elements), `max_nodes` 1,048,576
    (every node of the arena, template contents and detached nodes included),
    `max_attributes` 1,024 (on one element), `max_stored_bytes` 67,108,864 (64 MiB: the UTF-8
    bytes of every string the tree holds, clones included, and `limits::ATTRIBUTE_BYTES`, 80,
    per attribute) and `max_steps` 134,217,728 (2^27: the entries tree construction visits in
    its scans of the stack of open elements, the list of active formatting elements and a
    node's children, checked after each token). On a limit the parse stops, the tree holds what
    was built, and `Tree::stopped_by` names it (`limits::Limit`: `InputBytes`, `Depth`, `Nodes`,
    `Attributes`, `StoredBytes`, `Steps`, the names of `surf.document.ParseLimit`); `InputBytes`
    is named only when no other limit stopped the prefix. `Tree::node_count`,
    `Tree::stored_bytes` and `Tree::steps` read the counts. A tree's heap is at most 2 ×
    `stored_bytes()` + 3 × `limits::NODE_BYTES` (136) × `node_count()`. The section "Limits" of
    `src/lib.rs` says where each limit stops. Still unbounded, as the unit reported: the scan of
    one token before the step check, end tags in the public `tokenizer::tokenize`, and the
    decoded text of `parse_document_bytes` (up to 3 bytes per input byte). Since
    `story:html-document-port` (round 1) `surf` depends on `surf-html` and parses every
    committed HTML body with it. 123 tests: 18 unit
    tests, 13 in `tests/encoding.rs`, 1 in `tests/not_yet_parsed.rs`, 8 in `tests/tree_body.rs`,
    8 in `tests/tree_formatting.rs`, 11 in `tests/tree_costs.rs`, 16 in
    `tests/adversary_tree.rs`, 37 in `tests/limits.rs` and 11 in
    `tests/adversary_limits.rs`. The last two share `tests/common/mod.rs`, a counting allocator
    per thread, and assert the limit that stopped a parse, its steps and the bytes it allocated,
    no wall time. A test of a limit sets a small limit, the default values are asserted as
    numbers, and no test builds input of a default size (`story:limit-tests-at-small-limits`).
    One exception: the depth-limit test of the tree builder stays at the default limit,
    `element_at_depth_513_stops_at_the_depth_limit` in `tests/limits.rs`, whose comment says
    that the depth limit is cheap at its default and reached through both entry points.
    The test files are modules of the one test binary, whose global allocator is the one of
    `tests/common/mod.rs` (`story:one-test-binary-per-crate`).
  - `crates/page` (`surf-page`): since wave 9, the title and the actionable elements of a
    `surf-html` tree (`title`, `actionable_elements`), with their kind, target or value and the
    flags `hidden`, `disabled`, `checked` and `required`. `is_hidden` is the rule of
    `docs/design/html-parser.md` for "hidden": `aria-hidden` true in any ASCII case, the hidden
    elements of the HTML Standard's style sheet, a closed `dialog`, the content of a closed
    `details` but for its first `summary`. Since wave 10 each row has `label`, its accessible
    name (`src/label.rs`, `story:html-accessible-name`: Accessible Name and Description
    Computation 1.2, 4.3.2, with HTML-AAM 4.1, over the tree, without CSS), and `label_cut`.
    `LABEL_BYTES_MAX` is 1,024: a longer name is cut at the last UTF-8 character boundary at or
    below 1,024 bytes, without a space the cut would leave at its end. `LABEL_WORK_MAX` is
    16,777,216: the work for all labels of one call of `actionable_elements`, one unit per node
    read and one per byte read into a name; once it is spent, each later name comes from the
    element's own attributes only. `label_cut` says that either happened. A hidden element is
    named from its own subtree, hidden parts included (a deviation of the crate documentation).
    The one walk records what the labels need (a dense index of the nodes with their hidden
    flags, the first element per ID, each `label` with its control, and where the eleven
    attributes a name reads sit), so a name calls `is_hidden` for no node and searches no
    attributes. `surf_page::attribute_visits()`, hidden from the documentation, counts the
    attributes the crate's functions compared on this thread while searching an element's
    attributes; `tests/label.rs` holds it at 16 per attribute of the page. Since round 1
    (`story:page-text-outline`) it has `handles`, the one numbering of `l<n>`, `b<n>`, `f<n>`,
    and `hidden_roots`, the rule of `is_hidden` for a whole tree in one walk; that rule hides an
    element with a `popover` attribute but an open `dialog` (no script opens a popover), and
    the copy of the selected `option` the parser puts into a `selectedcontent`, from which
    `actionable_elements` lists no row (surf's reading, not the standard's rendering). Since
    `story:html-document-port` (round 1) `surf` depends on it, and `surf-page-text` too.
    65 tests: 28 in `tests/elements.rs`, 34 in `tests/label.rs`, 1 each in
    `tests/tree_tables.rs` and `tests/tree_template.rs` (the parser unit's) and 1 in
    `tests/adversary_select.rs` (the parser adversary's), modules of the one test binary
    (`story:one-test-binary-per-crate`). Standard library and `surf-html` only.
  - `crates/page-text` (`surf-page-text`): a `surf-html` tree as text, the outline
    (`outline`) and Markdown (`markdown::markdown`, tables as GitHub tables since
    `story:page-text-tables`), each line without the answer's `> `, one at a time.
    `Markdown::deep` gives the `n` of the answer's `note text: deep <n>` before the first
    line. A line has at most 1,024 bytes but a table row; its prefix at most 128
    (`PREFIX_BYTES_MAX`); an atom (a link with its handle, an image, a code span, a control) is
    never broken across lines. `node_visits()` and `table_visits()`, hidden from the
    documentation, count the nodes its walks and the rows and cells its table code visited on
    this thread. 57 tests in one test binary (`tests/main.rs` over `tests/cases/outline.rs`
    11, `tests/cases/markdown.rs` 19, `tests/cases/tables.rs` 13, `tests/cases/select.rs` 3
    and `tests/cases/adversary_text.rs` 11, and the pages of `docs/design/page-as-text.md` in
    `tests/common/mod.rs`). Depends on `surf-html`, `surf-page` and `surf-answer`.
  - `crates/html-conformance` (`surf-html-conformance`): the vendored html5lib-tests (commit
    `c5495c5`, 19 files, 7,134 cases, MIT) and web-platform-tests parser data (commit `13f8051`,
    62 files, 1,973 cases, 3-Clause BSD) under `data/`, each source with a `SOURCE` file of
    checksums, and a harness that prints the cases that exist, run and pass per file, per tag and
    in total. `cargo run -p surf-html-conformance` prints
    `html-conformance: 9107 cases, 9095 run, 9095 passed` after round 1 (`8428 passed` after
    wave 9); the gate holds the counts of `expected/`, which the parser unit raised. Nobody edits a
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
  - `crates/tls-conformance` (`surf-tls-conformance`, test tooling, not shipped): runs the
    x509-limbo cases of `crates/tls/tests/data/x509-limbo/` through `surf_tls::path::PathVerifier`
    at each case's `validation_time` or at `LIMBO_TIME` (the limbo commit date), never at the
    clock. `expected/limbo.txt` holds the counts (`x509-limbo: 5143 cases, 5089 run, 5060
    passed`, per group, per RFC 5280 section) and one `open <id> <reason>` line per case that
    runs and does not pass; `expected/limbo-kinds.txt` the kind each FAILURE case must end
    with. `surf-tls-conformance --check --base <revision>` holds a run to them with the rules
    of `surf-html-conformance --check`; the task `tls-x509-conformance` runs it (`TLS_BASE`,
    default `HTML_BASE`) and is part of `task check`. A case not run is printed with its
    reason (`client`, `crl`, `pedantic`, `policy`, `no-peer-name`). 16 tests (`check`).
  - `crates/tls` (`surf-tls`): TLS 1.3 for `https:`, being built (`epic:tls-https`, design
    note `docs/design/tls.md`). Its one dependency is ring 0.17.14, the one third-party crate in
    what ships, and `src/crypto.rs`, the seam, is the one file that names it: AEAD seal and
    open for AES-128-GCM, AES-256-GCM and ChaCha20-Poly1305, SHA-256 and SHA-384 one-shot and
    running, HMAC, HKDF extract and expand, X25519 and P-256 agreement from a `Random` source,
    the signature checks of nine schemes behind the trait `SignatureVerifier`, and `REFUSED`
    (every MD5 or SHA-1 signature, RSA keys under 2048 bits), named before any verification.
    `crypto.rs` names every ring item by its full path `ring::…` where it uses it: no `use`
    or `extern crate` of ring, no alias of a ring item. No public item of it holds `ring`:
    the signature of a `pub fn`, the whole of a `pub struct`, `pub enum`, `pub union`,
    `pub trait`, `pub type`, `pub const`, `pub static` or `pub use`, a `pub` field, the header
    of an `impl` block of a type that is not private to the file; `pub(crate)` counts as
    public. `Secret`, `Digest`, `RunningHash`, `AeadKey`, `EphemeralKey`, `SigningKey` and
    `Random` implement none of `PartialEq`, `PartialOrd` and `Hash`, which the test crate
    asserts at compile time: a secret or a tag is compared with `constant_time_eq` (ring's
    `hmac::verify`) or `hmac_verify`. `src/error.rs` holds the public `Error` of item 6 of the note. The feature
    `test-tooling`, off by default and never in a release, adds signing from PKCS#8, ring's
    fixed random sources (a fixed sequence dropped before every slice was drawn does not
    panic), `TEST_TOOLING_MARKER`, `Anchors::from_list`, the module `handshake` with its trace
    mode `handshake::trace::GivenClientHello`, and `Connection::set_write_sequence`; the
    crate's dev-dependency on itself turns it on for its tests. `.cargo/config.toml` compiles
    ring at `opt-level = 3` in the dev and test profiles (item 4 of the note).
    Since `story:tls-record` `src/record.rs`, `src/key_schedule.rs` and `src/transcript.rs`
    are the record layer over any `Read + Write` (keys given by the caller for each
    direction, sequence numbers that refuse to wrap, handshake messages reassembled up to
    65,535 bytes, the record rules of RFC 8446 section 5 each refused with its alert, at most
    23,726,566 records under one AES-GCM write key), the key schedule for SHA-256 and SHA-384
    and the transcript hash with `message_hash`, replaced once.
    Since `story:tls-x509` it reads certificates: `src/der.rs` (a DER reader over a slice,
    never past it and never allocating by a declared length; `DEPTH_MAX` 32,
    `LENGTH_BYTES_MAX` 4), `src/x509/` (`Certificate::from_der`, version 3 only,
    `SERIAL_BYTES_MAX` 20, `LeafKey`, and `TrustAnchor`, which a root the reader refuses still
    gives as name and key when it holds no nameConstraints), `src/pem.rs` and the private
    `src/base64.rs`, and `src/verify.rs` (the trait `ServerCertVerifier`,
    `CertificateRejection`, `Check` with the twelve checks of item 7); `Error::Certificate`.
    Since `story:tls-x509-path` `src/path.rs` and `src/names.rs`: `PathVerifier`, the one
    implementation of the trait `ServerCertVerifier` of `src/verify.rs`, `verify(certificates,
    host, now) -> Result<LeafKey, CertificateRejection>`, the certificates as the server sent
    them, leaf first; path building within the bounds of item 9 of the note
    (`INTERMEDIATES_MAX` 6, `SIGNATURE_CHECKS_MAX` 100, `CANDIDATES_MAX` 25,000,
    `NAME_CONSTRAINT_COMPARISONS_MAX` 250,000); `add_root` makes an anchor of a root
    certificate and keeps the certificate, so its own extensions are checked as an issuer's;
    `names::host` is the reference identifier of a host. When no path is found and
    certificates after the leaf were left out as unreadable, the refusal is `Unreadable`
    naming the first of them and how many were left out (`left_out`).
    Since `story:tls-anchors` it carries the root list of Mozilla as curl publishes it:
    `src/anchors/cacert.pem`, unchanged, and its record `src/anchors/SOURCE` (address, date of
    the list, sha256, count, licence MPL-2.0), both written only by `task anchors-refresh`.
    `Anchors::bundled()` reads it once per process; `with_extra_pem` adds an extra file of at
    most 32 KiB and 64 blocks, every one a CA certificate, or refuses it whole with an
    `AnchorError` naming the block. One root of the list of 2026-11-05, e-Szigno TLS Root CA
    2023 (P-521), gives no anchor: 117 anchors of 118 certificates.
    Since `story:tls-handshake` `src/handshake/` (the client's state machine; private, public
    under `test-tooling` for its trace mode) and `src/connection.rs`: `Connection` over any
    `Read + Write`, given a host, a `&dyn ServerCertVerifier` and the time, runs the whole
    handshake in `Connection::new` and calls the verifier once, for the server's Certificate
    message. `crates/tls/tests/data/flights/`: recorded flights of `openssl s_server` on
    127.0.0.1, pinned by `SOURCE` (sha256 per file) and `DIGEST` (sha256 of
    `crates/tls/examples/record-flights.rs`); `task tls-flights-check` in `check`, `task
    tls-flights-slow` in `slow` records them again and replays the new files (`WRITE=1`
    writes them).
    Tests: one binary, `tests/main.rs` declaring each module of `tests/tls/` with `#[path]`,
    among them `alloc` (a counting allocator, per thread, in the form of
    `crates/html/tests/common/mod.rs`, the third `unsafe` of the repository, test only; one
    case of `seam`, `adversary_the_seam_allocates_at_most_its_output`, reads it: AEAD seal and
    open of a 16 KiB record, a hash and an HKDF expand allocate nothing, an agreement at most
    64 bytes) and `seam` (one test per Wycheproof file of `tests/data/wycheproof/`, 14 files
    of commit `65ca494` with `LICENSE` and `SOURCE`; the 103 AES-192 vectors and the AES-GCM
    nonces other than 96 bits skipped by name; the hashes, signing under `test-tooling`, the
    refusals, the lengths and code points, the digests of the vendored files; and, read with
    comments and strings left out, that no file under `src/` but `crypto.rs` holds `ring` as
    an identifier); `record` (28: hostile records built in memory, one test per row of
    `tests/tls/rfc8446-record-musts.txt`, which `record::musts_rows_name_tests` holds to its
    tests), `rfc8448` (6: RFC 8448 sections 3, 5 and 7 replayed as the client and as the
    server from the RFC's text in `tests/data/rfc8448/`, with `SOURCE`) and `adversary_layer`
    (10); `der` (34), `pem` (13), `x509_parse` (30: the DER writer of the tests and the checks
    of item 7, `check_names` read against the table of item 7), `limbo_parse` (7: the vendored
    x509-limbo subset of `tests/data/x509-limbo/`, five files by case group, one test per
    file, each read once; `parse-counts.txt`) and `adversary_x509` (24); `anchors` (40 tests) and `adversary_anchors` (9: the bundle against
    `SOURCE`, the extra file and its refusals, the two tasks run with `sh` on local files under
    `CARGO_TARGET_TMPDIR`); `names` (11), `path` (71) and `adversary_path` (11); `handshake`
    (16, among them `handshake_through_the_path_verifier`, a whole handshake through
    `PathVerifier` over a test root of `crates/tls-test-certs`), `handshake_hostile` (73),
    `flights` (3) and `adversary_handshake` (8); `security_seams` (20: the security review of
    the seams, its 13 cases and the 7 of its fixes: the host normalised once and refused before
    the first byte, every anchor with its certificate held to item 9.4, refusal texts as cut
    excerpts, CertificateVerify bound to `ok-wildcard`'s own P-256 key, each client flight one
    write and one flush; and a compile-time
    check that the four secret-holding types have a `Drop`). Depends on `ring`; its tests on
    `surf-html-conformance` for its JSON reader.
  - `crates/tls-test-certs` (`surf-tls-test-certs`): test tooling, not shipped: the test
    certificates of `epic:tls-https` (`story:tls-test-certs`). A DER writer and a certificate
    builder that sign through the seam of `surf-tls` (feature `test-tooling`) under a fixed
    random source, so every certificate is reproducible byte for byte; the cases in
    `src/cases.rs`; the private keys in `keys/` (PKCS#8 DER, made once by `openssl genpkey`,
    `keys/SOURCE`, never regenerated: a certificate made from them is reproducible only while
    they stay); `openssl/make.sh`, which makes `sha1`, `md5` and `rsa-1024` (ring cannot sign
    them) with `openssl` 3.4 or later, with `openssl/SOURCE` (the version that made them and
    the sha256 of each file). The output is committed in `certs/`: per case `chain.pem` (the
    leaf first), `root.pem` (the one anchor of the case) and `leaf.pk8`, and `INDEX`, one
    line per case (`<case> <host> 2026-11-22T00:00:00Z <outcome>`, the outcome `ok`,
    `CertificateExpired`, `CertificateNameMismatch` or `CertificateNotTrusted <check>` with a
    check of item 7), 33 cases; and `DIGEST`, the sha256 of `src/`, `keys/`, `openssl/` and
    `Cargo.toml`. Tests read `certs/`; only `certs::reproducible` builds the cases, in memory,
    and compares them with `certs/`. A change to `src/`, `keys/`, `openssl/` or `Cargo.toml`
    fails `task tls-certs-check` until `task tls-certs-slow WRITE=1` has made `certs/`,
    `openssl/SOURCE` and `certs/DIGEST` again. 18 tests (`adversary` 4, `certs` 6, `check`
    8). Depends on `surf-tls`.
- `view-tests/`: the Playwright tests of the live view's page (`@playwright/test` 1.65.0,
  Chromium's headless shell, a committed `package-lock.json`), each case with its own
  `surf-view-fixture`: 38 cases, 18 in `tests/first-page.spec.mjs`, 6 in
  `tests/adversary-page.spec.mjs`, 6 in `tests/detail.spec.mjs` and 8 in
  `tests/adversary-detail.spec.mjs`. Node and Playwright are gate tooling only.
- `conformance/` (`surf-conformance`): runs `.out/suite.json` against the Rust system in process,
  with the runner of `ess-conformance`. A Cargo workspace of its own, outside the root one, with
  its own committed `Cargo.lock` and its own `target/`. It depends on `surf`, `surf-ports` and the
  generated crates by path, and on `ess-conformance`, `ess-primitives` and `ess-domain` from
  `https://github.com/beyond10x/ess` at tag `0.97.3`; its lock pins the third-party packages
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
    carries. Since wave 8 `Navigate` and `Fetch` read their inputs through `present()`, so an
    optional input sent as `null` reads as one left out.
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
| `agent` | `surf.agent` | `Session`, `Action` | `OpenSession`, `Go`, `Follow`, `Fill`, `Pick`, `Press`, `CloseSession`, `NoteNavigationStarted`, `NoteNavigationCommitted`, `NoteNavigationFailed`, `NoteNavigationEnded`, `RecordAction` |
| `shell` | `surf.shell` | `Window`, `Tab` | `OpenWindow`, `OpenTab`, `CloseTab` |
| `navigator` | `surf.navigation` | `Navigation` | `Navigate`, `CommitNavigation`, `FailNavigation`, `CancelNavigation` |
| `net` | `surf.net`, `surf.url` | `Request` | `Fetch`, `DeliverResponse`, `FailRequest` |
| `document` | `surf.document` | `Document`, `Element`, `Form`, `Option` (code name `ListOption`), `Submission` | `ParseDocument`, `AddElement`, `FinishParsing`, `AddForm`, `AddOption`, `SetValue`, `Check`, `SelectOption`, `StartSubmission`, `SubmitForm`, `AbandonSubmission` |
| `renderer` | `surf.render` | `Frame` | `PaintFrame` |

Since round 1 the agent also publishes `FillRequested`, `PickRequested` and `PressRequested`,
and the document has the views `Controls`, `Forms`, `Options` and `Submissions`.

The page load: `Navigate` publishes `NavigationStarted`, which invokes `Fetch`. `DeliverResponse`
publishes `ResponseReceived`, which invokes `CommitNavigation`. `NavigationCommitted` invokes
`ParseDocument` when its media type is `text/html`; the document is `Parsing` and
`ParsingStarted` carries the body. The parser step of `crates/surf`
(`story:html-document-port`) issues `AddElement` once per actionable element in tree order,
then `FinishParsing`, which sets the title and moves the document to `Parsed`. `surf-session`
and `surf` parse every committed HTML body; `conformance/src/target.rs` gives every scenario
browser the parser step `Unparsed`, which never answers. `FailRequest` publishes `FetchFailed`,
which invokes `FailNavigation`. Nothing invokes `PaintFrame` yet. A fifth reaction,
`navigate-on-form-submitted`: `FormSubmitted` invokes `Navigate`.

A response names the address it came from, after the last redirect (`DeliverResponse.url`,
`ResponseReceived.url`): the navigation commits there, the document is parsed at it, and the
session shows it. The request keeps the address asked for. `surf.net.FailureReason` has the
variant `TooManyRedirects`: a redirect that arrives after 20 followed ones, the wording of the
Fetch Standard; and `ForbiddenDestination`: a redirect from a public address to a local one
(the comment above `entities` in `spec/domains/net.yaml`, surf's own rule). Since wave 7 the
transport (`crates/fetch`) follows redirects inside one
fetch and returns the address it ended at; a redirect adds no event.

A request has a method (wave 8). `Navigate` and `Fetch` carry a method (`surf.net.Method`:
`Get`, `Post`), an optional body and its media type, and `NavigationStarted` and
`RequestStarted` carry the three on; `Requests` shows the method and the media type, and no row
stores a body. The agent's `Go` navigates with `Get`. Since wave 9 the transports of `surf`
and `surf-session` send a `Post` with its body and media type through `surf_fetch::send`, and
end a `Get` with a body or a media type with `UnsupportedScheme`, before connecting. No command
sends a `Post` yet; the form stories bring it. The marker of `spec/domains/net.yaml` is
`UNMAPPED: request headers, cookies, caching and TLS.`

The agent's session: `Go` publishes `NavigationRequested`, which invokes `Navigate`.
`NavigationStarted` also invokes `NoteNavigationStarted`; `NavigationCommitted` invokes
`NoteNavigationCommitted` (status and media type), `NavigationFailed` invokes
`NoteNavigationFailed` (the reason) and `NavigationCancelled` invokes `NoteNavigationEnded` with
the state `Cancelled`. `SessionClosed` invokes `CloseTab`. Since round 1 the agent's four new
bindings: `set-value-on-fill-requested` (`FillRequested` invokes `SetValue`),
`check-on-pick-requested` (`PickRequested` invokes `Check`), `select-on-pick-requested`
(`PickRequested` invokes `SelectOption`) and `submit-on-press-requested` (`PressRequested`
invokes `StartSubmission`). The `Sessions` view shows each
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
Since wave 8 a request has a method and a body in the specification, `surf-html` builds a
document outside tables, templates and foreign content (run by the conformance harness only),
and the live view's page has a detail panel. Since wave 9 `surf-fetch` sends a `POST` (no
command sends one yet), the tree builder handles misnested formatting, and `surf-page` reads
the title and the actionable elements out of a tree (run by its tests only). Since wave 10 the
HTML parser stops at six limits and names the one it met, and `surf-page` gives each
actionable element its label (run by the tests only).

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
  `task check-runner` is green on it, never before. After the push, `task forge-watch
  COMMIT=<commit>` (with `GATES_POLICY` naming the policy file when `../gates-policy/` is not
  beside the checkout) waits for the forge's runs of that commit and exits non-zero when one
  did not pass.
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
- Present state: wave 10 and round 1 are delivered (10 of 22 waves, and the stories of round
  1); release `0.2.0` follows wave 4, release `0.3.0` wave 9 and release `0.4.0` round 1 (the
  releases are listed at https://github.com/beyond10x/surf/releases). The
  session `host-operator` ordered the continuation after the checkpoint on 2026-11-21, for the
  operator: "continue as its `AGENTS.md` says". The next is round 2 of
  `docs/plan/agent-waves.md` ("Rounds after wave 10": the waves 11 to 22 in three rounds). The stories
  accepted behind the planned waves are listed in
  that file, in "Accepted on 2026-11-21, behind the planned waves". Open for the next steps:
  links, forms and history (round 2), and TLS.
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

On a machine whose installed `ess` is newer than the pin of `spec/ess-inputs.yaml`, set
`ESS_TOOLCHAIN=0.97.3` for every task that runs `ess`: the tasks run it from the repository's
root, where the pin under `spec/` is not found, and `--strict-requires` then refuses the newer
release (`ess toolchain which` says which release runs and why; seen with ess 0.97.1 on
2026-11-21). The forge's workflow installs 0.97.3.

- `task check`: the gate. Validates (a warning fails it), synthesizes the suite and runs it
  against the interpreter built into `ess`, then against the Rust system (`conform-rust`),
  compares the two runs' suite and total (`conform-totals`), checks `generated/rust/` against
  `spec/` (`generated-check`), checks `cli/surf-agent.yaml` against `spec/` (`cli`), checks
  the root `Cargo.lock` for third-party packages (`lock-check`), then builds and
  tests the root workspace with warnings denied, checks the live view's event names
  (`view-stream-check`, after `test`), runs the page tests in a browser (`view-test`, after
  `view-stream-check`), and last holds the HTML conformance counts (`html-conformance`). Each
  step's failure fails it. Because of `view-test`, `task check` needs Node and Chromium's
  headless shell on the machine (below); the forge's workflow installs Node 22 and the shell
  before it. Since `story:local-gate-equals-forge` it ends with `forge-watch-test` and
  `planning-only-test`. Since `story:one-test-binary-per-crate` it runs `test-layout` after
  `lock-check` and `test-layout-test` after `planning-only-test`. Since `story:agent-tasks`
  `test` runs `task build` first and sets `SURF_BIN_DIR` to `target/debug` for its tests, and
  `check` runs `agent-tasks-report` after `test`. Since 0.4.0 it runs `sh-echo` after
  `agent-tasks-report` and ends with `sh-echo-test`. Since the TLS stories it runs
  `tls-certs-check` and `tls-flights-check` after `test-layout` and `tls-x509-conformance`
  after `html-conformance`.- `task check-runner`: the gate to run before a merge, `task check` as the forge's runner runs
  it (`taskfiles/check-runner.sh`; `slot task check-runner`). What it does differently from
  `task check`: it runs in `.out/runner/tree`, a clean export of `HEAD` (a clone sharing this
  repository's objects, brought to `HEAD` by `git checkout --force` and `git clean -ffdx -e
  target/`, so an unchanged file keeps its time and the export's own `target/` is reused), so
  that a file that exists only in the working tree, changed, untracked or ignored, cannot make
  it pass; it refuses, naming the files, when the working tree has uncommitted changes, unless
  `ALLOW_DIRTY=1` is given, which it prints; and it sets the runner's environment: `CI=true`,
  `GITHUB_ACTIONS=true`, `CARGO_TERM_COLOR=always`, `CARGO_INCREMENTAL=0`, `ESS_TOOLCHAIN`
  read from `requires:` of `spec/ess-inputs.yaml` (no need to set it by hand),
  `HTML_BASE=origin/main` (the export's `origin` is this repository, so that is the local
  `main`), and it unsets `RUSTUP_TOOLCHAIN`, `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR`,
  `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS` and `RUSTDOCFLAGS`. `view-tests/node_modules` is not
  committed: `task view-test` installs it in the export with `npm ci`, as on the forge. What
  stays the machine's: the cargo caches of the user, the Playwright browser cache, the cargo
  configuration in the user's home, a compiler cache it names (`rustc-wrapper`) included, and
  the operating system. cargo also reads the working tree's `.cargo/config.toml`, a parent
  directory of the export: a change to it is uncommitted, so only `ALLOW_DIRTY=1` lets it
  through. On the forge `task check` takes 167 s (release `0.3.0`); on a machine at a load
  average of 106 to 113 `task check-runner` ran 832 s (2026-11-22), past the build pool's
  limit of 10 minutes for a job, so there it names its time:
  `slot --timeout 25m task check-runner`. `.out/runner/tree` is a clone inside the working
  tree, and `worktree finish` refuses a managed tree that still holds it: remove `.out/runner`
  before the tree is finished.
- `task forge-watch COMMIT=<commit>`: `taskfiles/forge-watch.sh <commit>` waits until every
  workflow run of the commit is completed, reading through `b10x-gates ... api --method GET`
  with the policy file from `GATES_POLICY` (default `../gates-policy/policy.json`) and `jq`,
  prints one line per run (`<name>: <status> <conclusion> <id>`) and exits non-zero when a run
  did not conclude `success`, `skipped` or `neutral`, when no run of the workflow `check` was
  listed within 180 s, or when a run was not completed within an hour. The script's head names
  the variables that change those bounds. It reaches the forge; no step of `check` runs it.
- `task forge-watch-test`: the decision of that script on the recorded files under
  `taskfiles/forge-watch-test/`, without the network (`sh taskfiles/forge-watch.sh --runs
  <file>`: 0 passed, 1 red, 3 not completed, 4 no run of `check`). `61fb7a9`, whose `check`
  concluded `failure`, exits 1; `e47b1d1` exits 0.
- `task planning-only-test`: both sides of `taskfiles/planning-only.sh`, which reads changed
  paths on standard input and exits 0 when all are planning or prose files: `.engineering/`,
  `docs/plan/`, and `*.md` outside `website/`, the code directories (`crates/`,
  `conformance/`, `generated/`, `spec/`, `cli/`, `view-tests/`) and the two documents the gate
  reads (`docs/design/html-parser.md`, `docs/view-stream.md`). An empty list is not planning
  only. On a pull request whose files are all such, the workflow `check` skips every step after
  the listing and ends green; a push to `main` always runs the whole gate.
- `task test-layout`: `sh taskfiles/test-layout.sh .`, the form of the integration tests
  (Layout). It visits every directory that holds a `Cargo.toml` and a `tests/` directory
  (`target`, `node_modules`, `.out`, `.git` and directories whose name starts with a dot not
  entered; a `Cargo.toml` below a `tests/` directory is a fixture, not a crate). In a file of a
  test binary it reads each line without its string and char literals and its comments, so a
  `mod` or a `#![cfg` there counts for nothing. It fails, naming the path or the module: when
  `tests/main.rs` is missing; when the
  manifest lacks `autotests = false` in its `[package]` table, or exactly one `[[test]]` entry
  of exactly `name = "main"` and `path = "tests/main.rs"` (a table header is compared without
  its spaces and quotes, so `[[ test ]]` and `[["test"]]` count), or holds `test = true` in an
  `[[example]]` or `[[bench]]` entry; when a line of `tests/main.rs` is not blank, a comment, an
  inner attribute without `cfg`, a `use` line or a module declaration (`mod <name>;`, or a
  `#[path]` line and `mod <name>;`); when a test file, or one a directory down (`common/` and
  `data/` skipped), is not declared; when a file of the binary has `#![cfg` or `#![cfg_attr`
  anywhere in a line, or an inner attribute `#![` that does not close on its line (the files of
  the binary: those `tests/main.rs` declares and every child module they declare out of line,
  with or without `#[path]`, followed to its file; a child whose file it cannot read fails, and
  so does an out-of-line `mod <name>;` inside an inline module, indented or after a `{` on its
  line, which it cannot follow; today `crates/secret/tests/secret.rs` declares `src/compare.rs`
  and the `common/mod.rs` of `surf-agent`, `surfd` and `surf-session` declare
  `crates/agent/tests/common/base.rs`); and, in a crate of its `SELECTIONS` list, when a module
  of `tests/main.rs` other than the selected or a skipped one, or any module declared inside a
  file of the binary (inline or out of line, any visibility, any attribute before it on its line,
  the selected module's own file included), has a name that ends with the selected or a skipped
  name, so that the selection's filter or its `--skip` would match its tests. Not seen by the
  check: a module that a macro expands to, a file pulled in by `include!`, `autotests = false`
  inside a multi-line string of `[package]`, and a crate reached through a symbolic link. It
  calls no cargo and prints `test-layout: <d> test directories, <f> test files, one test binary
  each`.
- `task test-layout-test`: that script on a copy of the manifests, the `src/*.rs` and the test
  files under `.out/test-layout/`, thirty-two cases: the copy as it is and four right layouts
  pass (a comment that names `#![cfg(any())]`, `mod x;` inside a string literal and in a
  trailing comment, a fixture crate below `tests/data/`), and each of twenty-seven planted
  defects fails naming its path or module. `SCRIPT=<path>` runs a copy of the script.
- `task tls-certs-check`: `sh taskfiles/tls-certs-check.sh crates/tls-test-certs`, no build and
  no `openssl`: the sha256 of `src/`, `keys/`, `openssl/` and `Cargo.toml` of
  `crates/tls-test-certs` against `certs/DIGEST`. It prints `tls-certs: <n> cases, certs/
  matches its sources`, or `tls-certs: the sources changed since certs/ was generated; run task
  tls-certs-slow` with the sources that differ, and fails.
- `task slow`: the lane that makes the committed products of the TLS tests again and compares
  them with the committed files; never a step of `check`. It runs `tls-certs-slow`, which
  builds and runs `surf-tls-test-certs` and `openssl/make.sh` into `.out/tls-certs/` and
  compares every file with `crates/tls-test-certs/certs/` byte for byte, then
  `tls-flights-slow`, which records the flights against `openssl s_server` on 127.0.0.1 into
  `.out/tls-flights/` and replays them with the tests `flights::` (outcomes, not bytes: the
  server draws its own randomness). It needs `openssl` 3.4 or later on `PATH`; the committed
  files were made with the version their `SOURCE` files name. No workflow runs it: the forge's
  runner pins no `openssl`, and the certificates `make.sh` writes come from that tool. Run it
  by hand after a change to `crates/tls-test-certs/` or to
  `crates/tls/examples/record-flights.rs`.
- `task view-test`: the page tests in a browser (`taskfiles/view-test.yml`). It builds
  `surf-agent`, `surf-session` and `surf-view-fixture`, installs `view-tests/` from its
  `package-lock.json` with `npm ci`, and runs `npx playwright test --workers=2` in
  `view-tests/`. It installs no browser: Chromium's headless shell of the revision
  `@playwright/test` names must be in the Playwright cache. On a new machine, once:
  `npm --prefix view-tests ci`, then `npx --prefix view-tests playwright install --only-shell
  chromium`. A change to what a `go` records is checked with `task view-test` too, because
  the crate tests do not run the page tests.
- `task view-stream-check`: the event names of `docs/view-stream.md` against the AsyncAPI
  projection of `spec/` (`taskfiles/view-stream-check.yml`). It fails, naming each, for a name
  that is no message of the projection; a new event whose name nobody adds to the document
  passes, so a story that maps a new event adds its name there.
- `task html-conformance`: `surf-html-conformance --check` against `crates/html-conformance/
  expected/`, with the base revision `HTML_BASE` (default `main`; `task html-conformance
  HTML_BASE=<revision>`). Log `.out/html-conformance.log`. In a clone where `main` is not a
  branch, pass `HTML_BASE`.
- `task agent-tasks-report`: prints `.out/agent-tasks.txt`, which `tasks::every_task` of
  `crates/agent-tasks` writes, and fails when it is missing; `task check` runs it after `test`.
- `task sh-echo`: `sh taskfiles/sh-echo.sh`, every test of `surf-agent-tasks` but
  `tasks::every_task`, and `slowest_tests` of `surf`, run by cargo nextest with
  `.out/sh-echo/bin/sh` (`bash --posix -O xpg_echo`, whose `echo` expands backslashes as dash's
  does and takes no option) first on `PATH`; it first checks that `echo 'a\nb'` prints two lines
  there. A script started through `#!/bin/sh` still gets `/bin/sh`. `FILTER=<expression>`
  replaces the filter. On `ab573de` its `sh` fails the three tests the forge failed, one each
  of `adversary2`, `adversary` and `model_driver`.
- `task sh-echo-test`: `adversary2::every_call_is_one_line_that_reads_back` through that
  script on a copy of `taskfiles/agent-tasks-model.sh` (`ADVERSARY_MODEL_SCRIPT`): the copy as
  it is passes, the copy whose wrapper's awk program is written with `echo` fails.
  `SCRIPT=<path>` runs a copy of the script.
- `task release-smoke DIR=<unpacked archive>`: `taskfiles/release-smoke.sh` on a release
  archive, as the release workflow runs it (`docs/releasing.md`).
- `task validate`, `task suite`, `task conform`: the specification steps on their own.
- `task conform-rust`: the crate `conformance/`, warnings denied. Its tests, the planted-defect
  test included, then the suite against the Rust system: log `.out/conform-rust.log`, report
  `.out/conform-rust.json` (`ess-conformance-report/2`). Fails when any scenario failed, errored
  or was unsupported. cargo fetches `github.com/beyond10x/ess` at tag `0.97.3` when its cache
  does not hold it. `task --summary conform-rust` says more.
- `task conform-totals`: fail, printing both, when `.out/report.json` (interpreter) and
  `.out/conform-rust.json` (Rust) are of different suites by digest or differ in
  `counts.total`. Needs both runs first.
- `task lock-check`: runs `sh taskfiles/lock-check.sh Cargo.lock taskfiles/lock-allow.txt`.
  It fails, one line per finding, when the root `Cargo.lock` holds a package with a `source`
  line (one from a registry or a git repository) that matches no line of
  `taskfiles/lock-allow.txt` on name, version, source and checksum (`Cargo.lock: <name>
  <version> from <source>: not on the allow-list`, with the listed line when the name is
  listed); when a crate of this repository other than `surf-tls` depends on a listed package
  (`Cargo.lock: <package> depends on <allowed>; only surf-tls may`); when a listed line is in
  no package (`taskfiles/lock-allow.txt: <name> <version> is in no package of Cargo.lock`);
  when a line of the lock is in no form cargo writes (`Cargo.lock:<n>: not in the form cargo
  writes: <line>`); and when the lock is missing. The check refuses a lock that cargo would
  accept in another spelling (keys without spaces, a comment after `[[package]]`, a
  dependency array on one line, literal strings, `[metadata]`, `[[patch.unused]]`, the
  `replace = "…"` line cargo writes for `[replace]`); it still reads such a lock for the
  other findings, so its packages are named too. Else it prints `Cargo.lock: <n> package(s),
  19 from outside this repository, each on the allow-list`. The list holds the 19 packages of ring 0.17.14
  (`docs/design/tls.md`, item 2), one line `<name> <version> <source> <checksum>` each,
  sorted; the script's head states the format. It does not read `conformance/Cargo.lock`. A
  path dependency outside the repository has no `source` line and is not caught.
- `task generate`: rewrite `generated/rust/` from `spec/`, in place. `ess` refuses, naming the
  file, when a file it wrote was edited since; delete that file or restore it from git, then run
  it again.
- `task generated-check`: generate into `.out/generated-check/` and fail with one line per path
  that differs from `generated/rust/`, in its bytes or its executable bit. `task --summary generated-check` names what it leaves out
  and why. It is two tasks: `generated-fresh` generates `spec/` into
  `.out/generated-check/fresh` (with `fresh.log` and a copy of `spec/` as `fresh.spec`), once per
  run of `task`; `generated-compare FRESH=<tree>` compares and regenerates `.ess-output` in
  place. The tests of `crates/surf/tests/*generated_check*.rs` read that one generation
  (`crates/surf/tests/common/mod.rs`) and run `generated-compare` in their copies; they fail,
  saying so, when it is missing or was made from another `spec/`. `cargo test -p surf` needs
  `slot task generated-fresh` first; `task test` depends on it.
- `task build`: `cargo build` of the root workspace with `--locked` and `RUSTFLAGS=-D warnings`.
- `task test`: `cargo nextest run --workspace --locked` (cargo-nextest 0.9.143, installed as a
  tool; no crate of the workspace changes) and `cargo test --doc --workspace --locked`, which
  nextest does not run, with `RUSTFLAGS=-D warnings`. The ten slowest tests come from
  `taskfiles/slowest-tests.sh <junit file> [count]` (POSIX sh, pipefail where the shell has
  it), held by `crates/surf/tests/slowest_tests.rs` on a report of 2,000 cases. Under
  `pipefail` no pipeline of a task ends its reader before its input (no `| head`, `grep -q` or
  `sed q` after a command that may write more than a pipe's 64 KiB): the reader stops, the
  writer dies of SIGPIPE and the task fails with 141 (2026-11-22, the list of 892 tests, 70,331
  bytes, under `sort -rn | head -n 10`). Logs `.out/test.log` and
  `.out/test-doc.log`, JUnit report `target/nextest/default/junit.xml`. Two bounds, from the
  rules of the folder this repository is worked in: `.config/nextest.toml` ends a test still
  running after 60 s and counts it failed (`slow-timeout` with `terminate-after`), and the task
  fails when its two commands together run over 120 s of wall time (`TEST_BUDGET`, not counting
  the time `slot` queues the task), naming the time. It prints the ten slowest tests from the
  JUnit report, nextest's `Summary` line, the count line `N tests: P passed, F failed, I ignored
  (cargo-nextest n, doc tests d)` (a test nextest ended counts as failed, one it skips as
  ignored), and `test step: <s> s of its budget of 120 s`. Raising either bound is not a fix: a
  test over the budget is. A test bounds a time only by a wait for the event with a limit at
  least 10 times the time it takes alone, and proves a cost by a count (such as
  `surf_page::attribute_visits()`), not by a wall time.
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
- `cargo test -p <package> --test main` does not build the package's examples, nor does
  `cargo test -p <package> --test main <module>::`, which selects one module of the package's
  one test binary. The tests of `surfd` and `surf-agent` that run an example need the whole
  package run (`cargo test -p surfd`); `crates/view/tests/prototype.rs` builds its example
  itself.
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

After round 1 (2026-11-22, on the wave branch at `fe5c723`: the parser, text and document units
and `story:agent-tasks` merged on `main` at `5415f27`), with ess 0.97.3: `169 scenario(s) (24
authored), 48 refusal(s)` (`slot task suite`; the document unit read the same on 0.96.0).
Against the interpreter and against the Rust system: `169 scenarios: 169 passed, 0 failed, 0
error, 0 unsupported`; `conform-totals`: `the interpreter and the Rust system ran 169
scenario(s) each, of suite
sha256:0aead5f74409f8c561f544df8056b9e6b6fe146c13ba2e5e3a3fbf0ffdea733c`. The planted defect:
`planted defect: 169 scenarios: 133 passed, 36 failed, 0 error, 0 unsupported`; the
conformance crate's `tests/main.rs` ran 15 passed. `generated-check`: `generated/rust matches
spec/ (30 artifact(s) and .ess-output)`, the committed bytes. `html-conformance: 9107 cases,
9095 run, 9095 passed`: the parser unit raised `crates/html-conformance/expected/`, which the
gate holds. `task test-layout`: `test-layout: 19 test directories, 107 test files, one test
binary each` (files under `tests/cases/` are checked but not counted; 106 before
`crates/surf/tests/slowest_tests.rs`, 3 tests). The text unit's four
stories and their two fix rounds add 81 tests to the root workspace (`surf-answer` 38 to 55,
`surf-page` 58 to 65 with the parser unit's two, `surf-page-text` 0 to 57), counted from the
runner's summary lines; the whole count is the gate's to print. The gate's first run on the
wave branch, `task check-runner` on `6b2e3a6`, printed `877 tests: 876 passed, 1 failed, 0
ignored (cargo-nextest 875, doc tests 2)`; `264c962` fixed the one failure
(`surf-agent-tasks`, `tasks::a_call_that_never_answers_ends_at_its_bound`) without adding or
removing a test. The run on `264c962` printed `877 tests: 877 passed, 0 failed, 0 ignored
(cargo-nextest 875, doc tests 2)`, `test step: 62.3 s of its budget of 120 s`, `surf v1 — 9
file(s), 24 scenario(s), valid`, `Cargo.lock: 26 package(s), none from outside this
repository` and `agent-tasks: 4 tasks, 2 pass, 2 fail, 0 skipped, lane gate, binaries
target/debug` (`article` and `table` pass, `link` and `search` are pending on later stories).
That run was not green: `task view-test` failed one case, `dropped entries are a gap`, which
held the 20 entries four `go`s dropped before `story:html-document-port`; the parser step
records `DocumentParsed` and one `ElementAdded` per actionable element, 28 are dropped now,
and the test derives the number from the entries' `index` since.

With `story:ess-0-97-3` (2026-11-22, on `896c64d`), surf builds on ess 0.97.3: `surf v1 — 9
file(s), 15 scenario(s), valid` and `96 scenario(s) (15 authored), 28 refusal(s)`, as with
0.96.0. The suite is byte for byte the one 0.96.0 writes, digest
`sha256:3041446cb0234779316d887f5a96d127f60fc03893977e2b341f92e69af28b81`, so no scenario
was lost or gained, and the 28 `refused:` lines are the same (neither breaking change of the
release reaches surf: its tasks pass no `--component`, and the specification serves no
component, so it owes no read-grant scenario). `generated/rust/` is what 0.97.3 generates,
unchanged, 30 artifacts. Against the interpreter: `96 scenarios: 96 passed, 0 failed, 0
error, 0 unsupported`; against the Rust system the same line, the same 96 scenarios passing
by name; `conform-totals`: `the interpreter and the Rust system ran 96 scenario(s) each`. The
conformance crate's tests: 13, 0 and 15 passed, the planted defect `96 scenarios: 62 passed,
34 failed, 0 error, 0 unsupported`, as on 0.96.0. The conformance crate's lock
changed only in the eight `ess-*` packages of the tag; no third-party package moved. `ess
verify conform synthesize` of `spec/` at `896c64d`, five runs of each release in turn at a load
average of 164: 0.96.0 906 to 1,303 ms (median 1,086 ms), 0.97.3 605 to 881 ms (median 763
ms).

With `story:one-test-binary-per-crate` (2026-11-22, on `main` at `b98ab10`), one test binary
per crate: `task test` prints `662 tests: 662 passed, 0 failed, 0 ignored (cargo-nextest 660,
doc tests 2)`, the same 660 tests under nextest, each now named `<file>::<name>` in the binary
`<package>::main`, and in the last of three runs
`test step: 15.3 s of its budget of 120 s`
(15.7 s, 16.5 s and 15.3 s at a load average of 13.7 to 14.4; on `b98ab10` before the change
16.6 s, 17.4 s and 15.3 s at 22.1 to 27.0). The root workspace's test executables fell from 80
(146,442,536 bytes) to 16 (41,435,688 bytes), one per crate. The conformance crate's tests:
`4 test suites: 28 passed` (the unit tests of `src/lib.rs` 13, of `src/main.rs` 0,
`tests/main.rs` 15, doc tests 0; 6 suites before), and `task conform-rust` printed `96
scenarios: 96 passed, 0 failed, 0 error, 0 unsupported`. `task test-layout`: `test-layout: 17
test directories, 84 test files, one test binary each`.

On 2026-11-22 with ess 0.96.0, after wave 10 of the agent interface plan, on the wave branch
at `1b234c4` (`main` of release `0.3.0` merged) with the comment on `label` in
`spec/domains/document.yaml` changed after it: `surf v1 — 9 file(s), 15 scenario(s), valid`
(`slot task validate`) and `96 scenario(s) (15 authored), 28 refusal(s)` (`slot task suite`).
Against the interpreter (`slot task conform`): `96 scenarios: 96 passed, 0 failed, 0 error, 0
unsupported`. The suite's digest, the sha256 of the bytes of `.out/suite.json` that
`conform-totals` prints (read with `sha256sum` and from the `digest` of `.out/report.json`):
`sha256:3041446cb0234779316d887f5a96d127f60fc03893977e2b341f92e69af28b81`. Wave 10 changed it
(`surf.document.ParseLimit` gained `Steps`, and `parse-stopped-at-a-limit.yaml` a sixth
document); the comment on `label` changed no byte of the suite. The planted defect, the test
alone: `planted defect: 96 scenarios: 62 passed, 34 failed, 0 error, 0 unsupported`.
`generated-check`: `generated/rust matches spec/ (30 artifact(s) and .ess-output)`. The root
workspace: `task test` is to print `661 tests: 661 passed, 0 failed, 0 ignored (cargo-nextest
659, doc tests 2)`, the 579 of `main` and the 82 `#[test]` lines the wave adds (36 in
`crates/html/tests/limits.rs`, 11 in `adversary_limits.rs`, 34 in `crates/page/tests/label.rs`,
1 unit test in `crates/html/src/tree.rs`; none removed), counted from the diff, not yet from a
run of the gate. The last gate run on `main` printed `test step: 61.9 s of its budget of 120 s`,
at a load average of 106 to 113. The Rust system's run, `conform-totals`, `cli`, `lock-check`,
`view-test` and `html-conformance` were not run for these lines; the wave changed no manifest
and no `Cargo.lock`, and its units reported `html-conformance: 9107 cases, 9095 run, 8428
passed`, unchanged.

After wave 9, on 2026-11-21 with ess 0.96.0, at `a9532a1`:
`surf v1 — 9 file(s), 14 scenario(s), valid` and `95 scenario(s) (14 authored), 28
refusal(s)`. Against the interpreter: `95 scenarios: 95 passed, 0 failed, 0 error, 0
unsupported`. Against the Rust system (`conform-rust`): `95 scenarios: 95 passed, 0 failed, 0
error, 0 unsupported`. `conform-totals`: `the interpreter and the Rust system ran 95
scenario(s) each`, of suite
`sha256:5fe34265b01df3ccfc43f30d4ddb41759c22407845edb2e1e5e675481bd29b9f`. Both reports say
`conformance_status: inconclusive`, with `execution_status: passed`: the suite carries no
coverage inventory (`coverage.knowledge: unknown`), and ess 0.96.0 never qualifies a run over
unknown coverage. The gate reads the counts. The planted defect, a `Navigation` storage that
drops every `put`, fails 33 of the 95. The crate's tests: `6 test suites: 28 passed`.
`generated-check`: `generated/rust matches spec/ (30 artifact(s) and .ess-output)`. `cli`:
`cli/surf-agent.yaml — valid CLI presentation binding`. `lock-check`: `Cargo.lock: 24 package(s),
none from outside this repository`. The root workspace, with `cargo test`: `126 test suites: 555
passed, 0 failed, 0 ignored`. At `a9532a1` it was 553; the two tests of `surf-view-fixture` came
with the fix unit after wave 9. Since the story `gate-test-step-within-budget` `task test` runs
cargo-nextest. Since the story `local-gate-equals-forge` (run of `task check-runner` at
`de08b24`, 2026-11-22) it prints
`561 tests: 561 passed, 0 failed, 0 ignored (cargo-nextest 559, doc tests 2)`, the same 561, and
`Summary [ … ] 559 tests run: 559 passed, 0 skipped`, and
`test step: 22.2 s of its budget of 120 s`. The 561 are the 555 of before plus the 8 tests of
`surf-test-process`, minus the 2 tests of `surf-view-fixture` whose subject moved there, and the
package count rose from 23 to 24 with that crate. Before that story it printed `555 tests: 555
passed, 0 failed, 0 ignored (cargo-nextest 553, doc tests 2)` and `Summary [ … ] 553 tests run:
553 passed, 0 skipped`. Its budget: no test over 60 s, the step within 120 s. Measured on
2026-11-22 on the shared, loaded machine, after `task build`, before that story:
`test step: 28.8 s`, `25.9 s` and `28.7 s` of its budget in three runs (33 to 36 s for the whole
task with `cli` and `generated-fresh`); before, `cargo test --workspace` ran 177 s, with
`review1_one_changed_byte_in_any_generated_file_fails_naming_it` over 60 s. The ten slowest tests
of the last of those runs: `review1_one_changed_byte_in_any_generated_file_fails_naming_it`
16.4 s (31 comparisons against one generation), `request_slower_than_its_deadline_is_closed`
(`surf-session` `built`) 10.2 s, `adversary_an_action_recorded_after_a_session_timeout_has_its_entries_kept`
(`surf-session` `adversary_events`) 7.2 s, `surfd_that_writes_nothing_exits_3_within_6_seconds`
(`surf-agent` `start`) 5.1 s, `every_character_is_written_as_printable_ascii_and_reads_back`
(`surf` `escape`) 4.8 s, then five tests of `surf-view` and `surf-session` between 2.3 and 3.7
s. Three of them wait on purpose: `built` and `start` wait out a deadline of the product
(`wire::REQUEST_TIMEOUT`, 10 s, and `wire::START_TIMEOUT`, 5 s, in `crates/agent/src/wire.rs`),
and `adversary_events` waits out `surfd`'s answer timeout of 2 s, a server that answers after 4
s and 5 s more, so that each step of the order it tests is 2 s or more apart from the next and
the test does not depend on the runner's speed. In the root workspace, `surf-url` prints `urltestdata: 904 cases, 872 run,
872 passed, 32 skipped`; the 32 are hosts outside ASCII, listed in
`crates/url/tests/data/SKIPPED`. `view-stream-check`: `view-stream-check: 19 event name(s),
each a message of the AsyncAPI projection`. `view-test`: `38 passed (15.8s)`; 12.9 s of it are
the case that waits to see that a refused token is not sent again. `html-conformance`, the last
step of `check`: `html-conformance: 9107 cases, 9095 run, 8428 passed` (82 encoding cases,
7,046 tokenizer cases and 1,300 tree construction cases), with `tag markup 1979/1979/1979`,
`tag text 328/328/328`, `tag charref
4739/4739/4739`, `tag body 476/476/476`, `tag formatting 22/22/22`, `tag tables 11/39/39`, `tag
template 30/123/123`, `tag foreign 6/222/222`, `tag mixed 755/1085/1085` and `tag encoding
82/82/82`; the section `tree-construction` passes 1300 of 1809 (503 failing, 6 not run). The
cases of the tags `tables`, `template` and `foreign` that pass, pass without rules of their own
(through the generic rules, and `template.dat` case 84 through the adoption agency algorithm);
each later tree story measures its tag again. Before wave 9 these were 22 packages, 121
suites, 503 passed, `38 passed (17.1s)`, 8362 passed cases (1,234 tree construction cases),
`tag formatting 8/22/22`, `tag template 29/123/123` and `tag mixed 704/1085/1085`; the
scenarios, the refusals, the suite's digest, the planted defect's 33 of the 95, the
conformance crate's tests, the 30 artifacts and the page tests' count did not change. Before
wave 8 these were 12 authored scenarios, 93 scenarios, the suite
`sha256:4cd1257a06a838a1bb2779b0309c347b1b6262703ff814da340988c32ecb7a3a`, 33 of the 93 failed
by the planted defect, `6 test suites: 27 passed` in the conformance crate, 118 suites, 486
passed, `24 passed (14.6s)`, 7121 passed cases (75 encoding cases and 7,046 tokenizer cases)
and `tag encoding 75/82/82`; the refusals (28, the same lines), the 22 packages and the 30
artifacts did not change. Before wave 7 these were 21 packages, 109 suites, 453 passed, 4442
passed cases and `tag charref 2060/4739/4739`, and `check` had no `view-test`; the scenarios,
the refusals, the suite's digest and the 30 artifacts did not change. Before wave 6 these were 9 authored
scenarios, 83 scenarios, 27 refusals and the suite `sha256:4d9ad0c3…`, 102 suites, 407 passed,
4164 passed cases, `tag text 57/328/328` and `tag charref 2053/4739/4739`, and `check` had no
`view-stream-check`; the 21 packages and the 30 artifacts did not change. Before wave 5 they
were 97 suites, 363 passed, and 5 passed cases. Before
wave 4 they were 7 authored scenarios, 81 scenarios, 31 failed by the planted defect, 20
packages, 88 suites and 324 passed; the refusals (27, the same lines) and the 30 artifacts did
not change, and the HTML conformance gate was not part of `check`.

With `story:security-review-fixes` (2026-11-22, the authored scenario
`page-load-fails-at-a-forbidden-destination.yaml` and the reason `ForbiddenDestination`):
`surf v1 — 9 file(s), 15 scenario(s), valid`, `96 scenario(s) (15 authored), 28 refusal(s)`,
`96 scenarios: 96 passed, 0 failed, 0 error, 0 unsupported` against the interpreter and against
the Rust system, and `conform-totals` of suite
`sha256:579a46b3389c56bb67b8fc3174268f525b8568ee47446e67d2a1cca936660ba8`; the refusals are the
same 28 and `generated-check` still names 30 artifacts. Read again on the release branch of
`0.3.0` with this story merged (`slot task validate`, `conform`, `conform-rust`,
`conform-totals`, `generated-check`, 2026-11-22): the same lines, and the planted defect
`planted defect: 96 scenarios: 62 passed, 34 failed, 0 error, 0 unsupported`, so 34 of the 96;
the conformance crate's tests are still `6 test suites: 28 passed`. The story adds 18 tests to
the root workspace (14 in its first commit, 4 in the review fix), so `task test` is to print
`579 tests: 579 passed, 0 failed, 0 ignored (cargo-nextest 577, doc tests 2)`, counted from the
`#[test]` lines of the diff; the gate on `main` counted the same 579 since.

`REFUSALS_MAX` in `Taskfile.yml` is 48. `task suite` fails above it. Lower it when a refusal goes
away; raise it only with the new refusal listed here. The list is `grep '^refused:'
.out/synthesize.log` (read on 2026-11-21 after wave 6: 28 lines, the same as below; the gate
after wave 7 printed `28 refusal(s)`; in wave 8 the unit of `agent-request-body-spec` read the
same 28 lines; the gate after wave 9 printed `28 refusal(s)`, and `task suite` after wave 10
too; after round 1, 48 lines with ess 0.97.3: the 28 and the 20 of `story:agent-forms-spec`
listed last).
Twenty-two of the 28 are `ESS-SYNTH-010`:

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

The 20 refusals `story:agent-forms-spec` added (28 to 48), with cause:

- `ESS-SYNTH-014` × 4: `Check/radio-checked`, `Check/unchecked`, `Check/checked`,
  `SetValue/set`. The `Elements` view declares an `order_by`, and synthesis refuses an outcome
  that updates an entity such a view orders.
- `ESS-SYNTH-008` × 1: `Check/radio-checked`. Its `affects:` filter reads the subject's stored
  fields (`observe_subject_fact`).
- `ESS-SYNTH-001` × 1: `SelectOption/selected`. The disabled and hidden refusals answer first on
  every subject synthesis arranges.
- `ESS-SYNTH-010` × 14, the bindings' cases, all `at_most_once` with `on_failure: drop`:
  delivery and on-failure (no redelivery to observe; drop is unobservable) of all five new
  bindings (10); flow of `check-on-pick-requested`, `select-on-pick-requested` and
  `set-value-on-fill-requested` (the invoked command's branch is decided by an input or a
  stored row) (3); condition-false of `select-on-pick-requested` (a `defined(...)`
  condition) (1).
- Evidence for each refused outcome and flow: the seven authored scenarios
  `agent-fill-sets-value`, `agent-pick-unchecks-radio-group`, `agent-pick-selects-option`,
  `document-refuses-disabled-control`, `document-refuses-hidden-control`,
  `agent-press-submits-post`, `agent-press-submission-ends-without-body`.

One note is printed with them: the specification declares actors and no component is served over
a network, so no `<command>/grant/denied` scenario is generated.
