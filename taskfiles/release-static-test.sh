#!/bin/sh
# The test of taskfiles/release-static.sh (story:release-linux-static-check): made ELF files, a
# stand-in for docker, a table of cases, and planted defects of the script.
#
# Usage: sh taskfiles/release-static-test.sh [<script>]
#
# It runs from the repository root and writes under .out/release-static-test/. It builds
# nothing, starts no container and makes no network request; it needs readelf.
#
# 1. The made files. Five small ELF64 files, little-endian, x86-64, no section headers, written
#    with printf octal escapes into made/:
#    - static: ET_EXEC, one PT_LOAD;
#    - interp: ET_EXEC, PT_INTERP naming /lib/ld-musl-x86_64.so.1 in the file, and PT_LOAD;
#    - needed: ET_DYN, a PT_LOAD over the whole file at p_vaddr = p_offset, and a PT_DYNAMIC
#      over DT_NEEDED, DT_STRTAB, DT_STRSZ, DT_NULL, followed by the string table;
#    - static-pie: ET_DYN, a PT_LOAD, and a PT_DYNAMIC that holds DT_NULL only;
#    - both: PT_INTERP and the dynamic part of needed.
#    And text, a file that is no ELF file.
# 2. The guard. readelf -lW and readelf -d on each made file: each exits 0 and prints no
#    warning or error, an INTERP row only for interp and both, a (NEEDED) entry only for needed
#    and both, a dynamic section only for needed, static-pie and both. A made file that reads
#    otherwise stops the test, so no case passes for the wrong reason.
# 3. The table. 18 cases, each on a directory
#    case<n>/surf-0.0.0-x86_64-unknown-linux-musl/ that holds made files under the four names
#    of a Linux archive. Each case holds an exit status, the text the output holds (and one it
#    does not hold where the case says so) and, where an image is given, whether the stand-in
#    docker was called and the arguments it was called with. The stand-in records its arguments
#    and prints the first line and exits with the status the case sets.
# 4. Planted defects, when no <script> is given: twelve copies of taskfiles/release-static.sh,
#    each made with sed from one rule's line; an edit that changes nothing fails the test. The
#    table runs on each copy, and each case the defect names must be decided otherwise.
#
# <script> names the script under test (default taskfiles/release-static.sh), so that a copy
# can run through the table; with one given, the planted defects are not made. It prints one
# line per case decided otherwise and exits 1, or one line with the count and exits 0.

set -u

[ "$#" -le 1 ] || {
  printf '%s\n' 'usage: sh taskfiles/release-static-test.sh [<script>]' >&2
  exit 2
}
given=$#
script=${1:-taskfiles/release-static.sh}
[ -f "$script" ] || {
  printf '%s\n' "release-static-test: $script: missing" >&2
  exit 1
}
command -v readelf > /dev/null 2>&1 || {
  printf '%s\n' 'release-static-test: readelf is not on PATH' >&2
  exit 1
}

out=.out/release-static-test
rm -rf "$out"
mkdir -p "$out/made" || exit 1
out=$(cd "$out" && pwd -P) || exit 1
made=$out/made
archive_name=surf-0.0.0-x86_64-unknown-linux-musl

# 1. The made files. bytes holds printf octal escapes; le <n> <value> appends <value> as <n>
# little-endian bytes, str <text> appends text and its terminating NUL.
le() {
  n=$1
  v=$2
  while [ "$n" -gt 0 ]; do
    b=$((v % 256))
    bytes=$bytes\\$((b / 64))$((b / 8 % 8))$((b % 8))
    v=$((v / 256))
    n=$((n - 1))
  done
}
str() {
  bytes=$bytes$1'\000'
}
pad() {
  le "$1" 0
}
# header <e_type> <e_entry> <e_phnum>
header() {
  bytes='\177ELF\002\001\001\000\000\000\000\000\000\000\000\000'
  le 2 "$1"
  le 2 62
  le 4 1
  le 8 "$2"
  le 8 64
  le 8 0
  le 4 0
  le 2 64
  le 2 56
  le 2 "$3"
  le 2 64
  le 2 0
  le 2 0
}
# phdr <p_type> <p_flags> <p_offset> <p_vaddr> <p_filesz> <p_align>; p_paddr is p_vaddr and
# p_memsz is p_filesz.
phdr() {
  le 4 "$1"
  le 4 "$2"
  le 8 "$3"
  le 8 "$4"
  le 8 "$4"
  le 8 "$5"
  le 8 "$5"
  le 8 "$6"
}
# dyn <d_tag> <d_val>
dyn() {
  le 8 "$1"
  le 8 "$2"
}
write() {
  printf "$bytes" > "$made/$1" || exit 1
}

