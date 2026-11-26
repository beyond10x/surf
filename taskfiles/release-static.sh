#!/bin/sh
# The static check of one unpacked Linux release archive (docs/releasing.md). The release
# workflow runs it on each Linux archive before the upload, and it runs unchanged on a
# workstation:
#
#   sh taskfiles/release-static.sh <dir> [--image <image>@sha256:<digest>]
#
# <dir> is the unpacked directory `surf-<version>-<target>/`; it is resolved to an absolute path
# before it is used. A host with an older glibc than the build's cannot run a binary linked
# against glibc, so each Linux archive holds static binaries.
#
# 1. For each of `surf`, `surf-agent`, `surfd` and `surf-session`, in this order and all four
#    before it exits: `readelf -lW` lists no program header row of type INTERP and `readelf -d`
#    lists no (NEEDED) entry. A binary with no dynamic section passes, and so does a static-pie
#    binary, whose dynamic section holds no NEEDED entry. A non-zero status of readelf (a file
#    that is no ELF file, a missing file) fails the binary; a binary with both an INTERP row and
#    a NEEDED entry gets the INTERP line only. No ldd, which can run the binary through the
#    host's loader. It prints one line per failed binary,
#      release-static: <dir>/<name>: INTERP header (dynamically linked)
#      release-static: <dir>/<name>: NEEDED entry (dynamically linked)
#      release-static: <dir>/<name>: not an ELF file readelf can read
#    and exits 1, or prints
#      release-static: <dir> holds 4 static binaries: surf surf-agent surfd surf-session
# 2. With --image, and only when all four are static: `$DOCKER run --rm --network none --volume
#    <dir>:/archive:ro <image> /archive/surf-agent help --for-agent` (DOCKER defaults to
#    docker) must exit 0 and print first `surf-agent <version>: a definition for agents`, the
#    <version> of the directory's name `surf-<version>-<arch>-unknown-linux-<abi>`; it prints
#      release-static: surf-agent help --for-agent ran in <image>: surf-agent <version>
#    The image is named by digest: an image that does not end in @sha256: and 64 hex digits, an
#    empty value included, is a usage error, refused before docker starts; --image given never
#    skips the container. Alpine has musl, so the container alone does not
#    prove a binary static (one linked dynamically against musl runs there); the readelf check
#    and the container together are the check. docker absent or failing fails the check.
#
# Exit 0 when the check holds, 1 when it does not, 2 for a usage error.

set -u

names='surf surf-agent surfd surf-session'
docker=${DOCKER:-docker}

say() {
  printf 'release-static: %s\n' "$*" >&2
}
usage() {
  say "$1"
  printf '%s\n' 'usage: sh taskfiles/release-static.sh <dir> [--image <image>@sha256:<digest>]' >&2
  exit 2
}
is_digest_ref() {
  h='[0-9a-f]'
  h8=$h$h$h$h$h$h$h$h
  case $1 in
    *@sha256:$h8$h8$h8$h8$h8$h8$h8$h8) return 0 ;;
    *) return 1 ;;
  esac
}

[ "$#" -ge 1 ] || usage 'no directory given'
arg_dir=$1
shift
image=
image_given=no
while [ "$#" -gt 0 ]; do
  case $1 in
    --image)
      [ "$#" -ge 2 ] || usage '--image needs a value'
      image=$2
      image_given=yes
      shift 2
      ;;
    *) usage "unknown option: $1" ;;
  esac
done
[ "$image_given" = no ] || is_digest_ref "$image" || usage "--image '$image' is not named by digest (<image>@sha256:<64 hex digits>)"
[ -d "$arg_dir" ] || usage "$arg_dir is not a directory"
dir=$(CDPATH='' cd -- "$arg_dir" && pwd -P) || exit 1
command -v readelf > /dev/null 2>&1 || {
  say 'readelf is not on PATH'
  exit 1
}

# check_binary <path>: 0 when the binary is static, else 1 after its line.
check_binary() {
  path=$1
  status=0
  program_headers=$(LC_ALL=C readelf -lW "$path" 2>&1) || status=$?
  dynamic_section=$(LC_ALL=C readelf -d "$path" 2>&1) || status=$?
  [ "$status" -eq 0 ] || { say "$path: not an ELF file readelf can read"; return 1; }
  if printf '%s\n' "$program_headers" | grep -q '^ *INTERP'; then say "$path: INTERP header (dynamically linked)"; return 1; fi
  if printf '%s\n' "$dynamic_section" | grep -q -F '(NEEDED)'; then say "$path: NEEDED entry (dynamically linked)"; return 1; fi
  return 0
}

# run_container: surf-agent help --for-agent in the image, the directory mounted read-only.
run_container() {
  version=$(basename "$dir" | sed -n 's/^surf-\(.*\)-[a-z0-9_]*-unknown-linux-[a-z]*$/\1/p')
  [ -n "$version" ] || { say "$(basename "$dir") is not named surf-<version>-<arch>-unknown-linux-<abi>"; return 1; }
  command -v "$docker" > /dev/null 2>&1 || { say "$docker: no such program; the container check needs it"; return 1; }
  status=0
  out=$("$docker" run --rm --network none --volume "$dir:/archive:ro" "$image" /archive/surf-agent help --for-agent) || status=$?
  [ "$status" -eq 0 ] || { say "$docker run $image /archive/surf-agent help --for-agent exited $status, not 0"; return 1; }
  first=$(printf '%s\n' "$out" | sed -n 1p)
  want="surf-agent $version: a definition for agents"
  [ "$first" = "$want" ] || { say "surf-agent help --for-agent printed the first line: $first (expected $want)"; return 1; }
  printf 'release-static: surf-agent help --for-agent ran in %s: surf-agent %s\n' "$image" "$version"
}

bad=0
for name in $names; do
  check_binary "$dir/$name" || bad=1
done
[ "$bad" -eq 0 ] || exit 1
printf 'release-static: %s holds 4 static binaries: %s\n' "$dir" "$names"
[ "$image_given" = no ] || run_container || exit 1
exit 0
