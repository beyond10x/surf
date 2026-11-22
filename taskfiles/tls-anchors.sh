#!/bin/sh
# usage: tls-anchors.sh refresh <anchors dir>
#        tls-anchors.sh age <anchors dir>
#
# The two tasks of the bundled trust anchors of surf-tls (story tls-anchors), POSIX sh. The
# anchors directory is crates/tls/src/anchors; it holds cacert.pem, curl's extract of the root
# store of Mozilla, and SOURCE, the record of it (docs/design/tls.md, item 11).
#
# refresh: takes a new list and writes <dir>/cacert.pem and <dir>/SOURCE. With FILE and SHA256
# (paths of local files: the list and its published .sha256) it takes those and reaches no
# network; without both it fetches https://curl.se/ca/cacert.pem and its .sha256 with curl into
# <dir>, under names starting with a dot that it removes. It refuses, changing nothing, when
# the sha256 of the list differs from the first field of the .sha256 file (that file's first
# line is `<64 lower-case hex digits>  <name>`), when the list has not exactly one header line
# `## Certificate data from Mozilla as of: <weekday> <month> <day> <hh:mm:ss> <year> GMT`, or
# when it holds no line `-----BEGIN CERTIFICATE-----`. The date of the list is that of the header
# line. On success it prints `anchors-refresh: <n> certificates of <yyyy-mm-dd>, sha256 <hex>`.
# It is never a step of check, check-runner or a workflow: only it reaches the network.
#
# age: reads `date:` and `count:` of <dir>/SOURCE, prints `anchors: <n> certificates of <date>,
# <d> days old`, the days counted to DATE (yyyy-mm-dd, default today in UTC), and exits 1 when
# <d> is over 90, naming it. No network.
#
# A refusal is one line on standard error and exit 1: a missing or wrong argument, a path with
# a line feed, a directory or file that is missing, an input not in its form. Every command that
# fails ends the script with a refusal.

set -eu
set -f
LC_ALL=C
export LC_ALL

URL=https://curl.se/ca/cacert.pem
SHA256_URL=https://curl.se/ca/cacert.pem.sha256
PAGE=https://curl.se/docs/caextract.html
LICENCE='MPL-2.0, as the page says: "The PEM file is only a converted version of the original one and thus it is licensed under the same license as the Mozilla source file: MPL 2.0"'
AGE_MAX=90
USAGE='usage: tls-anchors.sh refresh|age <anchors dir>'
NL='
'

fail() {
  printf '%s\n' "$1" >&2
  exit 1
}

# Refuses a path that holds a line feed: no refusal could name it in one line.
plain_path() {
  case $1 in
    *"$NL"*) fail "tls-anchors: $2 holds a line feed" ;;
    '') fail "tls-anchors: $2 is empty" ;;
  esac
}

# The value of the one line `<key>: <value>` of file $2.
field() {
  value=$(sed -n "s/^$1: //p" < "$2") || fail "tls-anchors: cannot read $2"
  case $value in
    '') fail "tls-anchors: $2 has no line $1:" ;;
    *"$NL"*) fail "tls-anchors: $2 has more than one line $1:" ;;
  esac
  printf '%s\n' "$value"
}

