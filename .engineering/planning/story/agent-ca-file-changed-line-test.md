---
format: aep.planning-md/3
id: story:agent-ca-file-changed-line-test
kind: story
status: draft
title: A test reads the refusal of a --ca-file file that changed while it was read
summary: the client prints one refusal line when the opened --ca-file file is not the file it looked at, and no test makes it print that line; the comparison gets a form a test can call without a race
relations:
- decomposes: epic:tls-https
- informed_by: story:tls-session-anchors
revision: 1
---
## Why

Since the fix round of `story:tls-session-anchors`, the client looks at a `--ca-file` path
before it opens it, and compares device and inode of the opened file with the ones it looked
at (`read_ca_file` in `crates/agent/src/client.rs`). A mismatch ends the command with
`surf-agent: the --ca-file file changed while it was read`. `docs/agent-answer.md` lists the
line. No test makes the client print it: the path would have to be swapped between the two
calls. The other four refusal lines of the same function each have a test.

## Change

To be made exact by the scoper and the critics; the direction:

- The comparison becomes a function of the two metadata values that a test can call with two
  different files, or the function takes the opened file from a caller the test can replace.
  No new dependency, no sleep and no thread race in the test.

## Acceptance

Draft:

1. A test gives the check the metadata of one regular file and an opened second regular file,
   and reads the refusal line byte for byte as `docs/agent-answer.md` lists it.
2. With the inode comparison removed, that test fails (the change is tried and removed).
3. Every test of the modules `ca_file` and `adversary_ca_file` of `surf-agent` passes
   unchanged; the new test runs under 1 s.

## Out of scope

Closing the window between the look and the open by another system call; symbolic links, which
the client follows.

## Writes

`crates/agent/src/client.rs`, `crates/agent/tests/` (one module).
