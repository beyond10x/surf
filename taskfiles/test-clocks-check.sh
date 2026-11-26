#!/bin/sh
# Fail when a test asserts an upper bound on an elapsed time that the ledger does not list: a
# test proves a cost by a count and bounds a time only by a wait with a limit (AGENTS.md, the
# entry of task test).
#
# Usage: sh taskfiles/test-clocks-check.sh [--count] [--ledger <file>] [<root>]   (default: .)
#
# - <root> is the top of a git work tree: `git -C <root> rev-parse --show-toplevel` prints what
#   `cd <root> && pwd -P` prints. Else it prints "test-clocks-check: <root> is not the top of a
#   git work tree" and exits 2.
# - AWK names the awk (default awk; it may hold words, as in AWK="gawk --posix").
# - --ledger <file> reads that ledger in place of <root>/taskfiles/test-clocks-ledger.txt;
#   --ledger /dev/null is an empty ledger. --count prints the ledger's lines for the tree,
#   "<path> <count>", one per file that holds a refused statement, sorted.
#
# POSIX awk only, run under LC_ALL=C; the forge's awk is mawk. Not used, by name: nextfile (not
# in every mawk), length(array) (gawk accepts it in every mode), IGNORECASE, gensub, asort,
# asorti, strftime, the word boundaries \< and \> and a regular expression interval {n,m} .
# gawk --posix ends on gensub, asort and strftime (undefined there) and reads \< as a literal <;
# it and the other awks accept nextfile, length(array), IGNORECASE (a plain variable) and
# intervals, so those are kept out by these rules: every array is named with the prefix arr_,
# and length( is written only on a string. A comment is a line of its own that starts with #.
#
# Reading: the files `git ls-files` lists under the patterns crates/**/tests/**/*.rs,
# generated/**/tests/**/*.rs and conformance/**/tests/**/*.rs, so an untracked file, a target
# directory and a dot directory git does not track are not read. A file that holds neither
# "elapsed(" nor "duration_since(" holds no elapsed value and is not read further. Each file is
# read with its comments, strings, raw strings and character literals blanked first, one pass,
# line by line, the state kept across lines: // to the line feed; /* */ nested; "..." with a
# backslash hiding the next character; r, br or cr not after a letter, a digit or _, then n
# times # and ", to the first " followed by n times #, no escape inside; a character literal is
# ' then a backslash, the character it hides and the text to the next ', or ' then one
# character (an ASCII byte or a UTF-8 sequence of 2 to 4 bytes) then '. Any other ' is a
# lifetime or a label and stays code. Line feeds stay, so a line of the blanked text is the
# file's.
#
# Refused, each a statement, named "<path>:<line>" at its first token:
# - an assert!, assert_eq!, assert_ne!, debug_assert!, debug_assert_eq! or debug_assert_ne!
#   whose arguments, read to the closing bracket over any number of lines, hold a comparison
#   with an elapsed value as its smaller operand: left of < or <=, right of > or >=. Every
#   comparison of the arguments is read. Also inside a poll loop: the scan cannot know the
#   limit of the wait.
# - an if or while whose condition, read from the keyword to the { of its block over any
#   number of lines, compares upward: an elapsed value as the larger operand (left of > or >=,
#   right of < or <=) against an operand that holds Duration, a number or an upper-case
#   constant; and whose block, to its matching }, holds panic!, unreachable! or a word that
#   starts with assert or debug_assert.
# An elapsed value is an operand that is exactly a call of .elapsed(), .duration_since(..) or
# .saturating_duration_since(..) with the method chain around it (.as_secs_f64(), .expect(..)),
# or exactly a tracked name with a method chain. A name is tracked from its let to the end of
# the file when the value of the let, read from the keyword to the first ; outside brackets,
# holds such a call (round 1), a name of round 1 (round 2) or a name of round 2 (round 3); in a
# tuple pattern only the names that hold took, elapsed, seconds, secs, waited, duration or time
# are tracked. A name after . is a field of another value, not the tracked name.
#
# Allowed: a lower bound (took >= LIMIT, LIMIT <= took); a print; a wait with a limit, the
# spelling to use: a loop that leaves on its deadline and asserts the event after the loop,
#   while !done() { if started.elapsed() > LIMIT { break; } sleep(STEP); }  assert!(done());
# or a recv_timeout.
#
# Not seen: a bound written through a helper function, a macro, or a bool of a let
# (let ok = took < LIMIT; assert!(ok)); a name bound through a name of round 3; the negated
# spelling assert!(!(took >= LIMIT)); assert_eq!(took, ..) by equality; a bound recorded in a
# list asserted after the loop; a test that returns Err from the block; a failure by .expect,
# .unwrap or std::process::exit in the block; a difference of two instants written with -; and
# whether the limit of a wait is at least 10 times the time alone.
#
# Ledger: one "<path> <count>" per line, sorted, no comment. A file with more refused
# statements than its line (no line: 0) fails, naming each statement; a file with fewer fails
# and names the count to write; a line whose file the scan did not read fails: delete the line.
# Else it prints "test-clocks-check: no elapsed-time bound outside
# taskfiles/test-clocks-ledger.txt" and exits 0. It calls no cargo.