# Sets y, m and d from the date $1 (yyyy-mm-dd), checked, without leading zeros; $2 names it.
split_date() {
  case $1 in
    [1-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]) ;;
    *) fail "tls-anchors: $2 $1 is not yyyy-mm-dd" ;;
  esac
  y=${1%%-*}
  m=${1#*-}
  m=${m%-*}
  m=${m#0}
  d=${1##*-}
  d=${d#0}
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
  [ -f "$source" ] || fail "tls-anchors: $source is missing"
  list_date=$(field date "$source")
  count=$(field count "$source")
  case $count in
    *[!0-9]*) fail "tls-anchors: $source has a count that is not a number: $count" ;;
  esac
  today=${DATE:-}
  if [ -z "$today" ]; then
    today=$(date -u +%Y-%m-%d) || fail "tls-anchors: date failed"
  fi
  split_date "$list_date" "the list date of $source"
  day_number
  from=$days
  split_date "$today" DATE
  day_number
  age=$((days - from))
  [ "$age" -ge 0 ] || fail "tls-anchors: DATE $today is before the list date $list_date"
  echo "anchors: $count certificates of $list_date, $age days old"
  if [ "$age" -gt "$AGE_MAX" ]; then
    fail "anchors: $age days is older than $AGE_MAX days; run task anchors-refresh before a release"
  fi
  exit 0
fi

# refresh
file=${FILE:-}
sums=${SHA256:-}
new_pem=$dir/.cacert.pem.new
new_source=$dir/.SOURCE.new
download=$dir/.cacert.pem.download
download_sums=$dir/.cacert.pem.sha256.download
trap 'rm -f "$new_pem" "$new_source" "$download" "$download_sums"' EXIT
trap 'exit 1' HUP INT TERM

if [ -n "$file" ] || [ -n "$sums" ]; then
  [ -n "$file" ] || fail "tls-anchors: SHA256 is given without FILE; give both or neither"
  [ -n "$sums" ] || fail "tls-anchors: FILE is given without SHA256; give both or neither"
  plain_path "$file" FILE
  plain_path "$sums" SHA256
  [ -f "$file" ] || fail "tls-anchors: FILE $file is not a file"
  [ -f "$sums" ] || fail "tls-anchors: SHA256 $sums is not a file"
else
  curl -fsS -o "$download" "$URL" || fail "tls-anchors: curl could not fetch $URL"
  curl -fsS -o "$download_sums" "$SHA256_URL" || fail "tls-anchors: curl could not fetch $SHA256_URL"
  file=$download
  sums=$download_sums
fi

# Everything below reads the copy that is written, never the file again.
cp "$file" "$new_pem" || fail "tls-anchors: cannot copy $file"

line=
IFS= read -r line < "$sums" || [ -n "$line" ] || fail "tls-anchors: $sums is empty"
expected=${line%%  *}
case $expected in
  *[!0-9a-f]* | '') fail "tls-anchors: $sums does not start with a sha256 and two spaces" ;;
esac
[ "${#expected}" -eq 64 ] && [ "$line" != "$expected" ] \
  || fail "tls-anchors: $sums does not start with a sha256 and two spaces"
sum=$(sha256sum < "$new_pem") || fail "tls-anchors: sha256sum failed"
sum=${sum%% *}
[ "$sum" = "$expected" ] || fail "tls-anchors: the sha256 of $file is $sum, $sums says $expected; nothing changed"

prefix='## Certificate data from Mozilla as of: '
header=$(sed -n "s/^## \(Certificate data from Mozilla as of: .*\)$/\1/p" < "$new_pem") \
  || fail "tls-anchors: cannot read $file"
case $header in
  '') fail "tls-anchors: $file has no line \"$prefix...\"" ;;
  *"$NL"*) fail "tls-anchors: $file has more than one line \"$prefix...\"" ;;
esac
# The fields of the header's date: weekday, month, day, time, year, GMT.
set -- ${header#Certificate data from Mozilla as of: }
[ $# -eq 6 ] && [ "$6" = GMT ] || fail "tls-anchors: the header line of $file is not in curl's form: $header"
case $2 in
  Jan) month=01 ;; Feb) month=02 ;; Mar) month=03 ;; Apr) month=04 ;;
  May) month=05 ;; Jun) month=06 ;; Jul) month=07 ;; Aug) month=08 ;;
  Sep) month=09 ;; Oct) month=10 ;; Nov) month=11 ;; Dec) month=12 ;;
  *) fail "tls-anchors: the header line of $file names no month: $header" ;;
esac
case $3 in
  [1-9]) day=0$3 ;;
  [1-3][0-9]) day=$3 ;;
  *) fail "tls-anchors: the header line of $file names no day: $header" ;;
esac
list_date=$5-$month-$day
split_date "$list_date" "the header date of $file"

if count=$(grep -c '^-----BEGIN CERTIFICATE-----$' < "$new_pem"); then
  :
else
  [ $? -eq 1 ] || fail "tls-anchors: grep failed on $file"
fi
[ "$count" -gt 0 ] || fail "tls-anchors: $file holds no -----BEGIN CERTIFICATE----- line"
bytes=$(wc -c < "$new_pem") || fail "tls-anchors: wc failed"
bytes=$((bytes + 0))
fetched=$(date -u +%Y-%m-%d) || fail "tls-anchors: date failed"

{
  echo 'The file cacert.pem beside this one is copied unchanged from the extract of the root store of'
  echo 'Mozilla that curl publishes (docs/design/tls.md, item 11). The task anchors-refresh writes both'
  echo 'files (taskfiles/tls-anchors.sh) and no one edits them by hand.'
  echo
  echo "url: $URL"
  echo "sha256-url: $SHA256_URL"
  echo "page: $PAGE"
  echo "header: $header"
  echo "date: $list_date"
  echo "fetched: $fetched"
  echo "bytes: $bytes"
  echo "count: $count"
  echo "sha256: $sum"
  echo "licence: $LICENCE"
} > "$new_source" || fail "tls-anchors: cannot write $new_source"

mv -f "$new_pem" "$dir/cacert.pem" || fail "tls-anchors: cannot write $dir/cacert.pem"
mv -f "$new_source" "$dir/SOURCE" || fail "tls-anchors: cannot write $dir/SOURCE; cacert.pem is new"
echo "anchors-refresh: $count certificates of $list_date, sha256 $sum"
