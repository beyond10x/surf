---
format: aep.planning-md/3
id: story:public-website
kind: story
status: active
title: A public documentation website for surf, published on its own
summary: A small Docusaurus site after the sibling repositories' pattern at beyond10x.github.io/surf, not part of the shared documentation.
scope:
- confidence: cited
  path: .github/workflows/b10x-docs-site.yml
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: .github/workflows/pages.yml
- confidence: cited
  path: .github/workflows/shared-gates.yml
- confidence: cited
  path: .gitignore
- confidence: cited
  path: taskfiles/site.yml
- confidence: cited
  path: website
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T01:32:42Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T01:32:42Z", actor: "human:timo", revision: 3}
---
## Context

The operator, 2026-11-21, the day the repository became public: "add a documentation just in
the style as all the other public ones, a public website documentation, like [...] harness or
canon [...]. Do not integrate it into the beyond10x super documentation", and: "It can be super
minimal, but there should be." `docs/plan/publishing.md` holds the survey of how the sibling
repositories publish their sites.

## Change

- `website/`: a Docusaurus site after canon's pattern (the shared product template pinned to a
  commit), with a landing page and docs pages taken from `README.md`, `docs/surf-command.md` and
  `docs/plan/agent-waves.md`: what surf is and is not, principles, what works, the roadmap, try
  it, the `surf` command, how it is built, status. The logo is `docs/logo.svg`.
- `.github/workflows/pages.yml` and `b10x-docs-site.yml` (build and deploy as an independent
  project site), `shared-gates.yml`, and `check.yml` running `task check`.
- `taskfiles/site.yml` with the site's build task.
- Not added: `b10x.docs.yaml`, the shared bundle and pages workflows, generated `b10x-docs:`
  blocks.

## Acceptance

`https://beyond10x.github.io/surf/` answers 200 with the landing page, and two docs pages under
it answer 200, fetched with `curl` after the deploy workflow's run on `main` succeeded; the
site's build exits 0 with broken links failing it; the repository holds no `b10x.docs.yaml`.

## Out of scope

Generated reference pages, a search index, versioned docs, any page about plans or reviews.
