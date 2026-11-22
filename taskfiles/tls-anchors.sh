#!/bin/sh
# usage: tls-anchors.sh refresh <anchors dir>
#        tls-anchors.sh age <anchors dir>
#
# The two tasks of the bundled trust anchors of surf-tls (story tls-anchors). POSIX sh only:
# the forge's sh is dash and a workstation's may be bash, so the script uses no `local`, no
# arrays, no `[[`, and writes data with printf '%s\n', never with echo. The anchors directory
# is crates/tls/src/anchors; it holds cacert.pem, curl's extract of the root store of Mozilla,
# and SOURCE, the record of it (docs/design/tls.md, item 11).
#
# refresh: takes a new list and writes <dir>/cacert.pem and <dir>/SOURCE. With FILE and SHA256
# (paths of local files: the list and its published .sha256) it takes those and reaches no
# network; without both it fetches https://curl.se/ca/cacert.pem and its .sha256 with curl into
# <dir>, under names starting with a dot that it removes. The .sha256 comes from the same
# server as the list, so it proves an uncut download only; the person who refreshes reads the
# titles it prints as added and removed. It refuses, changing nothing:
# - a target (cacert.pem, SOURCE, or one of its files starting with a dot) that exists and is
#   not a regular file, or a directory it cannot write;
# - a list whose sha256 differs from the first field of the .sha256 file (that file's first
#   line is `<64 lower-case hex digits>  <name>`);
# - a list without exactly one header line `## Certificate data from Mozilla as of: <weekday>
#   <month> <day> <hh:mm:ss> <year> GMT`, written as curl writes it (Mon to Sun, Jan to Dec, the
#   day padded to two places with a space, a time of 00:00:00 to 23:59:59, a year of 1000 to
#   9999);
# - a list with no block, or with a block that is not whole: every line
#   `-----BEGIN CERTIFICATE-----` followed by its `-----END CERTIFICATE-----` before the next
#   BEGIN line and before the end;
# - a list that removes more titles than ANCHORS_REMOVED_MAX (default 10). A title is the line
#   above a line of `=`, as curl writes one above each certificate. Before it writes, refresh
#   prints `anchors-refresh: added <title>` and `anchors-refresh: removed <title>`, one line
#   per title that the new list has more or fewer times than the old.
# The date of the list is that of the header line. On success it prints `anchors-refresh: <n>
# certificates of <yyyy-mm-dd>, sha256 <hex>`. It is never a step of check, check-runner or a
# workflow: only it reaches the network.
#
# age: reads `date:` and `count:` of <dir>/SOURCE, prints `anchors: <n> certificates of <date>,
# <d> days old`, the days counted to DATE (yyyy-mm-dd, default today in UTC), and exits 1 when
# <d> is over 90, naming it. No network.
#
# A refusal is one line on standard error and exit 1: a missing or wrong argument, a path with
# a line feed, a directory or file that is missing, an input not in its form. The standard error
# of the commands it runs does not reach the caller; every command that fails ends the script
# with a refusal.

set -eu
set -f
LC_ALL=C
export LC_ALL

URL=https://curl.se/ca/cacert.pem
SHA256_URL=https://curl.se/ca/cacert.pem.sha256
PAGE=https://curl.se/docs/caextract.html
LICENCE='MPL-2.0, as the page says: "The PEM file is only a converted version of the original one and thus it is licensed under the same license as the Mozilla source file: MPL 2.0"'
AGE_MAX=90
REMOVED_MAX_DEFAULT=10
USAGE='usage: tls-anchors.sh refresh|age <anchors dir>'
NL='
'

fail() {
  printf '%s\n' "$1" >&2
  exit 1
}

say() {
  printf '%s\n' "$1"
}

# Refuses a path that holds a line feed: no refusal could name it in one line.
plain_path() {
  case $1 in
    *"$NL"*) fail "tls-anchors: $2 holds a line feed" ;;
    '') fail "tls-anchors: $2 is empty" ;;
  esac
}

# Refuses $1 when it exists and is not a regular file (a symbolic link counts as not one).
regular_or_absent() {
  if [ -L "$1" ] || { [ -e "$1" ] && [ ! -f "$1" ]; }; then
    fail "tls-anchors: $1 exists and is not a regular file"
  fi
}

