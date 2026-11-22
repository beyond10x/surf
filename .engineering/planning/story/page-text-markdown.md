---
format: aep.planning-md/3
id: story:page-text-markdown
kind: story
status: implemented
title: A page's content as Markdown, tables aside
summary: surf-page-text renders headings, paragraphs, links, lists, landmarks, code, images and controls as escaped Markdown (UTF-8 by rule 2) that page text cannot forge; a table is its text until page-text-tables.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:page-text-outline
- depends_on: story:html-accessible-name
scope:
- confidence: inferred
  path: crates/page-text/src/lib.rs
- confidence: inferred
  path: crates/page-text/src/markdown.rs
- confidence: inferred
  path: crates/page-text/tests/cases/markdown.rs
- confidence: inferred
  path: crates/page-text/tests/main.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T04:59:50Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T04:59:50Z", actor: "human:operator", revision: 8, decided_on: {"recorded":{"verification":1}}}
- {from: "active", to: "implemented", at: "2026-11-22T16:29:17Z", actor: "human:operator", revision: 11, decided_on: {"recorded":{"test_result":1,"verification":2}}}
---
## Context

`docs/design/page-as-text.md` (proposed) sections 3 and 4: a page's content as Markdown
(CommonMark 0.31.2 with the tables of GitHub Flavored Markdown 0.29, from memory, confirmed by
this story against the two specifications), computed from the tree without CSS. The operator,
2026-11-22: "the possibility to create stuff in Markdown directly". This story writes everything
but tables, because the tree builder ignores `tr` and `td` start tags until `html-tree-tables`;
a `table` is written as one paragraph of its text until `page-text-tables`. Headings,
paragraphs, links, lists, landmarks, controls and every rule against forged text land here.

