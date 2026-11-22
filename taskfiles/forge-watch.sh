#!/bin/sh
# Wait for the forge's workflow runs of one commit and fail when one of them did not pass.
#
#   sh taskfiles/forge-watch.sh <commit>
#   sh taskfiles/forge-watch.sh --runs <file>
#
# The first form reads the runs of <commit> through the organization bot
# (`b10x-gates ... api --method GET`) until every run is completed, then prints one line per
# run, "<name>: <status> <conclusion> <id>", and decides. It exits 0 when every run concluded
# success, skipped or neutral and a run of the workflow `check` is among them; 1 when a run
# concluded otherwise, when no run of `check` was listed within FORGE_WATCH_APPEAR seconds, or
# when a run was still not completed after FORGE_WATCH_TIMEOUT seconds; 2 when the runs cannot
# be read.
#
# The second form decides on a file of runs as the forge's API answers them
# (`{"workflow_runs": [{"name", "status", "conclusion", "id"}, ...]}`), without the network,
# and prints the same lines: 0 and 1 as above, 3 when a run is not completed yet, 4 when every
# run is completed and none is of `check`, 2 when the file cannot be read. `task
# forge-watch-test` checks it on the files under taskfiles/forge-watch-test/.
#
# The environment:
# - GATES_POLICY: the bot's policy file (default ../gates-policy/policy.json, relative to the
#   repository root);
# - FORGE_REPOSITORY: owner/name on the forge (default beyond10x/surf);
# - FORGE_WATCH_INTERVAL: seconds between two reads (default 20);
# - FORGE_WATCH_APPEAR: seconds a run of `check` may take to be listed (default 180);
# - FORGE_WATCH_TIMEOUT: seconds to wait for every run to complete (default 3600);
# - FORGE_WATCH_DIR: where each read is written (default .out/forge-watch).

set -eu

usage() {
  echo "usage: sh taskfiles/forge-watch.sh <commit> | --runs <file>" >&2
  exit 2
}

# The decision on one file of runs; prints the lines and the decision, returns 0, 1, 2, 3 or 4.
decide() {
  file=$1
  verdict=$(jq -r '
    .workflow_runs as $runs
    | if ($runs | type) != "array" then "unreadable"
      elif ($runs | any(.status != "completed")) then "pending"
      elif ($runs | any(.name == "check") | not) then "no-check"
      elif ($runs | all(.conclusion == "success" or .conclusion == "skipped" or .conclusion == "neutral")) then "green"
      else "red" end' "$file" 2> /dev/null) || verdict=unreadable
  if [ "$verdict" = unreadable ]; then
    echo "forge-watch: $file: not a list of workflow runs"
    return 2
  fi
  jq -r '.workflow_runs[] | "\(.name): \(.status) \(.conclusion // "-") \(.id)"' "$file"
  count=$(jq '.workflow_runs | length' "$file")
  case $verdict in
    green)
      echo "forge-watch: $count run(s) completed, each concluded success, skipped or neutral"
      return 0
      ;;
    red)
      jq -r '.workflow_runs[]
        | select(.conclusion != "success" and .conclusion != "skipped" and .conclusion != "neutral")
        | "forge-watch: \(.name) (\(.id)) concluded \(.conclusion // "nothing")"' "$file"
      return 1
      ;;
    pending)
      echo "forge-watch: $(jq '[.workflow_runs[] | select(.status != "completed")] | length' "$file") of $count run(s) not completed"
      return 3
      ;;
    no-check)
      echo "forge-watch: $count run(s) completed, none of the workflow check"
      return 4
      ;;
  esac
}

[ "$#" -ge 1 ] || usage
command -v jq > /dev/null 2>&1 || { echo "forge-watch: jq is not installed" >&2; exit 2; }

if [ "$1" = --runs ]; then
  [ "$#" -eq 2 ] || usage
  status=0
  decide "$2" || status=$?
  exit "$status"
fi
[ "$#" -eq 1 ] || usage

root=$(git rev-parse --show-toplevel)
cd "$root"
sha=$(git rev-parse --verify --quiet "$1^{commit}") || { echo "forge-watch: $1: no commit of that name" >&2; exit 2; }
policy=${GATES_POLICY:-../gates-policy/policy.json}
repository=${FORGE_REPOSITORY:-beyond10x/surf}
interval=${FORGE_WATCH_INTERVAL:-20}
appear=${FORGE_WATCH_APPEAR:-180}
timeout=${FORGE_WATCH_TIMEOUT:-3600}
dir=${FORGE_WATCH_DIR:-.out/forge-watch}
mkdir -p "$dir"
file=$dir/runs-$sha.json
log=$dir/read-$sha.log

start=$(date +%s)
while :; do
  if ! b10x-gates --repo . --policy "$policy" --repository "$repository" api --method GET \
    --path "/repos/$repository/actions/runs?head_sha=$sha&per_page=100" --output "$file" > "$log" 2>&1; then
    cat "$log" >&2
    echo "forge-watch: $sha: the runs could not be read through b10x-gates" >&2
    exit 2
  fi
  status=0
  out=$(decide "$file") || status=$?
  waited=$(( $(date +%s) - start ))
  case $status in
    0 | 1 | 2)
      printf '%s\n' "$out"
      echo "forge-watch: $sha after $waited s"
      exit "$status"
      ;;
    3)
      if [ "$waited" -ge "$timeout" ]; then
        printf '%s\n' "$out"
        echo "forge-watch: $sha: not every run completed within $timeout s"
        exit 1
      fi
      ;;
    4)
      if [ "$waited" -ge "$appear" ]; then
        printf '%s\n' "$out"
        echo "forge-watch: $sha: no run of the workflow check within $appear s"
        exit 1
      fi
      ;;
  esac
  printf '%s\n' "$out" | tail -n 1 >&2
  sleep "$interval"
done