PT_LOAD=1
PT_DYNAMIC=2
PT_INTERP=3
ET_EXEC=2
ET_DYN=3
DT_NULL=0
DT_NEEDED=1
DT_STRTAB=5
DT_STRSZ=10
base=4194304
interp_path=/lib/ld-musl-x86_64.so.1

# static: header 0-63, LOAD 64-119; 120 bytes.
header "$ET_EXEC" $((base + 120)) 1
phdr "$PT_LOAD" 5 0 "$base" 120 4096
write static

# interp: header, INTERP 64-119, LOAD 120-175, the path at 176 (25 bytes); 201 bytes.
header "$ET_EXEC" $((base + 176)) 2
phdr "$PT_INTERP" 4 176 $((base + 176)) 25 1
phdr "$PT_LOAD" 5 0 "$base" 201 4096
str "$interp_path"
write interp

# needed: header, LOAD 64-119, DYNAMIC 120-175, dynamic entries 176-239, the string table at
# 240: NUL, libc.so, NUL (9 bytes); 249 bytes.
header "$ET_DYN" 176 2
phdr "$PT_LOAD" 6 0 0 249 4096
phdr "$PT_DYNAMIC" 6 176 176 64 8
dyn "$DT_NEEDED" 1
dyn "$DT_STRTAB" 240
dyn "$DT_STRSZ" 9
dyn "$DT_NULL" 0
pad 1
str libc.so
write needed

# static-pie: header, LOAD 64-119, DYNAMIC 120-175, DT_NULL 176-191, 8 bytes of padding so that
# the dynamic segment does not end the file; 200 bytes.
header "$ET_DYN" 176 2
phdr "$PT_LOAD" 6 0 0 200 4096
phdr "$PT_DYNAMIC" 6 176 176 16 8
dyn "$DT_NULL" 0
pad 8
write static-pie

# both: header, INTERP 64-119, LOAD 120-175, DYNAMIC 176-231, the path at 232 (25 bytes), 7
# bytes of padding, dynamic entries 264-327, the string table at 328 (9 bytes); 337 bytes.
header "$ET_DYN" 264 3
phdr "$PT_INTERP" 4 232 232 25 1
phdr "$PT_LOAD" 6 0 0 337 4096
phdr "$PT_DYNAMIC" 6 264 264 64 8
str "$interp_path"
pad 7
dyn "$DT_NEEDED" 1
dyn "$DT_STRTAB" 328
dyn "$DT_STRSZ" 9
dyn "$DT_NULL" 0
pad 1
str libc.so
write both

printf '%s\n' 'not an ELF file' > "$made/text" || exit 1

# 2. The guard.
guard() {
  name=$1
  want_interp=$2
  want_needed=$3
  want_dynamic=$4
  file=$made/$name
  status=0
  lw=$(LC_ALL=C readelf -lW "$file" 2>&1) || status=$?
  d=$(LC_ALL=C readelf -d "$file" 2>&1) || status=$?
  why=
  [ "$status" -eq 0 ] || why="readelf exited $status"
  if printf '%s\n%s\n' "$lw" "$d" | grep -q -i -E 'warning|error'; then
    why="readelf printed a warning or an error"
  fi
  has=no
  if printf '%s\n' "$lw" | grep -q '^ *INTERP'; then has=yes; fi
  [ "$has" = "$want_interp" ] || why="an INTERP row: $has, expected $want_interp"
  has=no
  if printf '%s\n' "$d" | grep -q -F '(NEEDED)'; then has=yes; fi
  [ "$has" = "$want_needed" ] || why="a (NEEDED) entry: $has, expected $want_needed"
  has=no
  if printf '%s\n' "$d" | grep -q '^Dynamic section at offset'; then has=yes; fi
  [ "$has" = "$want_dynamic" ] || why="a dynamic section: $has, expected $want_dynamic"
  [ -z "$why" ] && return 0
  printf '%s\n' "release-static-test: the made file $name does not read as named: $why" >&2
  printf '%s\n' "readelf -lW:" "$lw" "readelf -d:" "$d" >&2
  exit 1
}
guard static no no no
guard interp yes no no
guard needed no yes yes
guard static-pie no no yes
guard both yes yes yes
printf '%s\n' 'release-static-test: the made files read as named: static, interp, needed, static-pie, both'

