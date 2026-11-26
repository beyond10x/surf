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
# directory and a dot directory git does not track are not read. A file none of whose lines
# holds "elapsed" or "duration_since", and that does not hold both the word now and "Instant" or
# "SystemTime" (each on any of its lines), holds no elapsed value and no clock read and is not
# read further; a space or a line feed before the ( of a call does not hide one. Each
# file is read with its comments, strings, raw strings and character literals blanked first,
# one pass, line by line, the state kept across lines: // to the line feed; /* */ nested; "..."
# with a backslash hiding the next character; r, br or cr not after a letter, a digit or _,
# then n times # and ", to the first " followed by n times #, no escape inside; a character
# literal is ' then a backslash, the character it hides and the text to the next ', or ' then
# one character (an ASCII byte or a UTF-8 sequence of 2 to 4 bytes) then '. Any other ' is a
# lifetime or a label and stays code. Line feeds stay, so a line of the blanked text is the
# file's.
#
# Refused, each a statement, named "<path>:<line>" at its first token:
# - an assert!, assert_eq!, assert_ne!, debug_assert!, debug_assert_eq! or debug_assert_ne!
#   whose arguments, read to the closing bracket over any number of lines, hold a comparison
#   with an elapsed value or a clock read as its smaller operand, or with the time left as its
#   larger operand. A comparison is x < y, x <= y, y > x, y >= x, x.lt(y), x.le(y), y.gt(x) or
#   y.ge(x), x the smaller operand; every comparison of the arguments is read. Also inside a
#   poll loop: the scan cannot know the limit of the wait, and
#   assert!(Instant::now() < deadline) is the same assertion as assert!(s.elapsed() < WAIT).
# - an if or while whose condition, read from the keyword to the { of its block over any
#   number of lines, compares upward: an elapsed value or a clock read as the larger operand
#   against an operand that is not of its kind (a constant, a lower-case name, a call), or the
#   time left as the smaller operand against one that is not the time left; and whose block, to
#   its matching }, holds panic!, unreachable! or a word that starts with assert or
#   debug_assert.
# - an if whose condition compares downward (the other way round) and whose else block, to its
#   matching }, fails the test by the same words.
# - a match guard, pattern if condition =>, whose condition compares upward and whose arm, its
#   block or its expression to the next , fails the test by the same words.
# A clock read compared with an instant the product holds (a certificate's not_after, a cookie's
# expiry) is refused like a deadline, though it bounds no wait of the test.
# An operand is read without a leading * or & and without parentheses around the whole, and a
# cast after it (as u64) is part of it. A clock read is an operand that is exactly a path ending
# in Instant::now() or SystemTime::now(). An elapsed value is an operand that is exactly a call
# of .elapsed(), .duration_since(..), .saturating_duration_since(..) or
# .checked_duration_since(..) with the method chain around it (.as_secs_f64(), .expect(..)),
# or exactly a tracked name with a method chain; the time left is such a call that the next
# sentence calls so. a.duration_since(b) and its forms saturating_ and checked_ are: no elapsed
# value when b is a path ending in UNIX_EPOCH (a timestamp); an elapsed value when a is a clock
# read or a name bound to one; the time left when b is a clock read; no elapsed value when a is
# a fixed instant and b is a fixed instant or a name bound to a clock read; else an elapsed
# value (a side that is a parameter, a field or an expression may hold a clock read). A name
# bound to a clock read is one name bound by let or = to exactly a clock read; a fixed instant
# is a path ending in an upper-case constant, or one name bound by let or = from a value that
# holds + or - and a clock read or a name bound to one (a deadline, s + WAIT).
# A name is tracked from its binding to the end of the file when the value of the binding holds
# an elapsed value (round 1), a name of round 1 (round 2) or a name of round 2 (round 3). The
# bindings: a let, its value read from the keyword to the first ; outside brackets; an if let or
# while let, its value to the { of the block; a for, its pattern from the keyword to the in, its
# value from the in to the { of the block; NAME = and NAME += at the start of a statement,
# their value to the first ; or , outside brackets; the pattern of each arm of a match, its
# value what the match reads. A pattern binds its lower-case names, not the names of a variant,
# a struct, a path or a field in it (Some(took) binds took); of a pattern that binds several,
# only the names that hold took, elapsed, seconds, secs, waited, duration or time are tracked.
# A name after . is a field of another value, not the tracked name.
#
# Allowed: a lower bound (took >= LIMIT, LIMIT <= took); a print; a wait with a limit, the
# spelling to use: a loop that leaves on its deadline and asserts the event after the loop,
#   while !done() { if started.elapsed() > LIMIT { break; } sleep(STEP); }  assert!(done());
# or a loop head under the deadline (while Instant::now() < deadline), or a recv_timeout; an
# upper bound on the time left (deadline.saturating_duration_since(Instant::now()) < WAIT).
#
# Not seen: a bound written through a helper function, a macro, or a bool of a let
# (let ok = took < LIMIT; assert!(ok)); a name bound through a name of round 3; the negated
# spelling assert!(!(took >= LIMIT)); assert_eq!(took, ..) by equality; a bound recorded in a
# list asserted after the loop; a test that returns Err from the block; a failure by .expect,
# .unwrap or std::process::exit in the block; a difference of two instants written with -; a
# clock read through an alias (use Instant as Clock; Clock::now()) or through a name
# (let now = Instant::now(); assert!(now < deadline)); a name bound from the time left; a
# failing else if branch; a match guard that compares downward with a failing arm after it;
# an operand that is an expression around an elapsed value (took + SLACK, took / ROUNDS); a
# comparison through cmp, partial_cmp, min or max; a name bound by a destructuring assignment,
# a value assigned through a reference or to a field; the time left whose argument is a name
# or a parameter (deadline.saturating_duration_since(now)), and with a parameter on either side
# it is read as an elapsed value; a bound between two Unix timestamps; and whether the limit of
# a wait is at least 10 times the time alone.
#
# Ledger: one "<path> <count>" per line, sorted by path (LC_ALL=C), no comment. A line out of
# order fails, naming the first such line. A file with more refused statements than its line
# (no line: 0) fails, naming each statement; a file with fewer fails and names the count to
# write; a line whose file the scan did not read fails: delete the line. Else it prints
# "test-clocks-check: no elapsed-time bound outside taskfiles/test-clocks-ledger.txt" and exits
# 0. It calls no cargo.

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
      if (t == "." && (arr_tok[j + 1] in arr_clock) && arr_tok[j + 2] == "(" && callkind(j, from) == 1) return 1
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
# The names a pattern a..b binds: an identifier that starts with a lower-case letter or _, not
# after :: or ., and inside the pattern not before (, {, ::, ! or : (a variant, a struct, a
# path, a macro, a field). One name is tracked; of several, only those that hold a word of
# arr_words (took, elapsed, seconds, ...).
function pnames(a, b,   m, t, u, n, all, wn) {
  n = 0
  all = ""
  wn = ""
  for (m = a; m <= b; m++) {
    t = arr_tok[m]
    if (!ident(t) || t == "_" || (t in arr_notname) || t ~ /^[A-Z0-9]/) continue
    if (arr_tok[m - 1] == "::" || arr_tok[m - 1] == ".") continue
    u = arr_tok[m + 1]
    if (m < b && (u == "(" || u == "{" || u == "::" || u == "!" || u == ":")) continue
    n++
    all = all " " t
    if (worded(t)) wn = wn " " t
  }
  return n == 1 ? all : wn
}
# The last token of the pattern of a let that starts at k: before a : (its type) or the = at eq
# outside brackets.
function pattern_end(k, eq,   j, t) {
  j = k
  while (j < eq) {
    t = arr_tok[j]
    if ((t == "(" || t == "[" || t == "{") && arr_pair[j] && arr_pair[j] < eq) {
      j = arr_pair[j] + 1
      continue
    }
    if (t == ":") return j - 1
    j++
  }
  return eq - 1
}
# One binding: the names, the value from..to, bound at token at. One name bound to exactly a
# clock read goes into arr_clockat, one bound from a value that holds + or - and a clock read or
# a name of arr_clockat into arr_fixedat (each at its first binding).
function add(at, from, to, names,   nm, j, t, sum, clk) {
  if (names == "") return
  nm = substr(names, 2)
  if (!index(nm, " ")) {
    if (clockish(from, to, 1)) {
      if (!(nm in arr_clockat)) arr_clockat[nm] = at
    } else {
      sum = 0
      clk = 0
      for (j = from; j <= to; j++) {
        t = arr_tok[j]
        if (t == "+" || t == "-") sum = 1
        if ((t == "now" && arr_tok[j - 1] == "::" && (arr_tok[j - 2] in arr_clocktype)) || ((t in arr_clockat) && arr_clockat[t] < at && arr_tok[j - 1] != ".")) clk = 1
      }
      if (sum && clk && !(nm in arr_fixedat)) arr_fixedat[nm] = at
    }
  }
  nlet++
  arr_let_at[nlet] = at
  arr_let_from[nlet] = from
  arr_let_to[nlet] = to
  arr_let_names[nlet] = names
}
# The arms of the match at i: the names of the pattern of each arm (to its guard or =>) are bound
# from the value the match reads.
function arms(i,   j, t, brace, shut, p, q, g) {
  j = i + 1
  while (j <= ntok) {
    t = arr_tok[j]
    if (t == "{") break
    if (t == "(" || t == "[") {
      if (!arr_pair[j]) return
      j = arr_pair[j] + 1
      continue
    }
    if (t == ";" || t == ")" || t == "]" || t == "}") return
    j++
  }
  if (j > ntok || !arr_pair[j]) return
  brace = j
  shut = arr_pair[j]
  p = brace + 1
  while (p < shut) {
    q = p
    g = 0
    while (q < shut && arr_tok[q] != "=>") {
      t = arr_tok[q]
      if ((t == "(" || t == "[" || t == "{") && arr_pair[q]) {
        q = arr_pair[q] + 1
        continue
      }
      if (t == "if" && !g) g = q
      q++
    }
    if (q >= shut) return
    add(p, i + 1, brace - 1, pnames(p, (g ? g : q) - 1))
    p = arm_end(q + 1) + 1
    if (arr_tok[p] == ",") p++
  }
}
# The bindings of a file: a let (its value to the first ; outside brackets), an if let or while
# let (its value to the { of the block), a for (its pattern to the in, its value to the { of the
# block), an assignment NAME = and NAME += at the start of a statement (its value to the first ;
# or , outside brackets); then the names of each round.
function track(   i, j, t, eq, k, cond, nm, l, r, n) {
  nlet = 0
  split("", arr_rank)
  split("", arr_since)
  split("", arr_clockat)
  split("", arr_fixedat)
  for (i = 1; i <= ntok; i++) {
    t = arr_tok[i]
    if (t == "let") {
      t = arr_tok[i - 1]
      cond = (t == "if" || t == "while" || t == "&&")
      j = i + 1
      eq = 0
      while (j <= ntok) {
        t = arr_tok[j]
        if (t == ";" || t == ")" || t == "]" || t == "}") break
        if (cond && eq && (t == "{" || t == "&&")) break
        if (t == "(" || t == "[" || t == "{") {
          if (!arr_pair[j]) break
          j = arr_pair[j] + 1
          continue
        }
        if (t == "=" && !eq) eq = j
        j++
      }
      if (!eq) continue
      add(i, eq + 1, j - 1, pnames(i + 1, pattern_end(i + 1, eq)))
      continue
    }
    if (t == "for") {
      j = i + 1
      while (j <= ntok) {
        t = arr_tok[j]
        if (t == "in" || t == ";" || t == ")" || t == "]" || t == "}") break
        if (t == "(" || t == "[" || t == "{") {
          if (!arr_pair[j]) break
          j = arr_pair[j] + 1
          continue
        }
        j++
      }
      if (arr_tok[j] != "in" || j == i + 1) continue
      k = j + 1
      while (k <= ntok) {
        t = arr_tok[k]
        if (t == "{" || t == ";" || t == ")" || t == "]" || t == "}") break
        if (t == "(" || t == "[") {
          if (!arr_pair[k]) break
          k = arr_pair[k] + 1
          continue
        }
        k++
      }
      if (arr_tok[k] != "{") continue
      add(i, j + 1, k - 1, pnames(i + 1, j - 1))
      continue
    }
    if (t == "match") {
      arms(i)
      continue
    }
    if (!ident(t) || (t in arr_stopword) || t ~ /^[A-Z0-9]/) continue
    nm = arr_tok[i - 1]
    if (i > 1 && nm != ";" && nm != "{" && nm != "}" && nm != "=>") continue
    if (arr_tok[i + 1] == "=") eq = i + 1
    else if (arr_tok[i + 1] == "+" && arr_tok[i + 2] == "=") eq = i + 2
    else continue
    j = eq + 1
    while (j <= ntok) {
      t = arr_tok[j]
      if (t == ";" || t == "," || t == ")" || t == "]" || t == "}") break
      if (t == "(" || t == "[" || t == "{") {
        if (!arr_pair[j]) break
        j = arr_pair[j] + 1
        continue
      }
      j++
    }
    add(i, eq + 1, j - 1, " " arr_tok[i])
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
# The tokens a..b without a leading * or & (and the mut after &) and without parentheses
# around the whole, into sa and sb.
function strip(a, b) {
  while (1) {
    while (a <= b && (arr_tok[a] == "*" || arr_tok[a] == "&" || (arr_tok[a] == "mut" && arr_tok[a - 1] == "&"))) a++
    if (a < b && arr_tok[a] == "(" && arr_pair[a] == b) {
      a++
      b--
      continue
    }
    break
  }
  sa = a
  sb = b
}
# The first token of the method chain that ends before the . at k, not before lo: the receiver.
function recv_start(k, lo,   j, t) {
  j = k - 1
  while (j >= lo) {
    t = arr_tok[j]
    if (t == ")" || t == "]") {
      if (!arr_pair[j] || arr_pair[j] < lo) break
      j = arr_pair[j] - 1
      continue
    }
    if ((ident(t) && !(t in arr_stopword) && t != "as" && t != "mut") || t ~ /^[0-9]+$/ || t == "::" || t == "." || t == "?") {
      j--
      continue
    }
    break
  }
  return j + 1
}
# What the tokens a..b are, exactly, with a method chain after them: 1 an elapsed value (a call
# of a clock method, see direction, or a tracked name), 2 the time left, 0 neither.
function value(a, b,   i, t, head, res, p) {
  if (a > b) return 0
  t = arr_tok[a]
  head = 0
  if (t == "(") {
    p = arr_pair[a]
    if (!p || p > b) return 0
    head = kind(a + 1, p - 1)
    i = p + 1
  } else if (!ident(t)) {
    return 0
  } else if ((t in arr_rank) && arr_since[t] < a && (a == b || (arr_tok[a + 1] != "::" && arr_tok[a + 1] != "(" && arr_tok[a + 1] != "!"))) {
    head = 1
    i = a + 1
  } else {
    i = a + 1
    while (i + 1 <= b && arr_tok[i] == "::" && ident(arr_tok[i + 1])) i += 2
    if (i <= b && arr_tok[i] == "(") {
      if (!arr_pair[i] || arr_pair[i] > b) return 0
      i = arr_pair[i] + 1
    }
  }
  res = (head == 1 || head == 2) ? head : -1
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
    if (t == "as") {
      i++
      if (i > b || !ident(arr_tok[i])) return 0
      i++
      while (i + 1 <= b && arr_tok[i] == "::" && ident(arr_tok[i + 1])) i += 2
      continue
    }
    if (t != "." || i + 1 > b) return 0
    t = arr_tok[i + 1]
    if (t ~ /^[0-9]+$/) {
      i += 2
      continue
    }
    if (!ident(t)) return 0
    if (i + 2 <= b && arr_tok[i + 2] == "(") {
      p = arr_pair[i + 2]
      if (!p || p > b) return 0
      if (res < 0 && (t in arr_clock)) res = (t == "elapsed") ? 1 : direction(a, i - 1, i + 3, p - 1)
      i = p + 1
    } else {
      i += 2
    }
  }
  return res < 0 ? 0 : res
}
# Whether the tokens a..b are exactly a clock read: a path ending in Instant::now() or
# SystemTime::now().
function isclock(a, b,   m) {
  if (b - a < 4 || (b - a) % 2) return 0
  if (arr_tok[b] != ")" || arr_tok[b - 1] != "(" || arr_tok[b - 2] != "now" || arr_tok[b - 3] != "::" || !(arr_tok[b - 4] in arr_clocktype)) return 0
  for (m = a; m < b - 4; m += 2)
    if (!ident(arr_tok[m]) || arr_tok[m + 1] != "::") return 0
  return 1
}
# Whether the operand a..b is a clock read (direct > 0) or, when direct is 0, also a name bound
# to exactly a clock read before a.
function clockish(a, b, direct) {
  strip(a, b)
  a = sa
  b = sb
  if (a > b) return 0
  if (isclock(a, b)) return 1
  return !direct && a == b && (arr_tok[a] in arr_clockat) && arr_clockat[arr_tok[a]] < a
}
# Whether the operand a..b is a fixed instant: a path that ends in an upper-case constant, or a
# name bound before a from a value that holds + or - and a clock read or a name bound to one (a
# deadline, s + WAIT).
function fixed(a, b) {
  strip(a, b)
  a = sa
  b = sb
  if (a > b) return 0
  if (a == b && (arr_tok[a] in arr_fixedat) && arr_fixedat[arr_tok[a]] < a) return 1
  return ispath(a, b) && arr_tok[b] ~ /^[A-Z][A-Z0-9_]*$/
}
# Whether the tokens a..b are a path (one name or names joined by ::).
function ispath(a, b,   m) {
  if (a > b || (b - a) % 2) return 0
  for (m = a; m < b; m += 2)
    if (!ident(arr_tok[m]) || arr_tok[m + 1] != "::") return 0
  return ident(arr_tok[b])
}
# What a.duration_since(b) is, its forms saturating_ and checked_ too, for the receiver ra..rb
# and the argument aa..ab: 0 no elapsed value when b is UNIX_EPOCH (a timestamp) or when a is a
# fixed instant and b one too or a name bound to a clock read; 1 an elapsed value when a is a
# clock read or a name bound to one, and when a side is neither of these nor a fixed instant (a
# parameter, a field: it may hold a clock read); 2 the time left when b is a clock read and a is
# not.
function direction(ra, rb, aa, ab,   ca) {
  strip(aa, ab)
  if (ispath(sa, sb) && arr_tok[sb] == "UNIX_EPOCH") return 0
  if (clockish(ra, rb, 0)) return 1
  ca = clockish(aa, ab, 0)
  if (ca && clockish(aa, ab, 1)) return 2
  if (fixed(ra, rb) && (ca || fixed(aa, ab))) return 0
  return 1
}
# What the call of the clock method after the . at k is, its receiver not before lo: 1 an
# elapsed value, 2 the time left, 0 neither.
function callkind(k, lo,   p) {
  if (arr_tok[k + 1] == "elapsed") return 1
  p = arr_pair[k + 2]
  if (!p) return 1
  return direction(recv_start(k, lo), k - 1, k + 3, p - 1)
}
# What the operand a..b is: 1 an elapsed value, 2 the time left, 3 a clock read, 0 neither.
function kind(a, b) {
  strip(a, b)
  a = sa
  b = sb
  if (a > b) return 0
  if (isclock(a, b)) return 3
  return value(a, b)
}
# Every comparison of the tokens lo..hi (<, <=, >, >= and the methods lt, le, gt, ge), by its
# smaller operand s and its larger operand l: c_bound when s is an elapsed value or a clock read
# or l is the time left (an upper bound on the wait); c_up when l is an elapsed value or a clock
# read and s is not of its kind, or s is the time left and l is not (upward); c_down the other
# way round (downward).
function compare(lo, hi,   k, t, a, b, p, ks, kl) {
  c_bound = 0
  c_up = 0
  c_down = 0
  for (k = lo; k <= hi; k++) {
    t = arr_tok[k]
    if (t == "<" || t == "<=") {
      a = left_start(k, lo)
      b = right_end(k, hi)
      ks = kind(a, k - 1)
      kl = kind(k + 1, b)
    } else if (t == ">" || t == ">=") {
      a = left_start(k, lo)
      b = right_end(k, hi)
      kl = kind(a, k - 1)
      ks = kind(k + 1, b)
    } else if (t == "." && (arr_tok[k + 1] in arr_cmpfn) && arr_tok[k + 2] == "(" && arr_pair[k + 2] && arr_pair[k + 2] <= hi) {
      p = arr_pair[k + 2]
      a = recv_start(k, lo)
      if (arr_tok[k + 1] == "lt" || arr_tok[k + 1] == "le") {
        ks = kind(a, k - 1)
        kl = kind(k + 3, p - 1)
      } else {
        kl = kind(a, k - 1)
        ks = kind(k + 3, p - 1)
      }
    } else continue
    if (ks == 1 || ks == 3 || kl == 2) c_bound = 1
    if (((kl == 1 || kl == 3) && ks != kl) || (ks == 2 && kl != 2)) c_up = 1
    if (((ks == 1 || ks == 3) && kl != ks) || (kl == 2 && ks != 2)) c_down = 1
  }
}
# Whether the tokens strictly between o and c fail the test.
function fails(o, c,   j, t) {
  for (j = o + 1; j < c; j++) {
    t = arr_tok[j]
    if ((t == "panic" || t == "unreachable") && arr_tok[j + 1] == "!") return 1
    if (t ~ /^(debug_)?assert/) return 1
  }
  return 0
}
# The last token of the body of a match arm that starts at s: its block, or its expression to
# the , or the } of the match.
function arm_end(s,   j, t) {
  if (arr_tok[s] == "{" && arr_pair[s]) return arr_pair[s]
  j = s
  while (j <= ntok) {
    t = arr_tok[j]
    if (t == "," || t == ")" || t == "]" || t == "}") break
    if (t == "(" || t == "[" || t == "{") {
      if (!arr_pair[j]) break
      j = arr_pair[j] + 1
      continue
    }
    j++
  }
  return j - 1
}
function statements(   i, t, kw, o, j, c, brace, guard, n) {
  n = 0
  for (i = 1; i <= ntok; i++) {
    t = arr_tok[i]
    if ((t in arr_assert) && arr_tok[i + 1] == "!") {
      o = i + 2
      t = arr_tok[o]
      if ((t != "(" && t != "[" && t != "{") || !arr_pair[o]) continue
      compare(o + 1, arr_pair[o] - 1)
      if (c_bound) arr_found[++n] = arr_line[i]
      continue
    }
    if (t != "if" && t != "while") continue
    kw = t
    j = i + 1
    brace = 0
    guard = 0
    while (j <= ntok) {
      t = arr_tok[j]
      if (t == "{") {
        brace = j
        break
      }
      if (t == "=>") {
        guard = j
        break
      }
      if (t == "(" || t == "[") {
        if (!arr_pair[j]) break
        j = arr_pair[j] + 1
        continue
      }
      if (t == ";" || t == ")" || t == "]" || t == "}") break
      j++
    }
    if (guard && kw == "if") {
      compare(i + 1, guard - 1)
      if (c_up && fails(guard, arm_end(guard + 1) + 1)) arr_found[++n] = arr_line[i]
      continue
    }
    if (!brace || !arr_pair[brace]) continue
    compare(i + 1, brace - 1)
    c = arr_pair[brace]
    if (c_up && fails(brace, c)) {
      arr_found[++n] = arr_line[i]
      continue
    }
    if (kw == "if" && c_down && arr_tok[c + 1] == "else" && arr_tok[c + 2] == "{" && arr_pair[c + 2] && fails(c + 2, arr_pair[c + 2])) arr_found[++n] = arr_line[i]
  }
  return n
}
# The refused statements of the file path, their lines in arr_found[1..]; -1 when unreadable.
function scan(path,   n, line, got, has, now, typ) {
  n = 0
  has = 0
  now = 0
  typ = 0
  while ((got = (getline line < path)) > 0) {
    n++
    arr_raw[n] = line
    if (has) continue
    if (index(line, "elapsed") || index(line, "duration_since")) has = 1
    if (!now && index(line, "now") && line ~ /(^|[^A-Za-z0-9_])now([^A-Za-z0-9_]|$)/) now = 1
    if (!typ && (index(line, "Instant") || index(line, "SystemTime"))) typ = 1
  }
  close(path)
  if (got < 0) return -1
  if (!has && !(now && typ)) return 0
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
  split("elapsed duration_since saturating_duration_since checked_duration_since", arr_list, " ")
  for (x in arr_list) arr_clock[arr_list[x]] = 1
  split("lt le gt ge", arr_list, " ")
  for (x in arr_list) arr_cmpfn[arr_list[x]] = 1
  split("Instant SystemTime", arr_list, " ")
  for (x in arr_list) arr_clocktype[arr_list[x]] = 1
  split("if while return let match in else break continue loop for move", arr_list, " ")
  for (x in arr_list) arr_stopword[arr_list[x]] = 1
  split("mut ref box true false self", arr_list, " ")
  for (x in arr_list) arr_notname[arr_list[x]] = 1
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
    if (!unsorted && nsorted && (arr_field[1] "") < (last_path "")) {
      printf "test-clocks-check: %s:%d: \"%s\" is out of order; the lines are sorted by path\n", shown, nledger, entry
      status = 1
      unsorted = 1
    }
    nsorted++
    last_path = arr_field[1]
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
