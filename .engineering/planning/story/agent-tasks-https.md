---
format: aep.planning-md/3
id: story:agent-tasks-https
kind: story
status: draft
title: 'The https agent task: a page behind https: on the TLS test server, read with surf-agent alone'
summary: 'Adds the https row to crates/agent-tasks: a notice served by the TLS test server, opened with open --ca-file and the test root, read with look --as markdown; listed pass.'
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-tasks
- depends_on: story:tls-session-anchors
- depends_on: story:page-text-look
scope:
- confidence: cited
  path: crates/agent-tasks
revision: 3
---
## Context

`story:agent-tasks` measures what an agent can do with `surf-agent` alone, one row of
`crates/agent-tasks/tasks.txt` per task, and names a page behind `https:` among the tasks of the
measure (D85, `wave/a10-label-limits:docs/plan/agent-waves.md:161-165`, a file of that branch
until it merges). That task needs `surf-agent open --ca-file` and `https:` in the session
(`story:tls-session-anchors`), the TLS test server (`story:tls-test-server`, whose context names
it the server of this task, tls-test-server.md:28-30) and its test certificates
(`crates/tls-test-certs/certs/ok-rsa2048/root.pem`, the root `story:tls-anchors` case 2 uses), and
`look --as markdown` (`story:page-text-look`) to read the page. Neither `story:agent-tasks` nor
`story:tls-session-anchors` ships it (tls-session-anchors.md, "Out of scope", names this story).

## Decisions taken

Taken by the coordinating agent on 2026-11-22, reversible, reported to the operator:

1. The `https` agent task is a story of its own, depending on `story:agent-tasks` and
   `story:tls-session-anchors`. This story also depends on `story:page-text-look`, so its row can
   be listed `pass` when it lands.
2. Placement: after the merges of those three; it writes only `crates/agent-tasks`, so it runs
   beside any unit that does not write that crate (`story:agent-cookie-jar` writes it: not in the
   same wave).
3. The model lane does not run this task (`story:agent-tasks`: a row whose `server` is not `http`
   is written `skipped`); the fixture server of the model lane serves no TLS.
4. Round 3 of the panel: the run without the root is part of `tasks::every_task`, in the one
   `surfd` against the one TLS server, as a second session; no other test starts a server. Its
   result is a `check` line of the report, which no count of the first line includes.
5. Round 3 of the panel: the root's path is built from `CARGO_MANIFEST_DIR`, so it holds whatever
   the working directory of the test is.
6. Round 3 of the panel, the address. The host name is `localhost`, as in the end-to-end tests of
   `story:tls-session-anchors` (tls-session-anchors.md:107, :116). The certificate case
   `ok-rsa2048` carries it: leaf `localhost` with `subjectAltName` dNSName `localhost` and
   iPAddress 127.0.0.1 (tls-test-certs.md:65-66). The session resolves a name with the operating
   system's resolver, `to_socket_addrs` on a thread of its own (`crates/fetch/src/lib.rs:775-806`
   on `main`; the same in `resolve_name`, `6b900a5:crates/fetch/src/lib.rs:923-945`, of the
   security fix), and tries each address in turn until one connects
   (`crates/fetch/src/lib.rs:750-770`; `6b900a5:crates/fetch/src/lib.rs:836-874`). On this host
   `/etc/hosts` maps `localhost` to 127.0.0.1 and ::1; the TLS server listens on 127.0.0.1 only,
   so a refused ::1 is followed by 127.0.0.1. No name server is asked.

## Change

- `crates/agent-tasks/Cargo.toml`: dev-dependencies `surf-tls-test-server` and
  `surf-tls-test-certs` by path (test tooling; the crate is not shipped).
- `crates/agent-tasks/sites/https/notice.html` (a short notice; the words `the north entrance`
  once, in its second paragraph).
- `tasks.txt`, a block after the last: `task https`, `server https`, `start /notice.html`, `target
  /notice.html`, `connections 1`, `commands open look close`, `question What does the notice say
  is closed?`, `answer the north entrance`. `status.txt`: `https pass`.