# 3. The table. The stand-in docker records its arguments, one per line, in STANDIN_RECORD,
# prints STANDIN_FIRST and the last line of the definition, and exits STANDIN_STATUS.
standin=$out/docker
printf '%s\n' '#!/bin/sh' \
  'printf "%s\n" "$@" > "$STANDIN_RECORD"' \
  '[ -z "$STANDIN_ERR" ] || printf "%s\n" "$STANDIN_ERR" >&2' \
  'printf "%s\n" "$STANDIN_FIRST" "end of the definition"' \
  'exit "$STANDIN_STATUS"' > "$standin" || exit 1
chmod 755 "$standin" || exit 1
image=alpine@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
root=$(pwd -P)

# case_start <n> <name>: the case's directory, its record and its log; the stand-in as docker,
# printing the version of the directory's name and exiting 0.
case_start() {
  n=$1
  name=$2
  dir=$out/case$n/$archive_name
  record=$logs/case$n.args
  log=$logs/case$n.log
  docker=$standin
  first='surf-agent 0.0.0: a definition for agents'
  standin_status=0
  standin_err=
  red=
  rm -f "$record"
}
# fill <surf> <surf-agent> <surfd> <surf-session>: the made file of each name, - for none.
# The directory is made once per test run; every table run reads it.
fill() {
  [ -d "$dir" ] && return 0
  mkdir -p "$dir" || exit 1
  for binary in surf surf-agent surfd surf-session; do
    if [ "$1" != - ]; then
      cp "$made/$1" "$dir/$binary" || exit 1
    fi
    shift
  done
}
run() {
  status=0
  DOCKER=$docker STANDIN_FIRST=$first STANDIN_STATUS=$standin_status STANDIN_RECORD=$record \
    STANDIN_ERR=$standin_err sh "$under_test" "$@" > "$log" 2>&1 || status=$?
}
note() {
  red=yes
  if [ "$verbose" = yes ]; then
    printf '%s\n' "release-static-test: $n. $name: $1 ($log)"
  fi
}
expect_status() {
  [ "$status" -eq "$1" ] || note "exit $status, expected $1"
}
# expect_line <line>: a whole line of the output.
expect_line() {
  grep -q -x -F -e "$1" "$log" || note "the output holds no line: $1"
}
expect_text() {
  grep -q -F -e "$1" "$log" || note "the output does not hold: $1"
}
expect_no_text() {
  if grep -q -F -e "$1" "$log"; then note "the output holds: $1"; fi
}
expect_not_called() {
  if [ -e "$record" ]; then note "the stand-in docker was called"; fi
}
# expect_args <absolute dir>: the arguments the stand-in was called with.
expect_args() {
  if [ ! -f "$record" ]; then
    note "the stand-in docker was not called"
    return
  fi
  args=" $(tr '\n' ' ' < "$record")"
  case $args in
    ' run '*) ;;
    *) note "the stand-in's first argument is not run:$args" ;;
  esac
  for want in ' --rm ' ' --network none ' " --volume $1:/archive:ro " \
    " $image /archive/surf-agent help --for-agent "; do
    case $args in
      *"$want"*) ;;
      *) note "the stand-in's arguments do not hold '$want':$args" ;;
    esac
  done
}
case_end() {
  cases=$((cases + 1))
  if [ -n "$red" ]; then
    red_cases="$red_cases $n"
  fi
}
static_line() {
  printf '%s' "release-static: $dir holds 4 static binaries: surf surf-agent surfd surf-session"
}

