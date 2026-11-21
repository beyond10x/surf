# Making surf public: what the sibling repositories do

Status: a survey, 2026-11-21, by a read-only agent over the local checkouts of `canon`,
`harness`, `metaharness`, `gates`, `docs-system`, `website` and `atlas`. Nothing is published
yet. The operator asked for it the same day: push the repository, make it public, employ the
gates, add a documentation website in the style of the other public ones, not integrated into
the shared documentation.

## What the survey found

- **Who creates the repository.** No command is documented in the siblings. In the `atlas`
  checkout, `workspace/AGENTS.md` gives repository creation, enrolment in the private gates
  policy and rulesets to the steward session `b10x-atlas`, after the operator approves; new
  repositories are on its never-delegate list. So this session does not create the repository.
- **Enrolment comes first.** Every `b10x-gates` verb stops with `repository is not enrolled`
  until the private policy has an entry for the repository (a numeric id, a baseline commit, a
  signer grant). `b10x-gates policy` has no subcommand that adds one; it is a reviewed change by
  the steward.
- **Authorship.** The gates admit commits whose author and committer are the organization bot
  (`b10x-gates bot -- commit`). The local history of surf is authored by the operator's GitHub
  identity, so it is not admissible as it is: what goes public starts at a bot-made baseline.
- **What the gates refuse**, without printing the match: secrets (Gitleaks), private
  identifiers from the policy, home-directory paths, workflow actions not pinned to a commit,
  workflows without declared permissions.
- **The planning store.** `canon`, `harness` and `metaharness` track `.engineering/` in public,
  with a named human actor in their evidence files. A first name there is not what the gates
  refuse; a home path is.
- **The website.** `canon` and `harness` use Docusaurus 3.10.2 with the shared product template
  (`@beyond10x/docs-system`, `withProductSite`), sources under `website/docs/`, `url`
  `https://beyond10x.github.io`, `baseUrl` `/<repo>/`, and two workflows: `pages.yml` builds and
  uploads, `b10x-docs-site.yml` calls the website repository's reusable deploy workflow. Pages is
  created once through the bot. A Rust `<repo>-docs` crate binds the build to its commit.
- **Not to add** (that is the shared documentation): `b10x.docs.yaml`,
  `.github/workflows/b10x-docs-bundle.yml`, `b10x-docs-pages.yml`, the generated
  `b10x-docs:` blocks in `README.md` and `AGENTS.md`, and `atlas docs reconcile`.
- **Furniture of a public sibling:** `LICENSE` (canon: Apache-2.0) or a stated licence,
  `CHANGELOG.md`, `SECURITY.md` (harness), `.github/workflows/shared-gates.yml` pinned to a
  commit of the gates repository, a correctness workflow that runs `task check`. None of the
  three has badges or a logo in its README; surf's are local files, so nothing conflicts.

## Order of work

1. The operator decides what goes public (a fresh clean commit, recommended; the gates'
   authorship rule excludes the local history as it is) and the licence.
2. Clean the tree: home paths become relative or `$HOME`; the operator's name in `AGENTS.md`
   and the plan documents becomes "the operator" if he wants that.
3. The steward creates `beyond10x/surf`, enrols it, and later sets the rulesets.
4. Here: `b10x-gates install`, `audit`, the two gate workflows, licence and changelog, then
   `check`, `verify`, `publish` as the bot.
5. The website, built by a sub-agent from `canon`'s as the pattern, with surf's logo; Pages
   through the bot; the repository homepage set after the site answers.