set -u

usage='usage: sh taskfiles/test-clocks-check.sh [--count] [--ledger <file>] [<root>]'
count=0
ledger=
while [ "$#" -gt 0 ]; do
  case $1 in
    --count)
      count=1
      shift
      ;;
    --ledger)
      [ "$#" -ge 2 ] || {
        echo "$usage" >&2
        exit 2
      }
      ledger=$2
      shift 2
      ;;
    -*)
      echo "$usage" >&2
      exit 2
      ;;
    *)
      break
      ;;
  esac
done
[ "$#" -le 1 ] || {
  echo "$usage" >&2
  exit 2
}
root=${1:-.}

here=$(cd "$root" 2>/dev/null && pwd -P) || here=
top=$(git -C "$root" rev-parse --show-toplevel 2>/dev/null) || top=
if [ -z "$here" ] || [ "$top" != "$here" ]; then
  printf '%s\n' "test-clocks-check: $root is not the top of a git work tree"
  exit 2
fi

if [ -z "$ledger" ]; then
  ledger=$here/taskfiles/test-clocks-ledger.txt
  shown=taskfiles/test-clocks-ledger.txt
else
  shown=$ledger
  case $ledger in
    /*) ;;
    *) ledger=$PWD/$ledger ;;
  esac
fi
if [ "$count" -eq 1 ]; then
  ledger=/dev/null
elif [ ! -r "$ledger" ]; then
  printf '%s\n' "test-clocks-check: cannot read the ledger $shown"
  exit 2
fi

cd "$here" || exit 2
files=$(git -c core.quotePath=false ls-files -- ':(glob)crates/**/tests/**/*.rs' \
  ':(glob)generated/**/tests/**/*.rs' ':(glob)conformance/**/tests/**/*.rs') || {
  printf '%s\n' "test-clocks-check: git ls-files failed in $root"
  exit 2
}

status=0
# shellcheck disable=SC2086
printf '%s\n' "$files" | LC_ALL=C ${AWK:-awk} -v count="$count" -v ledger="$ledger" \
  -v shown="$shown" -v q="'" '
