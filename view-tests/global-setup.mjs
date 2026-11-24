// Before any browser starts: the page tests run in a network namespace of their own with only
// loopback (taskfiles/page-netns.sh, story:view-live-marker-under-load). Chromium cancels its
// requests in flight with net::ERR_NETWORK_CHANGED when an address of any interface it sees
// changes; on the host's network a container that starts or stops does that, and a page's module
// files were lost so. Outside such a namespace the run fails here with one line.
//
// The interfaces are read from /proc/self/net/dev, which lists those of this process's network
// namespace; /sys/class/net lists those of the namespace that mounted /sys, the host's.
//
// SURF_VIEW_TESTS_HOST_NETWORK=1 lets a person debug a test on the host's network; task
// view-test, task page-step-cpu and the gate never set it.
import { readFileSync } from "node:fs";

const ESCAPE = "SURF_VIEW_TESTS_HOST_NETWORK";

export default function globalSetup() {
  if (process.env[ESCAPE] === "1") {
    return;
  }
  const names = readFileSync("/proc/self/net/dev", "utf8")
    .split("\n")
    .slice(2)
    .map((line) => line.split(":")[0].trim())
    .filter((name) => name !== "");
  const others = names.filter((name) => name !== "lo");
  if (others.length > 0 || !names.includes("lo")) {
    throw new Error(
      `page tests: not in a network namespace of their own (interfaces: ${names.join(", ") || "none"}); ` +
        `run them through taskfiles/page-netns.sh (task view-test does), or set ${ESCAPE}=1 to debug on the host's network`,
    );
  }
}
