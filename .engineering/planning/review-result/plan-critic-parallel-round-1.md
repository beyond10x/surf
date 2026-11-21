---
format: aep.planning-md/3
id: review-result:plan-critic-parallel-round-1
kind: review-result
status: active
title: Plan critic (parallel), round 1, epic headless-rust-target
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
story:http-fetch-library — its acceptance (`cargo test -p surf-fetch`) runs in the root Cargo workspace that story:generated-rust-workspace creates, and no depends_on edge runs from it to that story, so one wave would hold both and each would create the root `Cargo.toml` and `Cargo.lock`, which do not exist yet (cited, both bodies). The fix is to add depends_on story:generated-rust-workspace, or to merge the two. — .engineering/planning/story/http-fetch-library.md:59, .engineering/planning/story/generated-rust-workspace.md:26
story:http-fetch-library — this story and story:memory-ports are unordered, and both add a crate to the root workspace. The workspace body says only "members under `crates/`" and commits the root `Cargo.lock`, which gains an entry per member crate (inferred from how Cargo records workspace members). Neither body admits the shared `Cargo.lock`, or the root `Cargo.toml` if members are listed explicitly. Either add an ordering edge that records the shared file as its reason, or have the workspace story fix a `crates/*` glob so the two items no longer share a file. — .engineering/planning/story/generated-rust-workspace.md:40, .engineering/planning/story/http-fetch-library.md:25, .engineering/planning/story/memory-ports.md:26

What I read: 6 stories and the epic, using `aep plan artifact graph`, `aep plan artifact show` on each, `$HOME/beyond10x/surf/Taskfile.yml`, and `$HOME/beyond10x/surf/AGENTS.md` "The gate's numbers".
- Surfaces: 6 items placed from what the bodies cite (the edit targets: `spec/domains/net.yaml`, `generated/rust`, root `Cargo.toml`, `Taskfile.yml`, `AGENTS.md`, `.gitignore`, `crates/fetch`, `crates/ports`, `crates/surf`, `README.md`, and the conformance test crate under `crates/`), 0 inferred beyond the Cargo.lock/members point above, 0 unplaceable.
- Acceptance reads traced to a producer in the set: 6, of which 5 are recorded by a `depends_on` edge. The one without an edge is the first finding.
- Edges that cover shared files:
  - `AGENTS.md` and `Taskfile.yml` are edited by net-fetch-failure-reasons, generated-rust-workspace, headless-cli and rust-conformance. They sit on one depends_on chain (net, then workspace, then cli, then conformance), so those collisions are ordered.
  - memory-ports reads `generated/rust/.../behaviour.rs`, and its edge to generated-rust-workspace covers that.
  - The planted-defect edit to `crates/ports` in rust-conformance is ordered after memory-ports through headless-cli.

What I could not establish:
- Whether the root workspace lists members explicitly or by glob. That is the implementor's choice per the workspace body, so the second finding rests on an inferred surface.
- Out of my lane, and it did not set the verdict: rust-conformance needs the `System` assembly, which headless-cli puts in a binary crate (`crates/surf`) that the conformance crate is told not to depend on. Whether that split is right is for the design critic.

```findings
- file: .engineering/planning/story/http-fetch-library.md
  line: 59
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance (cargo test -p surf-fetch) runs in the root Cargo workspace that story:generated-rust-workspace creates, and no depends_on edge runs from it to that story, so one wave would hold both and each would create the root Cargo.toml and Cargo.lock, which do not exist yet (cited, both bodies); add depends_on story:generated-rust-workspace or merge the two"
- file: .engineering/planning/story/generated-rust-workspace.md
  line: 40
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:http-fetch-library and story:memory-ports are unordered and both add a crate to the root workspace, so both change the committed root Cargo.lock (inferred from how Cargo records workspace members) and the root Cargo.toml if members are listed explicitly, and neither body says so; either add an ordering edge recording the shared file as its reason, or have the workspace story fix a crates/* glob so they no longer share a file"
```
