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
revision: 2
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
  first step is `open --ca-file crates/tls-test-certs/certs/ok-rsa2048/root.pem <address>`, and the
  connections counted are those the TLS server counts.

## Acceptance

`slot task check` exits 0; the folder's bounds hold (the test step within 120 s, no test over 60
s, no model call and no network request: the TLS server listens on 127.0.0.1, `localhost` is the
name its certificate carries; one TLS server per run, never per test; no Python). `every_task`
stays within 10 s; the time is in the report.

1. `agent-tasks-report` prints `task https pass 3 calls 1 connections` among its task lines, and
   its first line counts one task more than before this story, with one pass more.
2. `format::commands_match_the_steps` passes with the `https` steps `open look close`;
   `fixtures::each_answer_is_on_its_target_page_only` passes for `sites/https/`.
3. With `status.txt` listing `https pass` and the extra anchor file left out of the `open` (a
   case of module `tasks`, `https_without_the_root_fails`, judging a run of the one task): `open`
   exits 0 (a failed load is no refusal, docs/agent-answer.md:256-258) and answers `page failed
   CertificateNotTrusted "https://localhost:<port>/notice.html"`; `look --as markdown` answers
   `note text: no-document`; the reporting function returns `agent-tasks: task https is listed
   pass and failed: answer "the north entrance" not in the text`.
4. `git diff --stat` of the unit names only `crates/agent-tasks/` and, from the coordinator,
   `Cargo.lock`.

## Out of scope

- TLS in the model lane's fixture server; a task against a site on the internet.
- The TLS client, the anchors and `open --ca-file` themselves (`epic:tls-https`).