# table <script> <log dir> <verbose>: runs the 16 cases on <script>; red_cases lists the cases
# decided otherwise.
table() {
  under_test=$1
  logs=$2
  verbose=$3
  cases=0
  red_cases=
  mkdir -p "$logs" || exit 1

  case_start 1 'four static files'
  fill static static static static
  run "$dir"
  expect_status 0
  expect_line "$(static_line)"
  case_end

  case_start 2 'four static-pie files'
  fill static-pie static-pie static-pie static-pie
  run "$dir"
  expect_status 0
  expect_line "$(static_line)"
  case_end

  case_start 3 'surf-session is interp'
  fill static static static interp
  run "$dir"
  expect_status 1
  expect_line "release-static: $dir/surf-session: INTERP header (dynamically linked)"
  expect_no_text 'static binaries'
  case_end

  case_start 4 'surfd is needed'
  fill static static needed static
  run "$dir"
  expect_status 1
  expect_line "release-static: $dir/surfd: NEEDED entry (dynamically linked)"
  expect_no_text 'static binaries'
  case_end

  case_start 5 'surf-agent is a text file'
  fill static text static static
  run "$dir"
  expect_status 1
  expect_line "release-static: $dir/surf-agent: not an ELF file readelf can read"
  case_end

  case_start 6 'no surf in the directory'
  fill - static static static
  run "$dir"
  expect_status 1
  expect_line "release-static: $dir/surf: not an ELF file readelf can read"
  case_end

  case_start 7 'surf and surfd are both dynamic'
  fill interp static needed static
  run "$dir"
  expect_status 1
  expect_line "release-static: $dir/surf: INTERP header (dynamically linked)"
  expect_line "release-static: $dir/surfd: NEEDED entry (dynamically linked)"
  case_end

  case_start 8 'an image given as a tag only'
  fill static static static static
  run "$dir" --image alpine:3.24.4
  expect_status 2
  expect_not_called
  case_end

  case_start 9 'an image by digest, the stand-in prints the version of the directory'
  fill static static static static
  run "$dir" --image "$image"
  expect_status 0
  expect_line "$(static_line)"
  expect_line "release-static: surf-agent help --for-agent ran in $image: surf-agent 0.0.0"
  expect_args "$dir"
  case_end

  case_start 10 'an image by digest, the stand-in prints another version'
  fill static static static static
  first='surf-agent 0.0.1: a definition for agents'
  run "$dir" --image "$image"
  expect_status 1
  expect_no_text 'ran in'
  expect_args "$dir"
  case_end

  case_start 11 'an image by digest, the stand-in prints the right line and exits 125'
  fill static static static static
  standin_status=125
  run "$dir" --image "$image"
  expect_status 1
  expect_no_text 'ran in'
  expect_args "$dir"
  case_end

  case_start 12 'DOCKER names no program'
  fill static static static static
  docker=$out/no-such-docker
  run "$dir" --image "$image"
  expect_status 1
  expect_text "$docker"
  expect_no_text 'ran in'
  case_end

  case_start 13 'surf-session is interp and an image is given'
  fill static static static interp
  run "$dir" --image "$image"
  expect_status 1
  expect_line "release-static: $dir/surf-session: INTERP header (dynamically linked)"
  expect_not_called
  case_end

  case_start 14 'an unknown option'
  fill static static static static
  run "$dir" --unknown
  expect_status 2
  expect_no_text 'static binaries'
  case_end

  case_start 15 'a relative directory and an image by digest'
  fill static static static static
  run "${dir#"$root"/}" --image "$image"
  expect_status 0
  expect_line "$(static_line)"
  expect_args "$dir"
  case_end

  case_start 16 'surf is both'
  fill both static static static
  run "$dir"
  expect_status 1
  expect_line "release-static: $dir/surf: INTERP header (dynamically linked)"
  expect_no_text 'NEEDED'
  case_end

  # The workflow passes --image "$STATIC_IMAGE"; an empty value is no image named by digest,
  # and the check must not end green without the container.
  case_start 17 'an empty image value'
  fill static static static static
  run "$dir" --image ''
  expect_status 2
  expect_not_called
  expect_no_text 'static binaries'
  case_end

  # docker writes "Unable to find image" and the lines of the pull on stderr when the image is
  # not on the runner yet; the first line of stdout is still the definition's.
  case_start 18 'the stand-in writes a line on stderr before the definition'
  fill static static static static
  standin_err="Unable to find image '$image' locally"
  run "$dir" --image "$image"
  expect_status 0
  expect_line "release-static: surf-agent help --for-agent ran in $image: surf-agent 0.0.0"
  expect_args "$dir"
  case_end
}

