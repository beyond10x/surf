#!/bin/sh
# The tests that run a script with the sh of PATH, run again under an sh whose echo turns a
# backslash sequence into a control character, as dash's does (the sh of the forge's runner).
#
# Usage: sh taskfiles/sh-echo.sh [<nextest filter>]
#
# - Where /bin/sh is bash, echo prints a backslash as it is, and a script that writes a value
#   with echo passes here and fails on the runner. The stand-in is
#   .out/sh-echo/bin/sh: bash --posix -O xpg_echo, whose echo expands \n, \\ and the others and
#   takes no option, as POSIX leaves both open. It is first on PATH for the tests only.
# - Before the tests it checks that the stand-in expands: echo 'a\nb' must print two lines.
# - The default filter: every test of surf-agent-tasks but tasks::every_task (which writes
#   .out/agent-tasks.txt), and slowest_tests of surf. A filter given replaces it, in the
#   packages surf-agent-tasks and surf.
# - It runs cargo nextest with the environment of the caller: task sh-echo sets RUSTFLAGS and
#   SURF_BIN_DIR as task test does, so nothing is built again.
#
# It prints nextest's summary line, or the log of the run when a test failed, and exits as
# nextest did.

set -u

[ "$#" -le 1 ] || {
  echo 'usage: sh taskfiles/sh-echo.sh [<nextest filter>]' >&2
  exit 2
}
filter=${1:-'(package(surf-agent-tasks) & not test(=tasks::every_task)) | (package(surf) & test(/^slowest_tests::/))'}

bash=$(command -v bash) || {
  echo 'sh-echo: bash is not on PATH; the stand-in sh is bash --posix -O xpg_echo' >&2
  exit 1
}
dir=.out/sh-echo
mkdir -p "$dir/bin" || exit 1
dir=$(cd "$dir" && pwd) || exit 1
printf '#!/bin/sh\nexec %s --posix -O xpg_echo "$@"\n' "$bash" > "$dir/bin/sh"
chmod 755 "$dir/bin/sh"

lines=$(PATH=$dir/bin:$PATH sh -c "echo 'a\\nb'" | wc -l)
[ "$lines" -eq 2 ] || {
  printf '%s\n' "sh-echo: the stand-in $dir/bin/sh printed $lines line(s) for echo 'a\\nb', not 2" >&2
  exit 1
}

status=0
PATH=$dir/bin:$PATH cargo nextest run -p surf-agent-tasks -p surf --locked --color never \
  -E "$filter" > "$dir/test.log" 2>&1 || status=$?
if [ "$status" -ne 0 ]; then
  cat "$dir/test.log"
fi
summary=$(grep -E '^ *Summary \[' "$dir/test.log" | tail -n 1)
printf 'sh-echo: %s\n' "$(printf '%s' "${summary:-no summary line}" | sed 's/^ *//')"
exit "$status"
