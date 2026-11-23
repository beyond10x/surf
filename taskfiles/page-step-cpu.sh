#!/bin/sh
# The CPU seconds and the instructions of the page step (story:page-step-holds-under-load): the
# Playwright call of task view-test, the same command in the same directory, under perf stat -e
# task-clock,instructions. Both counters are inherited on fork, not on wait, so they count every
# thread of every process under the call, the ones nobody reaps included (Chromium's renderers,
# zygote and utility processes): node, Chromium, surf-view-fixture with surfd inside,
# surf-session and surf-agent. time and the shell's times count reaped children only (about 57 %
# of the CPU seconds).
#
# task-clock is user and kernel time: perf names it task-clock:u when kernel.perf_event_paranoid
# is 2, but a software clock counts the time a task runs in either mode (dd from /dev/zero to
# /dev/null, nearly all system time, read 705 ms). instructions are user only at paranoid 2
# (instructions:u: the same dd read 44.5 M, its kernel copies not in them). The bound of the page
# step is in instructions: they are the step's own work, while its CPU seconds follow the host's
# load (AGENTS.md, task page-step-cpu).
#
# Not a cgroup of its own: on this host a user scope (systemd-run --user --scope) read 11 to 12 %
# less than task-clock in each of three runs of the same call (the kernel accounts interrupt time
# apart from a task's time, CONFIG_IRQ_TIME_ACCOUNTING, and the page step's loopback traffic is
# handled in softirqs), and the scope would take the call out of the slot pool's cgroup and its
# CPU limit.
#
# Run from the repository root after the build and setup of task view-test (task page-step-cpu
# does both); every path in it is relative to the current directory, so run from the root of
# another checkout it measures that checkout's tests and binaries. The script takes no slot
# itself; the call takes one (slot --timeout 15m). It prints the 1-minute load before and after,
# the ten slowest tests, the line of task view-test ("page step: N s of its budget of 120 s" or
# "..., over its budget of 120 s"), "page step cpu: T s" and "page step instructions: I G (user
# only)", writes T to .out/page-step.cpu and I to .out/page-step.instructions, and fails when a
# test failed or the step ran over its budget, as task view-test does.
set -u

budget=${PAGE_BUDGET:-120}
junit=.out/view-test/junit.xml
counts=$(pwd)/.out/page-step.perf.csv
times=$(pwd)/.out/page-step.times
mkdir -p .out/view-test
rm -f "$junit" "$counts" "$times" .out/page-step.cpu .out/page-step.instructions \
  .out/page-step.seconds
tests=0
# The load and the start and end are read inside the slot job, so the time the job waited in
# the pool's queue is neither in the step's seconds nor between the two loads.
(cd view-tests && slot --timeout 15m sh -c '
  echo "load before: $(cut -d " " -f 1 /proc/loadavg)"
  date +%s%N > "$2"
  perf stat -e task-clock,instructions -x , -o "$1" -- \
    ./node_modules/.bin/playwright test --workers=2
  status=$?
  date +%s%N >> "$2"
  echo "load after: $(cut -d " " -f 1 /proc/loadavg)"
  exit "$status"
' page-step "$counts" "$times") || tests=1
if [ "$(wc -l < "$times" 2>/dev/null)" != 2 ]; then
  echo "page step: no start and end in $times"
  exit 1
fi
elapsed=$(( ($(sed -n 2p "$times") - $(sed -n 1p "$times")) / 1000000 ))
if [ -f "$junit" ]; then
  echo "the ten slowest tests:"
  sh taskfiles/slowest-tests.sh "$junit" 10
fi
seconds=$(( elapsed / 1000 )).$(( elapsed % 1000 / 100 ))
echo "$seconds" > .out/page-step.seconds
over=0
if [ "$elapsed" -gt $(( budget * 1000 )) ]; then
  echo "page step: ${seconds} s, over its budget of ${budget} s"
  over=1
else
  echo "page step: ${seconds} s of its budget of ${budget} s"
fi
# perf stat -x , writes "<msec>,msec,task-clock[:u],..." and "<count>,,instructions[:u],...".
msec=$(sed -n 's/^\([0-9]*\)[.0-9]*,msec,task-clock.*/\1/p' "$counts" 2>/dev/null)
count=$(sed -n 's/^\([0-9]*\),,instructions.*/\1/p' "$counts" 2>/dev/null)
if [ -z "$msec" ] || [ -z "$count" ]; then
  echo "page step cpu: no task-clock or instructions in $counts"
  exit 1
fi
cpu=$(( msec / 1000 )).$(( msec % 1000 / 100 ))
echo "page step cpu: $cpu s"
echo "$cpu" > .out/page-step.cpu
hundredths=$(( count / 10000000 ))
giga=$(( hundredths / 100 )).$(printf '%02d' $(( hundredths % 100 )))
echo "page step instructions: $giga G (user only)"
echo "$giga" > .out/page-step.instructions
test "$tests" -eq 0 && test "$over" -eq 0
