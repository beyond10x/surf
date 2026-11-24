---
format: aep.planning-md/3
id: decision-blocker:script-end-record
kind: decision-blocker
status: open
title: Nobody has decided how the specification records a script's end and how that record relates to its document
summary: 'The record of each script''s end has no shape in an ess/1 document: cardinality, owner and when it is written are open; story:scripts-spec waits for it.'
relations:
- blocks: story:scripts-spec
revision: 1
---
## Question

How does the specification record the end of one script of a page, and how does that record
relate to its document?

`epic:scripts-in-a-page` says "Each script's end is recorded with its document (ran, threw,
refused by swell, ended by a limit, not run and why), and a page whose script fails still shows
everything else" (epic, "Outcome"), and leaves "the specification shape of a script's end" to
`story:scripts-decisions` (epic, "Candidate stories", item 1). No `ess/1` document declares a
script or its end: `spec/domains/document.yaml` on `main` of 2026-11-24 has `Document` (line 136)
with `stopped_by` for the parser's limits, `Form` owned by its document (relation `forms`, line
183) and `Element` referencing it (relation `document`, line 297), and says the node tree is not
modelled (lines 130-133). No code records a script.

## The undecided relation

The document and the record of one script's end: how many per document (one per script element
that the parser reached, one per script that ran, or a list field on `Document`), which side owns
the other, when a record is written (while `Parsing`, before `FinishParsing`), whether a record
outlives its document, and whether a document parsed without scripts records its script elements
at all ("not run: scripting off"), or nothing.

## What it stops

`story:scripts-spec`, which writes the record into `spec/domains/document.yaml`, and through it
`story:scripts-document-port`, which issues the command. No other story of the epic depends on
the answer: the stories of `surf-script` record a script's end as a Rust value.

## The answer the decomposer would take

A record per script element the parser reached in a document whose session asked for scripts,
owned by its document (`owns`, many, by `document_id`, as `forms` is), written while the document
is `Parsing` in document order with a position, never outliving the document; its end one of
ran, threw, refused, ended by a limit, not run with a reason; no record in a document parsed
without scripts. The note's Q7 confirms or replaces it, and the operator clears this blocker.

## Decision
