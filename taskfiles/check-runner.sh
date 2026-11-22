#!/bin/sh
# Run the gate as the forge's runner runs it (.github/workflows/check.yml), on the committed tree.
#
#   sh taskfiles/check-runner.sh
#
# 1. It refuses, naming the files, when the working tree has uncommitted changes (changed,
#    staged or untracked files): the gate would not test what is merged. ALLOW_DIRTY=1 lets it
#    run HEAD all the same; it then prints the files it leaves out.
# 2. It brings a clean export of HEAD to the commit: .out/runner/tree, a git clone of this
#    repository that shares its objects (git clone --shared), kept between runs with its own
#    target/. git checkout --force rewrites only the files whose content differs from HEAD, so
#    a file that did not change keeps its time and cargo does not build it again; git clean
#    then removes every file HEAD does not hold, ignored ones included, but the directories
#    named target. No file that exists only in the working tree reaches the export.
# 3. It runs task check in the export with the runner's environment: CI=true,
#    GITHUB_ACTIONS=true, CARGO_TERM_COLOR=always, CARGO_INCREMENTAL=0, RUSTC_WRAPPER empty,
#    ESS_TOOLCHAIN the release spec/ess-inputs.yaml requires, HTML_BASE=origin/main when the
#    export has that ref (its origin is this repository), and without the variables that would
#    change what cargo, rustup and rustc do (RUSTUP_TOOLCHAIN, CARGO_TARGET_DIR,
#    CARGO_BUILD_TARGET_DIR, RUSTFLAGS, CARGO_ENCODED_RUSTFLAGS, RUSTDOCFLAGS). rustc is the one
#    rust-toolchain.toml of the export names.
#
# The page tests' view-tests/node_modules is not committed; task view-test installs it in the
# export with npm ci, as the workflow does. Chromium's headless shell comes from the user's
# Playwright cache, as on any machine that runs task check.
#
# CHECK_RUNNER_DIR moves the export (default .out/runner, relative to the repository root).
# It exits with the status of task check, or 1 when it refuses, 2 when it cannot export.

set -eu

root=$(git rev-parse --show-toplevel)
cd "$root"
sha=$(git rev-parse --verify --quiet 'HEAD^{commit}') || { echo "check-runner: no commit at HEAD"; exit 2; }

dirty=$(git status --porcelain=v1 --untracked-files=all)
if [ -n "$dirty" ]; then
  if [ "${ALLOW_DIRTY:-}" != 1 ]; then
    printf '%s\n' "$dirty" | sed 's/^/  /'
    echo "check-runner: the working tree has uncommitted changes, listed above; the gate would not test what is merged"
    echo "check-runner: commit them, or run with ALLOW_DIRTY=1 to gate HEAD without them"
    exit 1
  fi
  printf '%s\n' "$dirty" | sed 's/^/  /'
  echo "check-runner: ALLOW_DIRTY=1: the changes listed above are not in the export; it gates HEAD $sha"
fi

dir=${CHECK_RUNNER_DIR:-.out/runner}
mkdir -p "$dir"
dir=$(cd "$dir" && pwd)
tree=$dir/tree
source=$(git rev-parse --path-format=absolute --git-common-dir)
start=$(date +%s)
if [ ! -d "$tree/.git" ]; then
  rm -rf "$tree"
  git clone --quiet --shared --no-checkout "$source" "$tree" || { echo "check-runner: cannot clone $source"; exit 2; }
fi
git -C "$tree" fetch --quiet --prune origin || { echo "check-runner: cannot fetch into $tree"; exit 2; }
git -C "$tree" checkout --quiet --force --detach "$sha" || { echo "check-runner: cannot check out $sha in $tree"; exit 2; }
git -C "$tree" clean -ffdxq -e target/ || { echo "check-runner: cannot clean $tree"; exit 2; }
left=$(git -C "$tree" status --porcelain=v1 --untracked-files=all --ignored=no)
if [ -n "$left" ]; then
  printf '%s\n' "$left" | sed 's/^/  /'
  echo "check-runner: $tree differs from $sha after the export"
  exit 2
fi
exported=$(( $(date +%s) - start ))

ess=$(sed -n 's/^requires: ess \([0-9][0-9.]*\)$/\1/p' "$tree/spec/ess-inputs.yaml")
[ -n "$ess" ] || { echo "check-runner: spec/ess-inputs.yaml has no line \"requires: ess <release>\""; exit 2; }
if git -C "$tree" rev-parse --verify --quiet 'origin/main^{commit}' > /dev/null; then
  base=origin/main
else
  base=
fi

cd "$tree"
unset RUSTUP_TOOLCHAIN CARGO_TARGET_DIR CARGO_BUILD_TARGET_DIR RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTDOCFLAGS HTML_BASE
export CI=true GITHUB_ACTIONS=true CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0 RUSTC_WRAPPER= ESS_TOOLCHAIN="$ess"
if [ -n "$base" ]; then
  export HTML_BASE="$base"
fi
echo "check-runner: HEAD $sha exported to $tree in $exported s"
echo "check-runner: $(rustc --version), ess $ess, HTML_BASE=${base:-unset (task's default main)}"
status=0
task check || status=$?
echo "check-runner: task check exit $status in $(( $(date +%s) - start )) s, export included"
exit "$status"
