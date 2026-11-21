---
format: aep.planning-md/3
id: review-result:plan-critic-design-round-1
kind: review-result
status: active
title: Plan critic (design), round 1, epic headless-rust-target
relations:
- reviews: story:net-fetch-failure-reasons
- reviews: story:generated-rust-workspace
- reviews: story:http-fetch-library
- reviews: story:memory-ports
- reviews: story:headless-cli
- reviews: story:rust-conformance
revision: 1
---
needs-revision
story:http-fetch-library — its acceptance runs `cargo test -p surf-fetch` and its change puts the crate at `crates/fetch`, which needs the root Cargo workspace that story:generated-rust-workspace creates, and no depends_on edge runs from it to that story; add `depends_on story:generated-rust-workspace` (story:generated-rust-workspace also says it adds no placeholder crate, so the first crate has no other workspace to join) — .engineering/planning/story/http-fetch-library.md:59, .engineering/planning/story/generated-rust-workspace.md:26

**What you read:** 7 artifacts (the epic and the six stories). I ran `aep plan artifact relations`, `graph` and `validate` (valid), `show` on every id, and `AGENTS.md`. I walked 12 edges: 6 `decomposes` and 6 `depends_on`. All of them are inside the set, and the graph shows no edges to other artifacts. There is no cycle. The `depends_on` edges are not a chain: story:http-fetch-library has no needs-first edge, and story:generated-rust-workspace, story:memory-ports and story:headless-cli each have only the edges the acceptance reads require. I traced 9 acceptance reads to a producer in the set. 8 have a `depends_on` edge, directly or through another edge. The ninth is the finding above.

**What you could not establish:**
- story:rust-conformance and story:headless-cli may split one abstraction. The conformance story runs the System "in process" in a crate that is "not a dependency of `surf`". The headless-cli story builds the generated `System` inside the binary crate `crates/surf`. A binary cannot be imported, so the conformance crate either repeats that assembly or the assembly belongs in a shared library crate. No body says which, so I did not make it a finding.
- Conformance depends on `headless-cli` where `memory-ports` and `generated-rust-workspace` may be all it reads. The edge costs ordering and does not break the shape.
- Out of my lane, so it does not set the verdict: story:generated-rust-workspace, story:net-fetch-failure-reasons and story:rust-conformance all edit `AGENTS.md` "The gate's numbers", and conformance and the workspace story both edit the `check` task in `Taskfile.yml`. That is for parallel-safety.

```findings
- file: .engineering/planning/story/http-fetch-library.md
  line: 59
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance runs cargo test -p surf-fetch and its change puts the crate at crates/fetch, which needs the root Cargo workspace that story:generated-rust-workspace creates, and no depends_on edge runs from it to that story; add depends_on story:generated-rust-workspace (story:generated-rust-workspace also says it adds no placeholder crate, so the first crate has no other workspace to join)"
```
