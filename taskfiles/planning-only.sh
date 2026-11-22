#!/bin/sh
# Decide whether a change is planning or prose only, so that the workflow `check` may skip
# building and testing it on a pull request (.github/workflows/check.yml).
#
#   git diff --name-only <base>...<head> | sh taskfiles/planning-only.sh
#
# It reads one path per line, relative to the repository root, from standard input. It exits 0
# when every path is a planning or prose file, and 1 when one is not or when there is none; it
# prints each path that is not, and one line with the decision.
#
# The one list of planning and prose paths:
# - `.engineering/**`: the planning store;
# - `docs/plan/**`: the wave plans and their pages;
# - `*.md` anywhere else, `README.md` and `AGENTS.md` included, except:
#   - under `website/`: the documentation site is built from it;
#   - under `crates/`, `conformance/`, `generated/`, `spec/`, `cli/` and `view-tests/`: the
#     code and its data;
#   - `docs/design/html-parser.md` and `docs/view-stream.md`: the gate reads them
#     (`task html-conformance`, `task view-stream-check` and a test of `surf-view`).

set -eu

total=0
other=0
while IFS= read -r path || [ -n "$path" ]; do
  [ -n "$path" ] || continue
  total=$((total + 1))
  case $path in
    .engineering/*) continue ;;
    docs/plan/*) continue ;;
    website/* | crates/* | conformance/* | generated/* | spec/* | cli/* | view-tests/*) ;;
    docs/design/html-parser.md | docs/view-stream.md) ;;
    *.md) continue ;;
  esac
  other=$((other + 1))
  echo "planning-only: $path: not a planning or prose file"
done

if [ "$total" -eq 0 ]; then
  echo "planning-only: no changed file; the whole gate runs"
  exit 1
fi
if [ "$other" -ne 0 ]; then
  echo "planning-only: $other of $total changed file(s) not planning or prose; the whole gate runs"
  exit 1
fi
echo "planning-only: $total changed file(s), all planning or prose; the build and the tests are skipped"
