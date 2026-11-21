---
format: aep.planning-md/3
id: review-result:plan-critic-parallel-round-2
kind: review-result
status: active
title: Plan critic (parallel), round 2, epic headless-rust-target
relations:
- reviews: story:net-fetch-failure-reasons
- reviews: story:generated-rust-workspace
- reviews: story:http-fetch-library
- reviews: story:memory-ports
- reviews: story:headless-cli
- reviews: story:rust-conformance
revision: 1
---
approve

**What you read:** six stories, the epic's graph, and AGENTS.md, Taskfile.yml and the repository root. I ran `aep plan artifact graph`, `aep plan artifact show` on all six, `aep plan artifact relations`, `aep plan artifact waves` and `ls generated`. Placement: 6 of 6 items cited and 0 inferred or unplaceable. Of 9 acceptance reads traced to a producer in the set, all 9 are ordered, 8 by a direct `depends_on` edge and 1 transitively.

- **Cited surfaces:**
  - `net-fetch-failure-reasons` lands on `spec/domains/net.yaml`, AGENTS.md "The gate's numbers" and `REFUSALS_MAX` in Taskfile.yml.
  - `generated-rust-workspace` lands on `generated/rust`, root `Cargo.toml`, Taskfile.yml, `.gitignore`, `Cargo.lock`, and AGENTS.md "Layout" and "Commands".
  - `http-fetch-library` lands on `crates/fetch` and `Cargo.lock`.
  - `memory-ports` lands on `crates/ports` and `Cargo.lock`.
  - `headless-cli` lands on `crates/surf`, README.md and AGENTS.md.
  - `rust-conformance` lands on a test crate under `crates/`, Taskfile.yml and AGENTS.md "The gate's numbers".
- **Concurrent pair:** by the `depends_on` edges the order is net-fetch, then workspace, then {http-fetch-library, memory-ports}, then headless-cli, then rust-conformance. The only items that can run together are `http-fetch-library` and `memory-ports`.
- **Their shared file:** both add a `Cargo.lock` entry. Both bodies say that file is the coordinator's in a wave and that the unit's change to it is not merged, so the collision is named.
- **Shared files outside that pair:** the root `Cargo.toml` uses the `crates/*` glob, so neither story edits it. AGENTS.md and Taskfile.yml are shared by net-fetch, workspace, headless-cli and rust-conformance, which sit in one `depends_on` chain.

**What you could not establish:**
- `aep plan artifact waves` reports all six as unassessed because no scope is recorded yet. The scopers are still writing, so I used the body citations only.
- `headless-cli` and `rust-conformance` each add a crate and so change `Cargo.lock`, but neither body says so. Each runs alone in its wave, so there is no collision.
- `rust-conformance` reads `surf-ports`' `Navigation` storage through the `surf` library. That order is only transitive, through `headless-cli`, which is enough to order the two.
- Out of my lane:
  - `rust-conformance` leaves the choice of ESS runner open, which the acceptance critic should read.
  - `generated-rust-workspace` leaves the workspace-join choice to the implementor, which is a design question.

```findings
[]
```
