---
format: aep.planning-md/3
id: review-result:plan-critic-acceptance-round-1
kind: review-result
status: active
title: Plan critic (acceptance), round 1, epic headless-rust-target
relations:
- reviews: story:net-fetch-failure-reasons
- reviews: story:generated-rust-workspace
- reviews: story:http-fetch-library
- reviews: story:memory-ports
- reviews: story:headless-cli
- reviews: story:rust-conformance
revision: 2
---
needs-revision
story:http-fetch-library — the Change makes a connect timeout and a read timeout each end in a failure and never a hang, but the acceptance ("one test per case listed above") covers only the 12 listed cases. None of them stalls a server, so a missing read timeout or connect timeout passes — .engineering/planning/story/http-fetch-library.md:59
story:memory-ports — the acceptance requires only one test (the five-port `OpenWindow` and `Windows` view case), so the storage-order tests, the 1,000-identity v4 test and the unreadable-random-source `TryContext` obligation can be absent while "passes" holds, and no check covers the claim that sharing one store between two components does not compile — .engineering/planning/story/memory-ports.md:49
story:headless-cli — the acceptance binds to "the five cases above", and none of them exercises exit status 3 for an unmet obligation or `FailRequest` with `MalformedResponse`, although the Change states both — .engineering/planning/story/headless-cli.md:53
story:rust-conformance — the acceptance's "the recorded planted defect made the same task exit non-zero" does not say which task "the same" is (`check` or `conform-rust`) or where the red run is recorded, and the Change puts its summary line in the report under `.out/`, which `.gitignore` excludes, so nobody can later check that it happened — .engineering/planning/story/rust-conformance.md:45

What I read: all 6 given ids with `aep plan artifact show` (whole bodies) and `aep plan artifact graph`, plus the critic rubric and procedure, `$HOME/beyond10x/surf/.gitignore`, `Taskfile.yml` and a grep of `spec/`. I did not run `aep plan artifact kinds` or `lifecycle`.

What I could not establish:
- `generated/rust` does not exist, so I could not check the symbols memory-ports and headless-cli name (`Generated<P>`, `TryContext`, the `opened` outcome of `OpenWindow`).
- Whether `ess` offers a Rust conformance runner is stated as open in the story. That is scope and design, not acceptance, so it did not set my verdict.
- I did not flag the "X, and Y" acceptances of `net-fetch-failure-reasons` and `generated-rust-workspace`. Each pairs a gate guard with the transition that distinguishes before from after, so I judged both checkable.

```findings
- file: .engineering/planning/story/http-fetch-library.md
  line: 59
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Change makes a connect timeout and a read timeout each end in a failure and never a hang, but the acceptance (\"one test per case listed above\") covers only the 12 listed cases, none of which stalls a server, so a missing read timeout or connect timeout passes"
- file: .engineering/planning/story/memory-ports.md
  line: 49
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance requires only one test (the five-port OpenWindow and Windows view case), so the storage-order tests, the 1,000-identity v4 test and the unreadable-random-source TryContext obligation can be absent while \"passes\" holds, and no check covers the claim that sharing one store between two components does not compile"
- file: .engineering/planning/story/headless-cli.md
  line: 53
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance binds to \"the five cases above\", none of which exercises exit status 3 for an unmet obligation or FailRequest with MalformedResponse, although the Change states both"
- file: .engineering/planning/story/rust-conformance.md
  line: 45
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance's \"the recorded planted defect made the same task exit non-zero\" does not say which task \"the same\" is (check or conform-rust) or where the red run is recorded, and the Change puts its summary line in the report under .out/, which .gitignore excludes, so nobody can later check that it happened"
```