# The value of the one line `<key>: <value>` of file $2.
field() {
  value=$(sed -n "s/^$1: //p" < "$2" 2>/dev/null) || fail "tls-anchors: cannot read $2"
  case $value in
    '') fail "tls-anchors: $2 has no line $1:" ;;
    *"$NL"*) fail "tls-anchors: $2 has more than one line $1:" ;;
  esac
  say "$value"
}

# $1 without its leading zeros ("0" for a run of zeros), so that shell arithmetic never reads
# it as octal.
decimal() {
  digits=${1#"${1%%[!0]*}"}
  say "${digits:-0}"
}

# Sets y, m and d from the date $1 (yyyy-mm-dd), checked, as decimal numbers; $2 names it.
split_date() {
  case $1 in
    [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]) ;;
    *) fail "tls-anchors: $2 $1 is not yyyy-mm-dd" ;;
  esac
  y=$(decimal "${1%%-*}")
  m=${1#*-}
  m=$(decimal "${m%-*}")
  d=$(decimal "${1##*-}")
  [ "$y" -ge 1000 ] || fail "tls-anchors: $2 $1 has no year of four digits from 1000"
  [ "$m" -ge 1 ] && [ "$m" -le 12 ] || fail "tls-anchors: $2 $1 has no month $m"
  case $m in
    1 | 3 | 5 | 7 | 8 | 10 | 12) last=31 ;;
    4 | 6 | 9 | 11) last=30 ;;
    *)
      if [ $((y % 4)) -eq 0 ] && { [ $((y % 100)) -ne 0 ] || [ $((y % 400)) -eq 0 ]; }; then
        last=29
      else
        last=28
      fi
      ;;
  esac
  [ "$d" -ge 1 ] && [ "$d" -le "$last" ] || fail "tls-anchors: $2 $1 has no day $d"
}

# Sets days to the number of days from 0000-03-01 to y-m-d (the civil calendar).
day_number() {
  dy=$y
  if [ "$m" -le 2 ]; then
    dy=$((dy - 1))
    mp=$((m + 9))
  else
    mp=$((m - 3))
  fi
  era=$((dy / 400))
  yoe=$((dy - era * 400))
  doy=$(((153 * mp + 2) / 5 + d - 1))
  days=$((era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy))
}

# The titles of the list $1, sorted, one per line: each line above a line of `=`.
titles() {
  awk 'prev != "" && /^=+$/ { print prev } { prev = $0 }' "$1" 2>/dev/null | sort 2>/dev/null
}

[ $# -ge 1 ] || fail "tls-anchors: no task; $USAGE"
task=$1
case $task in
  refresh | age) ;;
  *) fail "tls-anchors: no task $task; $USAGE" ;;
esac
[ $# -eq 2 ] || fail "tls-anchors: $task takes one directory; $USAGE"
dir=$2
plain_path "$dir" "the anchors directory"
[ -d "$dir" ] || fail "tls-anchors: $dir is not a directory"

if [ "$task" = age ]; then
  source=$dir/SOURCE
  [ -f "$source" ] && [ -r "$source" ] || fail "tls-anchors: $source is missing or unreadable"
  list_date=$(field date "$source")
  count=$(field count "$source")
  case $count in
    *[!0-9]*) fail "tls-anchors: $source has a count that is not a number: $count" ;;
  esac
  today=${DATE:-}
  if [ -z "$today" ]; then
    today=$(date -u +%Y-%m-%d 2>/dev/null) || fail "tls-anchors: date failed"
  fi
  split_date "$list_date" "the list date of $source"
  day_number
  from=$days
  split_date "$today" DATE
  day_number
  age=$((days - from))
  [ "$age" -ge 0 ] || fail "tls-anchors: DATE $today is before the list date $list_date"
  say "anchors: $count certificates of $list_date, $age days old"
  if [ "$age" -gt "$AGE_MAX" ]; then
    fail "anchors: $age days is older than $AGE_MAX days; run task anchors-refresh before a release"
  fi
  exit 0
fi