function spc(n) {
  while (length(spaces) < n) spaces = spaces spaces
  return substr(spaces, 1, n)
}
function ident(t) {
  return t ~ /^[A-Za-z_][A-Za-z0-9_]*$/
}
# The length of the character literal that starts at p of s, or 0 for a lifetime or a label.
function charlit(s, p,   c, k, n, b) {
  c = substr(s, p + 1, 1)
  if (c == "" || c == q) return 0
  if (c == "\\") {
    k = index(substr(s, p + 3), q)
    if (k == 0) return 0
    return k + 3
  }
  n = 1
  while (n < 4) {
    b = substr(s, p + 1 + n, 1)
    if (b == "" || index(cont, b) == 0) break
    n++
  }
  if (substr(s, p + 1 + n, 1) == q) return n + 2
  return 0
}
# arr_raw[1..n] into arr_code[1..n]: comments, strings, raw strings and literals as spaces.
function blank(n,   i, s, len, out, pos, rest, p, c, k, pre, m, t, state, depth, hashes) {
  state = 0
  depth = 0
  hashes = ""
  for (i = 1; i <= n; i++) {
    s = arr_raw[i]
    len = length(s)
    out = ""
    pos = 1
    while (pos <= len) {
      rest = substr(s, pos)
      if (state == 1) {
        if (match(rest, /\/[*]|[*]\//)) {
          out = out spc(RSTART + 1)
          if (substr(rest, RSTART, 1) == "/") depth++
          else depth--
          if (depth == 0) state = 0
          pos += RSTART + 1
        } else {
          out = out spc(length(rest))
          pos = len + 1
        }
        continue
      }
      if (state == 2) {
        if (match(rest, /^([^"\\]|\\.)*"/)) {
          out = out spc(RLENGTH)
          pos += RLENGTH
          state = 0
        } else {
          out = out spc(length(rest))
          pos = len + 1
        }
        continue
      }
      if (state == 3) {
        t = "\"" hashes
        k = index(rest, t)
        if (k) {
          out = out spc(k + length(t) - 1)
          pos += k + length(t) - 1
          state = 0
        } else {
          out = out spc(length(rest))
          pos = len + 1
        }
        continue
      }
      if (!match(rest, code_re)) {
        out = out rest
        pos = len + 1
        continue
      }
      p = RSTART
      c = substr(rest, p, 1)
      pre = substr(rest, 1, p - 1)
      if (c == "/") {
        if (substr(rest, p + 1, 1) == "/") {
          out = out pre spc(length(rest) - p + 1)
          pos = len + 1
        } else {
          out = out pre "  "
          pos += p + 1
          state = 1
          depth = 1
        }
        continue
      }
      if (c == "\"") {
        if (match(pre, /[bc]?r#*$/) && (RSTART == 1 || substr(pre, RSTART - 1, 1) !~ /^[A-Za-z0-9_]$/)) {
          m = RSTART
          t = substr(pre, m)
          gsub(/[^#]/, "", t)
          hashes = t
          out = out substr(pre, 1, m - 1) spc(p - m + 1)
          pos += p
          state = 3
          continue
        }
        out = out pre " "
        pos += p
        state = 2
        continue
      }
      k = charlit(rest, p)
      if (k) {
        out = out pre spc(k)
        pos += p - 1 + k
      } else {
        out = out pre q
        pos += p
      }
    }
    arr_code[i] = out
  }
}
function tokenize(n,   i, s) {
  ntok = 0
  for (i = 1; i <= n; i++) {
    s = arr_code[i]
    while (match(s, /[A-Za-z_][A-Za-z0-9_]*|[0-9][A-Za-z0-9_]*|<=|>=|==|!=|&&|[|][|]|->|=>|::|<<|>>|[.][.]=|[.][.]|[^ \t\r]/)) {
      ntok++
      arr_tok[ntok] = substr(s, RSTART, RLENGTH)
      arr_line[ntok] = i
      s = substr(s, RSTART + RLENGTH)
    }
  }
  arr_tok[ntok + 1] = ""
  arr_tok[0] = ""
}
function pair(   i, t, sp, o) {
  sp = 0
  for (i = 1; i <= ntok; i++) {
    arr_pair[i] = 0
    t = arr_tok[i]
    if (t == "(" || t == "[" || t == "{") {
      sp++
      arr_stack[sp] = i
    } else if (t == ")" || t == "]" || t == "}") {
      if (sp == 0) continue
      o = arr_tok[arr_stack[sp]]
      if ((o == "(" && t == ")") || (o == "[" && t == "]") || (o == "{" && t == "}")) {
        arr_pair[arr_stack[sp]] = i
        arr_pair[i] = arr_stack[sp]
        sp--
      }
    }
  }
}
# Whether the tokens from..to hold a call of a clock method (round 1) or a name of a round
# before round r, bound before the let at token at.
function holds(from, to, r, at,   j, t) {
  for (j = from; j <= to; j++) {
    t = arr_tok[j]
    if (r == 1) {
      if (t == "." && (arr_tok[j + 1] in arr_clock) && arr_tok[j + 2] == "(") return 1
    } else if ((t in arr_rank) && arr_rank[t] < r && arr_since[t] < at && arr_tok[j - 1] != "." && arr_tok[j + 1] != "::" && arr_tok[j + 1] != ":") {
      return 1
    }
  }
  return 0
}
function worded(t,   k) {
  for (k = 1; k <= nwords; k++)
    if (index(t, arr_words[k])) return 1
  return 0
}
function track(   i, j, t, eq, k, m, names, nlet, l, r, n) {
  nlet = 0
  split("", arr_rank)
  split("", arr_since)
  for (i = 1; i <= ntok; i++) {
    if (arr_tok[i] != "let") continue
    j = i + 1
    eq = 0
    while (j <= ntok) {
      t = arr_tok[j]
      if (t == ";" || t == ")" || t == "]" || t == "}") break
      if (t == "(" || t == "[" || t == "{") {
        if (!arr_pair[j]) break
        j = arr_pair[j] + 1
        continue
      }
      if (t == "=" && !eq) eq = j
      j++
    }
    if (!eq) continue
    k = i + 1
    if (arr_tok[k] == "mut") k++
    names = ""
    if (arr_tok[k] == "(" && arr_pair[k]) {
      for (m = k + 1; m < arr_pair[k]; m++)
        if (ident(arr_tok[m]) && worded(arr_tok[m])) names = names " " arr_tok[m]
    } else if (ident(arr_tok[k]) && arr_tok[k] != "_") {
      names = " " arr_tok[k]
    }
    if (names == "") continue
    nlet++
    arr_let_at[nlet] = i
    arr_let_from[nlet] = eq + 1
    arr_let_to[nlet] = j - 1
    arr_let_names[nlet] = names
  }
  for (r = 1; r <= 3; r++) {
    for (l = 1; l <= nlet; l++) {
      if (!holds(arr_let_from[l], arr_let_to[l], r, arr_let_at[l])) continue
      n = split(arr_let_names[l], arr_names, " ")
      for (k = 1; k <= n; k++) {
        if (arr_names[k] in arr_rank) continue
        arr_rank[arr_names[k]] = r
        arr_since[arr_names[k]] = arr_let_at[l]
      }
    }
  }
}
function stop(t) {
  return (t in arr_stop) || (t in arr_stopword)
}
# The first token of the operand left of the operator at k, not before lo.
function left_start(k, lo,   j, t) {
  j = k - 1
  while (j >= lo) {
    t = arr_tok[j]
    if (t == ")" || t == "]") {
      if (!arr_pair[j] || arr_pair[j] < lo) break
      j = arr_pair[j] - 1
      continue
    }
    if (t == "(" || t == "[" || t == "{" || t == "}" || stop(t)) break
    j--
  }
  return j + 1
}
# The last token of the operand right of the operator at k, not after hi.
function right_end(k, hi,   j, t) {
  j = k + 1
  while (j <= hi) {
    t = arr_tok[j]
    if (t == "(" || t == "[") {
      if (!arr_pair[j] || arr_pair[j] > hi) break
      j = arr_pair[j] + 1
      continue
    }
    if (t == ")" || t == "]" || t == "{" || t == "}" || stop(t)) break
    j++
  }
  return j - 1
}
# Whether the tokens a..b are exactly an elapsed value: a clock call or a tracked name, with a
# method chain.
function elapsed(a, b,   i, t, named, clock) {
  if (a > b || !ident(arr_tok[a])) return 0
  i = a
  t = arr_tok[i]
  named = 0
  clock = 0
  if ((t in arr_rank) && arr_since[t] < a && (i == b || (arr_tok[i + 1] != "::" && arr_tok[i + 1] != "(" && arr_tok[i + 1] != "!"))) {
    named = 1
    i++
  } else {
    i++
    while (i + 1 <= b && arr_tok[i] == "::" && ident(arr_tok[i + 1])) i += 2
    if (i <= b && arr_tok[i] == "(") {
      if (!arr_pair[i] || arr_pair[i] > b) return 0
      i = arr_pair[i] + 1
    }
  }
  while (i <= b) {
    t = arr_tok[i]
    if (t == "?") {
      i++
      continue
    }
    if (t == "[") {
      if (!arr_pair[i] || arr_pair[i] > b) return 0
      i = arr_pair[i] + 1
      continue
    }
    if (t != "." || i + 1 > b) return 0
    t = arr_tok[i + 1]
    if (t ~ /^[0-9]+$/) {
      i += 2
      continue
    }
    if (!ident(t)) return 0
    i += 2
    if (i <= b && arr_tok[i] == "(") {
      if (!arr_pair[i] || arr_pair[i] > b) return 0
      if (t in arr_clock) clock = 1
      i = arr_pair[i] + 1
    }
  }
  return named || clock
}
# Whether the tokens a..b hold Duration, a number or an upper-case constant.
function limit(a, b,   j, t) {
  for (j = a; j <= b; j++) {
    t = arr_tok[j]
    if (t == "Duration" || t ~ /^[0-9]/ || t ~ /^[A-Z][A-Z0-9_]*$/) return 1
  }
  return 0
}
# Whether the block between the braces o and c fails the test.
function fails(o, c,   j, t) {
  for (j = o + 1; j < c; j++) {
    t = arr_tok[j]
    if ((t == "panic" || t == "unreachable") && arr_tok[j + 1] == "!") return 1
    if (t ~ /^(debug_)?assert/) return 1
  }
  return 0
}
function statements(   i, t, o, c, k, a, b, j, brace, up, n) {
  n = 0
  for (i = 1; i <= ntok; i++) {
    t = arr_tok[i]
    if ((t in arr_assert) && arr_tok[i + 1] == "!") {
      o = i + 2
      t = arr_tok[o]
      if ((t != "(" && t != "[" && t != "{") || !arr_pair[o]) continue
      c = arr_pair[o]
      for (k = o + 1; k < c; k++) {
        t = arr_tok[k]
        if ((t == "<" || t == "<=") && elapsed(left_start(k, o + 1), k - 1)) break
        if ((t == ">" || t == ">=") && elapsed(k + 1, right_end(k, c - 1))) break
      }
      if (k < c) arr_found[++n] = arr_line[i]
      continue
    }
    if (t != "if" && t != "while") continue
    j = i + 1
    brace = 0
    while (j <= ntok) {
      t = arr_tok[j]
      if (t == "{") {
        brace = j
        break
      }
      if (t == "(" || t == "[") {
        if (!arr_pair[j]) break
        j = arr_pair[j] + 1
        continue
      }
      if (t == ";" || t == "=>" || t == ")" || t == "]" || t == "}") break
      j++
    }
    if (!brace || !arr_pair[brace]) continue
    up = 0
    for (k = i + 1; k < brace && !up; k++) {
      t = arr_tok[k]
      if (t == ">" || t == ">=") {
        a = left_start(k, i + 1)
        b = right_end(k, brace - 1)
        up = elapsed(a, k - 1) && limit(k + 1, b)
      } else if (t == "<" || t == "<=") {
        a = left_start(k, i + 1)
        b = right_end(k, brace - 1)
        up = elapsed(k + 1, b) && limit(a, k - 1)
      }
    }
    if (up && fails(brace, arr_pair[brace])) arr_found[++n] = arr_line[i]
  }
  return n
}
# The refused statements of the file path, their lines in arr_found[1..]; -1 when unreadable.
function scan(path,   n, line, got, has) {
  n = 0
  has = 0
  while ((got = (getline line < path)) > 0) {
    n++
    arr_raw[n] = line
    if (!has && (index(line, "elapsed(") || index(line, "duration_since("))) has = 1
  }
  close(path)
  if (got < 0) return -1
  if (!has) return 0
  blank(n)
  tokenize(n)
  pair()
  track()
  return statements()
}
BEGIN {
  nwords = split("took elapsed seconds secs waited duration time", arr_words, " ")
  split("assert assert_eq assert_ne debug_assert debug_assert_eq debug_assert_ne", arr_list, " ")
  for (x in arr_list) arr_assert[arr_list[x]] = 1
  split("elapsed duration_since saturating_duration_since", arr_list, " ")
  for (x in arr_list) arr_clock[arr_list[x]] = 1
  split("if while return let match in else break continue loop for move", arr_list, " ")
  for (x in arr_list) arr_stopword[arr_list[x]] = 1
  split(", ; = == != < <= > >= && || | => -> .. ..=", arr_list, " ")
  for (x in arr_list) arr_stop[arr_list[x]] = 1
  cont = ""
  for (x = 128; x < 192; x++) cont = cont sprintf("%c", x)
  spaces = " "
  code_re = "/[/*]|[\"" q "]"
  status = 0
  nledger = 0
  while ((entry_got = (getline entry < ledger)) > 0) {
    nledger++
    if (entry !~ /^[^ ]+ [1-9][0-9]*$/) {
      printf "test-clocks-check: %s:%d: \"%s\" is not \"<path> <count>\" with a count above 0\n", shown, nledger, entry
      status = 1
      continue
    }
    split(entry, arr_field, " ")
    if (arr_field[1] in arr_allowed) {
      printf "test-clocks-check: %s:%d: a second line for %s\n", shown, nledger, arr_field[1]
      status = 1
      continue
    }
    arr_allowed[arr_field[1]] = arr_field[2] + 0
    arr_ledger_path[nledger] = arr_field[1]
  }
  close(ledger)
  if (entry_got < 0) {
    printf "test-clocks-check: cannot read the ledger %s\n", shown
    status = 2
  }
}
$0 != "" && status != 2 {
  file = $0
  arr_read[file] = 1
  nfound = scan(file)
  if (nfound < 0) {
    printf "test-clocks-check: %s: cannot read it\n", file
    status = 1
    next
  }
  if (count) {
    if (nfound > 0) printf "%s %d\n", file, nfound
    next
  }
  allowed = (file in arr_allowed) ? arr_allowed[file] : 0
  if (nfound > allowed) {
    for (x = 1; x <= nfound; x++) printf "%s:%d\n", file, arr_found[x]
    printf "test-clocks-check: %s: %d elapsed-time bound(s), the ledger allows %d; prove the cost by a count, or wait in a loop that leaves on its deadline and assert after it\n", file, nfound, allowed
    status = 1
  } else if (nfound == 0 && allowed > 0) {
    printf "test-clocks-check: %s: no elapsed-time bound, the ledger allows %d; delete its line in %s\n", file, allowed, shown
    status = 1
  } else if (nfound < allowed) {
    printf "test-clocks-check: %s: %d elapsed-time bound(s), the ledger allows %d; write \"%s %d\" in %s\n", file, nfound, allowed, file, nfound, shown
    status = 1
  }
}
END {
  if (status == 2) exit 2
  for (x = 1; x <= nledger; x++) {
    if (!(x in arr_ledger_path) || (arr_ledger_path[x] in arr_read)) continue
    printf "test-clocks-check: %s:%d: %s names a file the check did not read (not tracked, renamed, deleted or outside the path patterns); delete the line\n", shown, x, arr_ledger_path[x]
    status = 1
  }
  exit status
}
' || status=$?

case $status in
  0) ;;
  1) exit 1 ;;
  *)
    printf '%s\n' "test-clocks-check: ${AWK:-awk} ended with exit $status"
    exit 2
    ;;
esac
if [ "$count" -eq 0 ]; then
  echo 'test-clocks-check: no elapsed-time bound outside taskfiles/test-clocks-ledger.txt'
fi