table "$script" "$out/table" yes
table_cases=$cases
if [ -n "$red_cases" ]; then
  set -- $red_cases
  printf '%s\n' "release-static-test: $# of $table_cases case(s) decided otherwise by $script"
  exit 1
fi

# 4. Planted defects. defect <label> <sed program> <cases>: a copy of the script edited by the
# program; each of <cases> must be decided otherwise on the copy.
defects=0
wrong=0
defect() {
  defects=$((defects + 1))
  copy=$out/defect$defects/release-static.sh
  mkdir -p "$out/defect$defects" || exit 1
  sed "$2" "$script" > "$copy" || exit 1
  if cmp -s "$script" "$copy"; then
    printf '%s\n' "release-static-test: planted defect $defects ($1): the edit finds no line of $script"
    wrong=$((wrong + 1))
    return
  fi
  table "$copy" "$out/defect$defects" no
  green=
  for c in $3; do
    case " $red_cases " in
      *" $c "*) ;;
      *) green="$green $c" ;;
    esac
  done
  if [ -n "$green" ]; then
    printf '%s\n' "release-static-test: planted defect $defects ($1): case(s)$green" \
      "  decided as expected, named to turn red; red:${red_cases:- none} ($copy)"
    wrong=$((wrong + 1))
    return
  fi
  # A copy that no longer runs (an edit that left a broken if or an open quote) turns every case
  # red, the named ones too; case 1, four static files and no image, holds that the copy runs.
  case " $red_cases " in
    *' 1 '*)
      printf '%s\n' "release-static-test: planted defect $defects ($1): case 1 decided otherwise" \
        "  too: the copy does not check four static files; red:$red_cases ($copy)"
      wrong=$((wrong + 1))
      return
      ;;
  esac
  printf '%s\n' "release-static-test: planted defect $defects ($1): case(s)$red_cases decided otherwise"
}
if [ "$given" -eq 0 ]; then
  defect 'INTERP test removed' '/^  if .*INTERP header/d' '3 13 16'
  defect 'NEEDED test removed' '/^  if .*NEEDED entry/d' '4'
  defect 'status of readelf not read' '/not an ELF file readelf can read"; return 1/d' '5 6'
  defect 'the loop stops at the first bad binary' 's/|| bad=1$/|| { bad=1; break; }/' '7'
  defect 'digest form not required' '/|| is_digest_ref/d' '8'
  defect 'container run before the static check' \
    's/^bad=0$/[ -z "$image" ] || run_container || exit 1; bad=0/' '13'
  defect 'status of docker not read' '/exited \$status, not 0"; return 1/d' '11'
  defect 'version not compared' '/^  \[ "\$first" = /d' '10'
  defect 'an absent docker skips the container' \
    's/the container check needs it"; return 1;/the container check needs it"; return 0;/' '12'
  defect 'an unknown option accepted' 's/^    \*) usage "unknown option: \$1" ;;$/    *) shift ;;/' '14'
  defect '--network none left out' '/^  out=/s/ --network none//' '9'
  defect '<dir> not resolved before the mount' 's/^dir=\$(CDPATH=.*$/dir=$arg_dir/' '15'
  if [ "$wrong" -ne 0 ]; then
    printf '%s\n' "release-static-test: $wrong of $defects planted defect(s) not caught as named"
    exit 1
  fi
  printf '%s\n' "release-static-test: $defects planted defect(s), each caught by the cases it names"
fi
printf '%s\n' "release-static-test: $table_cases case(s), each decided as expected by $script"