# refresh
file=${FILE:-}
sums=${SHA256:-}
removed_max=${ANCHORS_REMOVED_MAX:-$REMOVED_MAX_DEFAULT}
case $removed_max in
  '' | *[!0-9]*) fail "tls-anchors: ANCHORS_REMOVED_MAX $removed_max is not a number" ;;
esac
removed_max=$(decimal "$removed_max")
new_pem=$dir/.cacert.pem.new
new_source=$dir/.SOURCE.new
download=$dir/.cacert.pem.download
download_sums=$dir/.cacert.pem.sha256.download
for target in "$dir/cacert.pem" "$dir/SOURCE" "$new_pem" "$new_source" "$download" "$download_sums"; do
  regular_or_absent "$target"
done
[ -w "$dir" ] || fail "tls-anchors: cannot write the directory $dir"
trap 'rm -f "$new_pem" "$new_source" "$download" "$download_sums" 2>/dev/null' EXIT
trap 'exit 1' HUP INT TERM

if [ -n "$file" ] || [ -n "$sums" ]; then
  [ -n "$file" ] || fail "tls-anchors: SHA256 is given without FILE; give both or neither"
  [ -n "$sums" ] || fail "tls-anchors: FILE is given without SHA256; give both or neither"
  plain_path "$file" FILE
  plain_path "$sums" SHA256
  [ -f "$file" ] && [ -r "$file" ] || fail "tls-anchors: FILE $file is not a readable file"
  [ -f "$sums" ] && [ -r "$sums" ] || fail "tls-anchors: SHA256 $sums is not a readable file"
else
  curl -fsS -o "$download" "$URL" 2>/dev/null || fail "tls-anchors: curl could not fetch $URL"
  curl -fsS -o "$download_sums" "$SHA256_URL" 2>/dev/null \
    || fail "tls-anchors: curl could not fetch $SHA256_URL"
  file=$download
  sums=$download_sums
fi

# Everything below reads the copy that is written, never the file again.
cp "$file" "$new_pem" 2>/dev/null || fail "tls-anchors: cannot copy $file into $dir"

line=
{ IFS= read -r line < "$sums"; } 2>/dev/null || [ -n "$line" ] || fail "tls-anchors: $sums is empty"
expected=${line%%  *}
case $expected in
  *[!0-9a-f]* | '') fail "tls-anchors: $sums does not start with a sha256 and two spaces" ;;
esac
[ "${#expected}" -eq 64 ] && [ "$line" != "$expected" ] \
  || fail "tls-anchors: $sums does not start with a sha256 and two spaces"
sum=$(sha256sum < "$new_pem" 2>/dev/null) || fail "tls-anchors: sha256sum failed"
sum=${sum%% *}
[ "$sum" = "$expected" ] || fail "tls-anchors: the sha256 of $file is $sum, $sums says $expected; nothing changed"

prefix='## Certificate data from Mozilla as of: '
header=$(sed -n "s/^## \(Certificate data from Mozilla as of: .*\)$/\1/p" < "$new_pem" 2>/dev/null) \
  || fail "tls-anchors: cannot read $file"
case $header in
  '') fail "tls-anchors: $file has no line \"$prefix...\"" ;;
  *"$NL"*) fail "tls-anchors: $file has more than one line \"$prefix...\"" ;;
esac
# The fields of the header's date: weekday, month, day, time, year, GMT.
when=${header#Certificate data from Mozilla as of: }
set -- $when
not_curls="tls-anchors: the header line of $file is not in curl's form"
[ $# -eq 6 ] && [ "$6" = GMT ] || fail "$not_curls"
case $1 in
  Mon | Tue | Wed | Thu | Fri | Sat | Sun) ;;
  *) fail "$not_curls: no weekday" ;;
esac
case $2 in
  Jan) month=01 ;; Feb) month=02 ;; Mar) month=03 ;; Apr) month=04 ;;
  May) month=05 ;; Jun) month=06 ;; Jul) month=07 ;; Aug) month=08 ;;
  Sep) month=09 ;; Oct) month=10 ;; Nov) month=11 ;; Dec) month=12 ;;
  *) fail "$not_curls: no month" ;;
esac
case $3 in
  [1-9]) day=0$3 ;;
  [1-3][0-9]) day=$3 ;;
  *) fail "$not_curls: no day" ;;
