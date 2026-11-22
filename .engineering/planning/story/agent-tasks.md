---
format: aep.planning-md/3
id: story:agent-tasks
kind: story
status: draft
title: 'Agent tasks: five tasks solved with surf-agent alone, scripted in the gate and run by a model outside it'
summary: Five fixture sites on 127.0.0.1 with questions and expected answers; a scripted agent in the gate holds which pass; a model lane outside it records its result in the release notes.
relations:
- decomposes: epic:agent-cli-target
revision: 1
---
## Context

The operator, 2026-11-22: "I wonder what is needed to actually make [surf] useful for agents,
because right now I think it isn't." The coordinator's order of the remaining work (D85) asks
for a measure beside the six items: a set of agent tasks. An answer of release 0.3.0 names a
page's status, media type and address and nothing of its content, and only `http:` loads, so
today none of the tasks below can be solved.

The folder's rules (`$HOME/beyond10x/AGENTS.md`, "Tests stay fast"): the test step makes no
model call and no network request; no test runs over 60 s; the step ends within 120 s. So the
measure has two lanes: a scripted agent in the gate, and a real model outside it.

## The five tasks

Each task is a fixture site that the tests serve on 127.0.0.1, a question, and the expected
answer. An agent solves a task with `surf-agent` and nothing else.

| # | task | fixture | expected answer | needs |
|---|---|---|---|---|
| 1 | read an article and answer a question: "In which year was the bridge opened?" | `/article.html`, an article of about 800 words with headings, the year in the fourth paragraph only | `1932` | round 1 (page text) |
| 2 | read a table: "What does one kilo of pears cost?" | `/prices.html`, a table with a header row, six rows, a caption | `2.40` | round 1, its table story and `html-tree-tables` |
| 3 | go from an index page to a page by a link: "What are the opening hours on Saturday?" | `/index.html` with twelve links, one labelled "Opening hours", to `/about/hours.html` | `10 to 14` | rounds 1 and 3 (`follow`) |
| 4 | send a search form: "What is the title of the first result for lighthouse?" | `/search.html`, a `GET` form with one text field and a submit button; `/results` answers from the query | `The Lighthouse at Point Reyes` | rounds 1 and 3 (`fill`, `press`) |
| 5 | read a page behind `https:`: "What does the notice say is closed?" | `https://localhost:<port>/notice.html`, served over TLS 1.3 with a certificate of a test root that the session is given | `the north entrance` | rounds 1 and 2 (TLS) |

Which round makes which pass, by the coordinator's order (D85): round 1 (read a page,
`plan/page-as-tree-and-markdown`) makes 1 pass, and 2 once its table story lands; round 2
(`epic:tls-https`) makes 5 pass; round 3 (links, forms, history: `story:agent-follow-links`,
`story:agent-fill-pick`, `story:agent-press-submit`) makes 3 and 4 pass. Round 4 (cookies,
`story:agent-cookie-jar`), round 5 (scripts) and round 6 (`story:agent-skill-definition`) make
none of the five pass; the cookie story adds a sixth task (log in and read a page behind the
login) in its own change. The skill story changes the model lane: its text is the model's only
instruction.

## Change

- A crate of test tooling, `crates/agent-tasks` (`surf-agent-tasks`), standard library only, not
  shipped (the lock check holds): the five fixture sites as files under `sites/`, a server that
  serves them on 127.0.0.1 port 0 (the TLS server of task 5 is the test server of
  `epic:tls-https`), and the file `tasks.txt`: per task its number, start address, question and
  expected answer, one field per line, in a format the two lanes read.
- The scripted lane, in the gate: one test per task in `tests/main.rs`. Each runs the built
  `surf-agent`, `surfd` and `surf-session` (or those in `SURF_BIN_DIR`, as the `e2e` test of
  `surf-session` does) in a private `XDG_RUNTIME_DIR`, with a `PATH` that holds only them. It
  issues the commands a model would (`open`, `look --as markdown`, `follow <handle>`, `fill
  <handle> -`, `press <handle>`, `close`), finds each handle by reading the answer through
  `surf_answer::parse` (by label, never by a handle written into the test), and passes when a
  line of a `look` answer holds the expected answer. It makes no model call.
- What the lane holds: `crates/agent-tasks/expected.txt`, one line per task, `pass` or `pending
  <story id>`. A task marked `pass` that fails fails the gate; a `pending` task that passes prints
  `raise: task <n>` and does not fail, as `surf-html-conformance --check` does for its counts. The
  first line printed is `agent-tasks: 5 tasks, <n> pass, <m> pending`, and the task `check`
  prints it.
- The model lane, outside the gate: `task agent-tasks-model MODEL=<name>` starts the fixture
  server, gives a model the output of `surf-agent help --for-agent` (or the README section until
  `story:agent-skill-definition` lands) and one task's question, allows it one tool, a shell
  limited to `surf-agent`, and compares its last answer with the expected one, word for word after
  lower-casing. It writes `.out/agent-tasks-model.txt`: per task pass or fail, the number of
  calls, and the commands issued. `docs/releasing.md` gains a step: the release notes carry the
  line `agent tasks: <n> of 5 with <model> on <date>`, copied from that file. It never runs in
  `check`, `check-runner` or a forge workflow.

## Acceptance

- `slot task check` prints `agent-tasks: 5 tasks, <n> pass, <m> pending` with `n + m = 5`, and on
  `main` after this story `0 pass, 5 pending`, each pending line naming the story that makes it
  pass.
- Planting a defect fails a passing task: with the expected answer of task 1 changed in
  `tasks.txt`, the test of task 1 fails once task 1 is `pass` (a test of the harness, against a
  fixture answer it serves itself, so it can be shown before round 1).
- Each task's test ends within 5 s on the shared machine; the five together add at most 15 s to
  the test step, which stays within 120 s.
- The crate depends on `surf-answer` and `surf-test-process` only, starts no program but the
  three binaries, and its fixture server counts the connections it accepts; each test checks that
  the session made exactly the connections the task needs, all to 127.0.0.1 (task 5 through the
  name `localhost`).

## Out of scope

- Scoring a model, comparing models, or a task that needs scripts.
- Tasks against sites on the internet; the measure is reproducible because it is local.
- Choosing the model of the model lane: the release record names whichever was run.

## Why `epic:agent-cli-target`

Its acceptance already names a walk test in which "an agent with nothing but `surf-agent`" acts on
a page (`story:agent-change-and-log`); this story turns that into five tasks with expected
answers and a count the gate holds. The tasks span rounds of other epics (`epic:tls-https`, the
read-a-page plan), which this story only names as what makes a task pass.
