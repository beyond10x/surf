#!/bin/sh
# The sum of the four timed steps of task check: task step-budgets, its last step.
#
# usage: sh taskfiles/step-budgets.sh [<directory>]   (default .out)
#
# It reads the seconds each step wrote into <directory>: test-step.seconds (task test),
# tls-test-step.seconds (task test-tls), conform-step.seconds (task conform-rust) and
# page-step.seconds (task view-test), and prints one line:
# "four steps: N s (test step A s, tls test step B s, conformance step C s, page step D s)",
# N their sum with one decimal. It exits 1 with one line naming the file when one is missing or
# holds no number of seconds. It has no budget of its own: each step fails over its own.
# POSIX sh: the forge's sh is dash.

set -u

dir=${1:-.out}
values=''
for file in test-step.seconds tls-test-step.seconds conform-step.seconds page-step.seconds; do
  path=$dir/$file
  if [ ! -f "$path" ]; then
    printf 'step-budgets: %s: no such file\n' "$path"
    exit 1
  fi
  value=$(cat "$path")
  case $value in
    '' | *[!0-9.]* | *.*.* | .* | *.)
      printf 'step-budgets: %s: not a number of seconds\n' "$path"
      exit 1
      ;;
  esac
  values="$values $value"
done

# shellcheck disable=SC2086
set -- $values
awk -v a="$1" -v b="$2" -v c="$3" -v d="$4" 'BEGIN {
  printf "four steps: %.1f s (test step %s s, tls test step %s s, conformance step %s s, page step %s s)\n", a + b + c + d, a, b, c, d
}'