esac
case $4 in
  [01][0-9]:[0-5][0-9]:[0-5][0-9] | 2[0-3]:[0-5][0-9]:[0-5][0-9]) ;;
  *) fail "$not_curls: no time" ;;
esac
# curl writes the date as asctime does: single spaces, the day padded to two places.
canonical=$(printf '%s %s %2s %s %s GMT' "$1" "$2" "$3" "$4" "$5")
[ "$when" = "$canonical" ] || fail "$not_curls: spacing"
list_date=$5-$month-$day
split_date "$list_date" "the header date of $file"

# The blocks: whole, in order; their count.
count=$(awk '
  /^-----BEGIN CERTIFICATE-----$/ { if (open) bad = 1; open = 1; n++; next }
  /^-----END CERTIFICATE-----$/ { if (!open) bad = 1; open = 0; next }
  END { if (open || bad) exit 1; print n + 0 }
' "$new_pem" 2>/dev/null) || fail "tls-anchors: $file holds a block that is not whole"
[ "$count" -gt 0 ] || fail "tls-anchors: $file holds no -----BEGIN CERTIFICATE----- line"
bytes=$(wc -c < "$new_pem" 2>/dev/null) || fail "tls-anchors: wc failed"
bytes=$((bytes + 0))
fetched=$(date -u +%Y-%m-%d 2>/dev/null) || fail "tls-anchors: date failed"

# The titles added and removed, against the list in place (none when there is none).
old_titles=
if [ -f "$dir/cacert.pem" ]; then
  old_titles=$(titles "$dir/cacert.pem") || fail "tls-anchors: cannot read the titles of $dir/cacert.pem"
fi
new_titles=$(titles "$new_pem") || fail "tls-anchors: cannot read the titles of $file"
printf '%s\n' "$old_titles" | sed '/^$/d' > "$new_source" 2>/dev/null \
  || fail "tls-anchors: cannot write $new_source"
added=$(printf '%s\n' "$new_titles" | sed '/^$/d' | comm -13 "$new_source" - 2>/dev/null) \
  || fail "tls-anchors: comm failed"
removed=$(printf '%s\n' "$new_titles" | sed '/^$/d' | comm -23 "$new_source" - 2>/dev/null) \
  || fail "tls-anchors: comm failed"
removed_count=0
if [ -n "$added" ]; then
  printf '%s\n' "$added" | sed 's/^/anchors-refresh: added /'
fi
if [ -n "$removed" ]; then
  printf '%s\n' "$removed" | sed 's/^/anchors-refresh: removed /'
  removed_count=$(printf '%s\n' "$removed" | wc -l)
  removed_count=$((removed_count + 0))
fi
[ "$removed_count" -le "$removed_max" ] \
  || fail "tls-anchors: the list removes $removed_count titles, more than $removed_max; nothing changed (ANCHORS_REMOVED_MAX=<n> allows more)"

{
  say 'The file cacert.pem beside this one is copied unchanged from the extract of the root store of'
  say 'Mozilla that curl publishes (docs/design/tls.md, item 11). The task anchors-refresh writes both'
  say 'files (taskfiles/tls-anchors.sh) and no one edits them by hand. The .sha256 it checks comes'
  say 'from the same server as the list and proves an uncut download only, so the person who'
  say 'refreshes reads the titles the task prints as added and removed.'
  say ''
  say "url: $URL"
  say "sha256-url: $SHA256_URL"
  say "page: $PAGE"
  say "header: $header"
  say "date: $list_date"
  say "fetched: $fetched"
  say "bytes: $bytes"
  say "count: $count"
  say "sha256: $sum"
  say "licence: $LICENCE"
} 2>/dev/null > "$new_source" || fail "tls-anchors: cannot write $new_source"

mv -f "$new_pem" "$dir/cacert.pem" 2>/dev/null || fail "tls-anchors: cannot write $dir/cacert.pem"
mv -f "$new_source" "$dir/SOURCE" 2>/dev/null \
  || fail "tls-anchors: cannot write $dir/SOURCE; cacert.pem is new"
say "anchors-refresh: $count certificates of $list_date, sha256 $sum"