- Module `tasks` of `crates/agent-tasks/tests/main.rs`: `every_task` starts, once per run beside
  the plain fixture server, `surf_tls_test_server::TestServer::start` with the certificate case
  `ok-rsa2048`, serving `/notice.html` (status 200, `Content-Type: text/html`, the file's bytes);
  for a row whose `server` is `https` the start address is `https://localhost:<port><start>`, the
  first step is `open --ca-file <root> <address>` with `<root>` the path
  `concat!(env!("CARGO_MANIFEST_DIR"), "/../tls-test-certs/certs/ok-rsa2048/root.pem")`, and the
  connections counted are those the TLS server counts.
- The same test, after the last task and before the report is written: the check
  `https-without-root`, a second session in the same `surfd` against the same TLS server,
  `open https://localhost:<port>/notice.html` without `--ca-file`, then `look --as markdown`,
  then `close`. It passes when `open` exits 0 and answers `page failed CertificateNotTrusted
  "https://localhost:<port>/notice.html"` (a failed load is no refusal,
  docs/agent-answer.md:256-258) and `look --as markdown` answers `note text: no-document`. Its
  connection is counted in no task line (the `https` task's count is read before the check).
- `src/` of `crates/agent-tasks`, the reporting function: a check result is written as one line
  after the task lines and before the `raise:` lines, `check <name> pass` or `check <name> fail:
  <reason>`; a failed check makes the function return the error `agent-tasks: check <name>
  failed: <reason>` after the file is written. `SURF_AGENT_TASKS=measure` writes the line and
  returns no error.

## What changes when the row lands

Exact lines, with `<n>`, `<p>`, `<f>`, `<s>` the counts of the line before this story's merge:

- `.out/agent-tasks.txt`, first line: `agent-tasks: <n> tasks, <p> pass, <f> fail, 0 skipped`
  becomes `agent-tasks: <n+1> tasks, <p+1> pass, <f> fail, 0 skipped`; after the last task line
  come `task https pass 3 calls 1 connections` and `check https-without-root pass`. On base B of
  `story:agent-tasks` case 1, with no other row added and `link` and `search` still failing: `agent-tasks: 5 tasks, 3 pass, 2 fail, 0
  skipped`.
- `.out/agent-tasks-model.txt`, first line: `agent-tasks-model: <n> tasks, <p> pass, <f> fail,
  <s> skipped, command <c>, model <m>, <date>` becomes `agent-tasks-model: <n+1> tasks, <p> pass,
  <f> fail, <s+1> skipped, command <c>, model <m>, <date>`; the line `task https skipped: server
  https` follows the line of the task before it. `story:agent-tasks` case 7, built from
  `tasks.txt`, then expects `agent-tasks-model: 5 tasks, 1 pass, 3 fail, 1 skipped, command
  stub-model.sh, model stub, <date>` (four `http` rows and this one).
- Release notes (`docs/releasing.md`, written by `story:agent-tasks`): `agent tasks (scripted):
  <n+1> tasks, <p+1> pass, <f> fail, 0 skipped` and `agent tasks: <n+1> tasks, <p> pass, <f> fail,
  <s+1> skipped, model <m>, <date>`. No file of `docs/` changes: the lines are copied from the two
  first lines at a release.

## Acceptance

`slot task check` exits 0; the folder's bounds hold (the test step within 120 s, no test over 60
s, no model call and no network request: the TLS server listens on 127.0.0.1, `localhost` comes
from `/etc/hosts`; one TLS server and one `surfd` per run, never per test; no Python).
`every_task` stays within 10 s; the time is in the report.

1. `agent-tasks-report` prints the lines of "What changes when the row lands": the first line
   with one task and one pass more than before, `task https pass 3 calls 1 connections` after the
   last other task line, then `check https-without-root pass`.
2. `format::commands_match_the_steps` passes with the `https` steps `open look close`;
   `fixtures::each_answer_is_on_its_target_page_only` passes for `sites/https/`.
3. The check: the answers of the second session are compared with the two lines of the Change
   by `every_task` itself. `check::a_failed_check_is_reported` (no process), results with the
   check `https-without-root` failed (reason `open answered page 200`) written to a file under
   `CARGO_TARGET_TMPDIR`: that file holds `check https-without-root fail: open answered page 200`
   after the task lines, and the function returns `agent-tasks: check https-without-root failed:
   open answered page 200`.
4. `model_driver::a_stub_model_is_counted` passes with the first line of "What changes when the
   row lands".
5. `git diff --stat` of the unit names only `crates/agent-tasks/` and, from the coordinator,
   `Cargo.lock`.

## Out of scope

- TLS in the model lane's fixture server; a task against a site on the internet.
- The TLS client, the anchors and `open --ca-file` themselves (`epic:tls-https`).
