---
format: aep.planning-md/3
id: story:scripts-utf16-length
kind: story
status: draft
title: Read a script String's length with swell's to_utf16_len, and correct the stale comments
summary: The generated glue counts the code units of a script String with `Cx::to_utf16_len` (HOST-220) instead of an engine call that reads `length`, and the comments that swell 0.9.1 and 0.10.0 made stale in the generated code and in one test are rewritten.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:swell-0-9-0
- depends_on: story:scripts-dom-reading
revision: 1
---
## Context

`story:swell-0-9-0` pins swell `0.10.0`, which holds `Cx::to_utf16_len(&self, string:
StringRef) -> Result<usize, HostError>` (`crates/swell/src/host/mod.rs:548-550 @ 0.10.0`, held
by `HOST-220`, with a test). The glue of `crates/webidl/src/prelude.rs` (copied to
`crates/script/src/bindings.rs`) reads a String's length with `string_length`
(`prelude.rs:782-794 @ 77e8f3f`): `ToObject(string).length` through two engine calls, an
allocation for the key, and an answer as `f64`. That story left the function and the comments
below alone because both files are rewritten by `story:scripts-dom-reading`.

Stale comments: `prelude.rs:236-246` (a listed String lives "from the sentence and the
signature, not from a requirement": `HOST-218` is the requirement); `prelude.rs:365-380` (the
answer `Terminated(Interrupted)` of a host function "works at the tag but swell does not promise
it": `HOST-217` promises it); the cite `object/builtin_function.rs:497 @ 0.8.0` in
`crates/script/src/bindings.rs:443` and `prelude.rs:372`;
`crates/script/tests/adversary_5.rs:226` ("Swell's host interface @ 0.8.0, section 4", for
Strings that `HOST-218` holds). `prelude.rs:310-313` (an interrupt "set after the throw" ends
the read) stays true.

`story:scripts-dom-reading` adds a length check of its own for a member's String answer (a Rust
string, counted in Rust). This story does not change that check.

## Change

- `string_length` in `crates/webidl/src/prelude.rs` calls `Cx::to_utf16_len`; its callers
  (the checks of a name's units, of an empty String, of a one-unit String) keep their meaning.
- The comments above are rewritten for `0.10.0` with citations `@ 0.10.0`.
- `bindings.rs` is regenerated with the task `webidl-check`; the generator digests change.

## Acceptance

1. `git grep -n -F 'ToObject(string).length' -- crates` prints nothing.
2. `git grep -n -F 'to_utf16_len' -- crates/webidl/src/prelude.rs crates/script/src/bindings.rs`
   prints at least one line in each file.
3. `git grep -n -F 'host interface @ 0.8.0' -- crates/script/tests/adversary_5.rs` prints
   nothing (one line on `main`, `:226`).
4. `git grep -c -F 'builtin_function.rs:497 @ 0.8.0' -- crates` prints nothing.
5. The task `webidl-check` passes: `bindings.rs` equals the generated text.
6. `slot cargo nextest run -p surf-script` passes in under 120 s, each test under 60 s, with
   the tests that count code units unchanged (a lone surrogate counts one unit).

## Out of scope

The length check of `story:scripts-dom-reading` for a member's answer; the memory budget's hold
on a String made with `Cx::new_string` (`docs/design/host-interface.md:165-168 @ 0.10.0`); any
change in swell.