`page-text-outline` creates `crates/page-text` and `surf_page::handles`; the labels are those of
`html-accessible-name` (the label field of `surf_page::ActionableElement`); hidden is
`surf_page::is_hidden`. Page text is untrusted input to a language model
(`docs/agent-answer.md`): page text can write neither an unescaped Markdown construct nor a line
of its own; its characters follow rule 2 of design note section 3 (D1 B, the operator's) through
`surf_answer::escaped_in_text`. The note states the limit: the escapes protect a parser, not
established for a model reader.

Placement (the coordinator, 2026-11-22): round 1, the text unit, after `page-text-outline` and
before `page-text-tables`; it needs `html-accessible-name`. It writes only `crates/page-text`.

## Change

- `crates/page-text/src/markdown.rs` and one `pub mod` line in `src/lib.rs`:
  `pub fn markdown(tree: &Tree) -> impl Iterator<Item = String>`, the lines without the `>`
  prefix (an empty line is the empty string), one at a time, in one walk with its own stack, and
  a first walk that counts the lines of each landmark:
  - the element map of design note section 4, each row of it: the fixed list of block elements
    taken from the `display: block` and `display: list-item` rules of the HTML Standard's
    rendering section (the snapshot `docs/design/html-parser.md` names), listed in the crate
    documentation; blocks parted by one empty line, the items of one list not; headings,
    paragraphs, `br`, `hr`, `blockquote`, emphasis, `code`, `pre`/`listing`/`xmp`, links with
    `[@l<n>]` (block content and `img` inside joined into one run), `img` by `alt`, `iframe` as
    `[iframe](<src>)`, lists with nesting and `start`, `dl` as `**term**` and paragraphs,
    controls as `[@<handle> <kind>[ disabled][ required][ checked]: <label>]` (no `option`
    text), landmarks `nav header footer main aside` with the line `[<name> <n> lines]` before
    their content (none for an empty one), `noscript` content written;
  - dropped: hidden nodes, comments, doctypes, processing instructions, `svg` and `math`;
    `meta` and `lang` are not written;
  - whitespace: ASCII whitespace stripped and collapsed per block, U+00A0 kept;
  - the escapes of design note section 3, rules 2 to 4: the characters of rule 2, one set in
    text, link text and link destination, the line-start set applied after wrapping, in code
    only `\ [ ] < |`, fences and spans one backtick longer than the longest run inside; every
    wrap and cut between escape units;
  - bounds of section 6: a line wrapped at the last space before 1,024 bytes (a longer word
    broken between units), heading, link text and `alt` cut at 200 characters of page text and
    a label at 60, each ending in `[cut]`, the escapes added before the cut is placed;
  - `table`: one paragraph of its text, until `page-text-tables`.
- The crate documentation lists the rules, the deviations (`\\`, `\[`, `\<`, `\|` and `\x..` in
  code show as written in a renderer; a word broken at 1,024 bytes reads as two; `href` as
  written, not resolved; a landmark line joins the paragraph after it in a renderer) and what is
  not claimed (no CSS, no script, no model reader).
- Tests in `crates/page-text/tests/cases/markdown.rs`, a module of the one test binary
  `tests/main.rs`, declared there as `#[path = "cases/markdown.rs"] mod markdown;`
  (`taskfiles/test-layout.sh` passes), no server.

Writes: `crates/page-text/src/markdown.rs` (new), `crates/page-text/src/lib.rs`,
`crates/page-text/tests/cases/markdown.rs` (new), `crates/page-text/tests/main.rs` (the line
`#[path = "cases/markdown.rs"] mod markdown;`).

Bounds for every command of this story. Every build and test runs through the build pool:
`cargo` is wrapped by the pool's shim, every other command that builds or tests gets the `slot`
prefix. The pool ends a job after 10 minutes; a command that needs longer names its time with
`slot --timeout <t>` or `SLOT_TIMEOUT=<t>`, at most 30 minutes; a `timeout` wrapped around the
command extends nothing. A script that measures or waits starts without the `slot` prefix; the
builds and test runs inside it carry it. No test runs over 60 s and the test step ends within
120 s. No Python.

## Acceptance

`cargo test -p surf-page-text` passes; no test runs over 60 s and the test step ends within 120 s
(the folder's budget). Each case gives exactly the lines named, the empty ones as the empty
string:

1. `fruit_shop_markdown_without_table`: the page of design note section 1 without its `table`:
   `# Fruit shop`, ``, `Fresh fruit, *picked today*. See [our offers](</offers>) [@l1].`, ``,
   `- Apples`, `- Pears`, ``, `![A basket of apples]`, ``,
   `E-mail [@f1 text: E-mail] [@b1 button: Subscribe]`.
2. `forged_page_without_table`: the forged page of section 3 without its `table`: the 16 lines
   of the section's block that remain, byte for byte.
3. `a_fence_is_longer_than_any_run_inside`: `<pre>a ``` b</pre>` gives ` ```` `, `a ``` b`,
   ` ```` `; `<code>a`b</code>` gives ``` ``a`b`` ```.
4. `code_cannot_write_a_link_or_a_handle`: `<pre>[Bank](<http://bank.example/>) [@l1] | x</pre>`
   gives ```` ``` ````, `\[Bank\](\<http://bank.example/>) \[@l1\] \| x`, ```` ``` ````.
5. `line_start_characters_are_escaped`: paragraphs `- x`, `+ x`, `= x`, `2) x`, `# x`, `> x`
   give `\- x`, `\+ x`, `\= x`, `2\) x`, `\# x`, `\> x`.
6. `nested_lists_and_quotes`: `<ul><li>a<ol start=3><li>b</ol></ul><blockquote>q</blockquote>`
   gives `- a`, ``, `  3. b`, ``, `> q`.
7. `hidden_content_is_dropped`: `<p>a</p><p hidden>h1</p><div aria-hidden="true">h2</div>
   <script>h3</script><style>h4</style><template>h5</template><details><summary>s</summary>h6
   </details><input type=hidden value=h7><p>b</p>` gives `a`, ``, `s`, ``, `b`.
8. `long_runs_are_wrapped_and_cut`: a `p` of 300 words `abcdefghi` gives three lines of 102, 102
   and 96 words (1,019, 1,019 and 959 bytes; the 1,024 do not count `> `); a `p` of 2,000 `a`
   gives 1,024 `a` and 976 `a`; an `h1` of 300 `a` gives `# `, 200 `a` and `[cut]`;
   `<button>` and 100 `a` gives `[@b1 button: `, 60 `a`, `[cut]]`.
9. `wraps_and_cuts_fall_between_units`: a `p` of 1,023 `a` and `[b` gives 1,023 `a`, then `\[b`;
   a `p` of 1,023 `a`, a space and `# x` gives 1,023 `a`, then `\# x`; `<button>`, 59 `a` and `[[`
   gives `[@b1 button: `, 59 `a`, `\[[cut]]`.
10. `links_hold_one_run_and_one_escape_set`: `<a href="x>) [@l9](<y">t</a>` gives
   `[t](<x\>) \[@l9\](\<y>) [@l1]`; `<a href="/x"><h2>Title</h2><img alt="pic"></a>` gives
   `[Title pic](</x>) [@l1]`.
11. `the_rest_of_the_map`: `<p>a<br>b</p><hr><dl><dt>T<dd>D</dl><iframe src="/ad"></iframe>`
   gives `a\`, `b`, ``, `---`, ``, `**T**`, ``, `D`, ``, `[iframe](</ad>)`;
   `<label for=s>Size</label><select id=s><option>S<option>M</select>` gives
   `Size [@f1 select: Size]`; `<label>Terms <input type=checkbox checked required
   disabled></label>` gives `Terms [@f1 checkbox disabled required checked: Terms]`;
   `<html lang="de"><head><meta name="description" content="d"></head><body><noscript>n
   </noscript><p>a&nbsp;&nbsp;b</p>` gives `n`, ``, `a` U+00A0 U+00A0 `b`;
   `<p>x</p><table><tr><td>a b</td></tr></table><p>y</p>` gives `x`, ``, `a b`, ``, `y`
   (`page-text-tables` replaces this last one).
12. `landmarks_have_a_line_of_surf`: `<nav><a href=/a>A</a> <a href=/b>B</a></nav><main><p>x
   </p><p>y</p></main>` gives `[nav 1 lines]`, `[A](</a>) [@l1] [B](</b>) [@l2]`, ``,
   `[main 3 lines]`, `x`, ``, `y`; `<p>&#91;nav 1 lines]</p>` gives `\[nav 1 lines\]`.
13. (D1 B) `characters_follow_rule_2`: a `p` of the UTF-8 bytes of `K&auml;se`, U+200B, U+0085,
   U+0007, U+E0041 (all four raw, not references), then `&#0;` and U+1F600 raw gives the one line
   `K` U+00E4 `se\xe2\x80\x8b\xc2\x85\x07\xf3\xa0\x81\x81` U+FFFD U+1F600 (bytes `c3 a4`,
   `ef bf bd` and `f0 9f 98 80` as they are); over the pages of cases 1 to 15 no line holds a
   character `escaped_in_text` names.
14. (D1 B) `fullwidth_syntax_is_escaped`: `<p>［Bank］（＜http://bank.example/＞） ［＠l1］</p>`
   gives the one line
   `\xef\xbc\xbbBank\xef\xbc\xbd\xef\xbc\x88\xef\xbc\x9chttp://bank.example/\xef\xbc\x9e\xef\xbc\x89 \xef\xbc\xbb\xef\xbc\xa0l1\xef\xbc\xbd`;
   `<p>東京（とうきょう）。Ａ</p>` gives `東京\xef\xbc\x88とうきょう\xef\xbc\x89。Ａ` (the rest of the
   fullwidth block kept).
15. (D1 B) `a_mark_cannot_start_a_run`: `<p>&#x338;x <a href="/a">&#x338;y</a> e&#x301;</p>` gives
   `\xcc\xb8x [\xcc\xb8y](</a>) [@l1] e` U+0301 (bytes `cc 81` kept inside the run).

`cargo fmt -p surf-page-text -- --check` exits 0.

## Out of scope

Tables (`page-text-tables`); printing in an answer (`page-text-look`, `page-text-surf-command`);
resolving link targets (`agent-follow-links`); control values (`now`); whether a model reader
tells `\[` from `[` (a limit the design note states).

## As built

What differs from the text above, as of 64c6621:

- Context, "from memory, confirmed by this story against the two specifications": only parts of
  GitHub Flavored Markdown 0.29 were read (the tables extension, the info string of a backtick
  fence, the 1 to 9 digits of a list number); the rest is from memory of CommonMark 0.31.2, as
  the crate documentation of `crates/page-text/src/lib.rs` states.
- Change, block list "taken from ... the snapshot `docs/design/html-parser.md` names": the list
  was taken from the HTML Standard's rendering section as published on 2026-11-22, not from the
  snapshot; it has 46 names, `option`, `optgroup` and `table` among them (`BLOCK_ELEMENTS` and its
  doc comment in `crates/page-text/src/markdown.rs`).
- Change, `pub fn markdown(tree) -> impl Iterator<Item = String>`: it returns `Markdown<'_>`, an
  iterator of the same lines with `deep()`, the levels of lists and quotes written without their
  prefix, known before the first line.
- Change, bounds, "a longer word broken between units": a link with its handle, an image, an
  `iframe`, a code span and a control are never broken across lines; a line may break between two
  of them where the page has no space. One longer than `ATOM_BYTES_MAX` (864 bytes) is cut before
  `[cut]`: a link's text and its target both, a cut target ending `[cut]>`, the handle whole. See
  the `adversary_*` cases of `crates/page-text/tests/cases/adversary_text.rs`.
- Change, a heading: it is always one line, cut before `[cut]` when it does not fit, not wrapped.
- Change, "heading ... cut at 200 characters of page text": a link, image or code span in a
  heading spends the characters of page text it shows (at least 1), and the cut falls between
  them (`adversary_a_cell_and_a_heading_keep_200_characters`).
- Change, "lists with nesting": the prefix of a line (list markers, indentation, `> `) has at most
  `PREFIX_BYTES_MAX` (128) bytes. An item or a quote that would pass it is written at the
  indentation of the one it is in, without its marker, and the answer says `note text: deep <n>`;
  so a line's text keeps at least 896 bytes and the Markdown grows with the page, not with its
  depth times its words. See `deep_nesting_is_written_at_the_bound` and
  `output_grows_linearly_with_nesting` in `crates/page-text/tests/cases/markdown.rs`.
- Change, `start`: a list number stops at 999,999,999 (`LIST_NUMBER_MAX`, the 9 digits CommonMark
  allows); a `start` outside 0 to 999,999,999 or not a number is 1 (`escapes_and_list_starts`).
- Change, `pre`/`listing`/`xmp` as a fence: when the fence would need more than
  `FENCE_BYTES_MAX` (16) backticks, the text is an indented code block (four spaces, no fence),
  so no line passes 1,024 bytes (`adversary_no_line_passes_1024_bytes`).
- Change, controls `[@<handle> <kind>...: <label>]`: with an empty label the `: ` and the label
  are left out. An `img` with a blank `alt` and an `iframe` without `src` write nothing.
- Change, "dropped: hidden nodes": an element with a `popover` attribute is hidden unless it is
  an open `dialog`, and so is the copy of the selected `option` in a `selectedcontent`
  (`adversary_a_popover_is_hidden`, `crates/page-text/tests/cases/select.rs`).
- Change, the deviations the crate documentation lists: it also names two atoms broken at 1,024
  bytes reading as two, a `\` of a `br` left out where it would pass 1,024 bytes, and an item or
  a quote nested past 128 bytes of prefix losing its marker.
- Acceptance 4 does not hold as written: in `<pre>[Bank](<http://bank.example/>) ...` the parser
  reads `<http:` as a start tag, so the case writes the text with `&lt;`
  (`code_cannot_write_a_link_or_a_handle`, comment in the test).
- Writes: besides the files listed, `docs/design/page-as-text.md` (sections 4 and 6 for the bounds
  above) and the cases in `crates/page-text/tests/cases/adversary_text.rs` and `select.rs`, each
  with its line in `crates/page-text/tests/main.rs`.
