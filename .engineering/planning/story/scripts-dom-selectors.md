---
format: aep.planning-md/3
id: story:scripts-dom-selectors
kind: story
status: proposed
title: The 9 selector members over a subset of Selectors Level 4, bounded in text, count and steps
summary: 'querySelector, querySelectorAll, closest, matches and webkitMatchesSelector on Element and the two of Document and DocumentFragment, for the subset of type, universal, #id, .class, [a], [a=v] (CSS escapes, strings, comments and the newlines CR, FF and CRLF included; an unterminated string or an unclosed block at the end of the text is closed, not an error), compounds, descendant and child combinators and lists, with the case rules of the HTML Standard (a type name and an attribute name lower-cased for an HTML element and compared exactly, the 46-name list of case-insensitive attribute values held as a table) and the tree''s quirks mode, every refusal a SyntaxError (a text that is not valid says so; a valid selector outside the subset or over the bounds of 4,096 bytes and 512 simple selectors says surf does not support it, as does a pseudo-class recognised by its shape even where a browser calls it not valid, two named deviations beside the nine rows), every read of the tree in a file of its own (selector_reads.rs) that charges before the work at one step per 16 bytes, through the glue''s constant, and a step a token, a node, an ancestor, a cell of the matcher and each further simple selector of a compound against the entry''s 10,000,000 steps, the walk keeping a stack of at most 513 frames and copying no list of children, the matcher filling every cell of a two-byte memo of compounds times chain that is charged before it is made, and querySelectorAll a static NodeList that is not a [SameObject] and keeps nothing.'
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-dom-reading
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/script/src/bindings.rs
- confidence: cited
  path: crates/script/src/dom/mod.rs
- confidence: cited
  path: crates/script/src/dom/reads.rs
- confidence: cited
  path: crates/script/src/dom/selector_reads.rs
- confidence: cited
  path: crates/script/src/dom/selectors.rs
- confidence: cited
  path: crates/script/tests/dom_reading.rs
- confidence: cited
  path: crates/script/tests/dom_selectors.rs
- confidence: cited
  path: crates/webidl/selection.txt
- confidence: cited
  path: docs/design/scripts.md
revision: 36
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 13}
---
## Context

The epic's eleventh candidate (`.engineering/planning/epic/scripts-in-a-page.md:478-481 @
929269e`), the slice "Selectors, 9" (`:742-750`) and the deviation lines (`:134-138`,
`:762-770`): `querySelector`, `querySelectorAll`, `closest`, `matches` and
`webkitMatchesSelector` for a subset of Selectors Level 4. The epic's text was corrected to
what this story rules (decision 5: the DOM Standard gives a caller only `SyntaxError`, so a
valid selector outside the subset throws a `SyntaxError` whose message says surf does not
support it, not `NotSupportedError`, and a pseudo-class recognised by its shape gets the same
message also where a browser says not valid: the epic holds both clauses of the deviation, since
plan commit `4013bde`; decision 14 says what was corrected). There are nine members, not the
seven the epic once counted; the selection
has nine rows with the slice `selectors` (`crates/webidl/selection.txt:110-112, 126-127,
175-176, 231-232 @ 83e6b8a`, checked with `awk '$3=="selectors"{n++} END{print n}'`): `Element`
`closest`, `matches`, `webkitMatchesSelector`, `querySelector`, `querySelectorAll`; `Document`
`querySelector`, `querySelectorAll`; `DocumentFragment` `querySelector`, `querySelectorAll`
(the mixin `ParentNode` is included by all three). None of the nine rows has a note today, and
none is a `[SameObject]` attribute: all nine are operations. The subset was
`story:scripts-decisions`' question Q9 and moved here (`docs/design/scripts.md:5`).

The base of this story is the reading unit as built, `83e6b8a` (branch of
`story:scripts-dom-reading`; `main` is `a14e3b0` and does not hold it). Every line below is of
`83e6b8a` unless it says `@ a14e3b0`. The reading unit built the structure of its decision 20
(one file of reads), but its prices are not built at `83e6b8a`: the glue and the reads still
charge per 64 bytes (`bindings.rs:628`, `reads.rs:313`). This story takes the prices from that
text (decision 7) and starts after the price commit is on its base (acceptance 1a shows it).
What is there:

- The trait `Selectors` (`crates/script/src/bindings.rs:3790-3881`) has the nine methods, each
  `(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId, selectors: String)`. The answers are
  `DomResult<Option<NodeId>>` for the three `querySelector` and `closest`, `DomResult<bool>` for
  `matches` and `webkitMatchesSelector`, and `DomResult<Vec<NodeId>>` for the three
  `querySelectorAll` (`NodeList`, `[NewObject]`, `dom.html:5058`). Every default answers
  `NotSupportedError` naming the member (`not_supported`, `bindings.rs:150-155`); the slice
  replaces that default for the nine, and a member of another slice keeps it.
  `crates/script/src/dom/selectors.rs:1-8` is an empty `impl` of the unit struct
  `SurfSelectors`, built at `crates/script/src/dom/mod.rs:54`;
  `crates/script/tests/dom_selectors.rs:1` holds its head comment only and
  `crates/script/tests/main.rs:21` declares it.
- The workspace holds no selector parser and no matcher (`git grep -n -i 'selector' 83e6b8a --
  crates ':!crates/script/src/bindings.rs' ':!crates/webidl/data'` finds only the SVG attribute
  name table `crates/html/src/tree_builder/foreign.rs:161-162`, test data, the three
  `TestSelectors` types of the tests and the stub). There is no CSS crate. The story writes the
  parser and the matcher. The tree's public reads are `Tree::parent` (`crates/html/src/tree.rs:
  624`), `children` (`:665`), `data` (`:655`), `root` (`:646`), `quirks_mode` (`:338`),
  `NodeData::Element { namespace, local_name, attributes, .. }` (the enum `NodeData` is at
  `:135`) and `Attribute` (`:111`); the members reach them only through `reads.rs` and
  `selector_reads.rs` (decision 17). `Tree::children` answers a slice `&[NodeId]` and
  `Tree::parent` an `Option<NodeId>`, both in constant time; the tree has no link to a sibling
  or a first child.
- The glue of each member (for example `operation_element_query_selector_all`,
  `bindings.rs:5321-5342`; the nine are at `:4991, :5006, :5021, :5303, :5321, :6157, :6175,
  :7119, :7139`) charges one step, checks the brand of `this`, requires the argument, converts
  it to a String and copies it with `Copies::copy` (`bindings.rs:620-630`): the whole
  `DOMString`, a lone surrogate written U+FFFD (3 bytes), and a charge of one step per 64 bytes
  after the copy (`:628`; the reading story's decision 20 rules 16, not built at `83e6b8a`). It
  then calls the slice. A `NodeList` answer goes through `collection_value`
  (`bindings.rs:1314-1345`): `COLLECTION_NODE_STEPS`, 8 steps a node (`:1251`, `:1333`), a new
  host object holding the snapshot, one wrapper per node through `Wrap`. A `DomEnd::Throw`
  becomes a `DOMException` whose name and message are cut at `EXCEPTION_TEXT_BYTES`, 1,024
  bytes (`bindings.rs:399`); `SyntaxError` is a name of Web IDL's table with code 12
  (`bindings.rs:2274`).
- The reading unit's own pieces this story uses: `crates/script/src/dom/reads.rs` holds every
  read of the tree of the slice `reading`, and `reading.rs` holds the members and no call of
  `tree()` (a test reads its source, `dom_reading_cost.rs:593-598`). `reads.rs` has a private
  `charge(cx, steps)` (`:120-131`: `cx.engine().charge_steps`, past the limit a throw the engine
  does not deliver, no recorded limit), a walk with its own stack `find` (`:349-366`: one step
  for a node, then `pending.extend(tree.children(next).iter().rev())`, all of its children at
  once; this story's walk does not do that, decision 17) and `select` (`:370-384`), whose
  callback gets `&Tree` and no `cx`, `scan_for` (`:303-322`: one
  step per 4 attributes of the list before the one pass, then the value's bytes, then one per
  class token) and `quirks(cx)` (`:1022-1025`), `positions` and `value_at` (`:267-299`),
  `ATTRIBUTES_A_STEP` (`:252`) and `attribute_count` (`:255`), all private, and the enum
  `Reads` (`:236`, `Name` and `Value` take a `&'static str`), and the private `parent_element_in`
  (`:578`), `by_tag_name`'s match (`:395-410`) and `by_class_name` (`:454-506`), which the
  story copies on purpose (decision 17). `SurfWrap` keeps one wrapper per node
  (`crates/script/src/dom/wrapper.rs:20-25`, `:60-73`); `dom::slices(info: DocumentInfo)`
  (`mod.rs:49`) and `DocumentInfo` with its `Default` (`mod.rs:28-46`); `QuirksMode::{NoQuirks,
  LimitedQuirks, Quirks}` (`crates/html/src/tree_builder.rs:233-239`, re-exported at
  `crates/html/src/lib.rs:227`).
- The limits a selector can reach: the entry's 10,000,000 steps (`runner.rs:41`), the heap
  `memory_bytes` 67,108,864 (`runner.rs:42`), `string_length` 16,777,216 (`runner.rs:46`), the
  tree's `max_nodes` 1,048,576, `max_depth` 512 and `max_attributes` 1,024
  (`crates/html/src/limits.rs:104-106`). After `story:html-script-hook` no node of any tree is
  deeper than `max_depth`, counted as the write surface counts: the elements on its path from
  the root, itself included (its "As built"), so an element has at most 511 element ancestors.
  `Tree::descendants` (`tree.rs:635`) returns a list of every node, which this story does not
  use; a `template` element's contents are a separate fragment node and not among its children.
- The memory of a node a script reaches, as the reading unit measured it (`crates/script/tests/
  dom_reading.rs:1701-1706`): 423 bytes per wrapper with its data, 349 bytes per empty
  collection object and about 121 bytes per node of a collection beside its wrapper. So a
  `NodeList` of `n` nodes not yet wrapped costs about 544 bytes a node and the heap ends it at
  about 123,000 nodes (67,108,864 / 544), not at the 262,144 wrappers the reading story's text
  bounds it by.
- The standards. The DOM Standard says the three `ParentNode` and `Element` methods in
  `dom.html:737-757` (scope-match a selectors string), `:5234-5244` (`querySelector`,
  `querySelectorAll`: "the static result") and `:11106-11160` (`closest`, `matches`); the
  descendant rule is in the notes at `:5130` and `:5137`. The Selectors Level 4 text is not in
  the tree: the DOM page names it only as a reference (`dom.html:744`, `:754`, `:17240-17245`,
  `:17374-17375`). `dom.html:754-755` says a match runs "against a tree" with the node's root
  and the node as scoping root. An agent has read four texts for this story, through a fetch
  tool that paraphrases sentences over 125 characters: Selectors Level 4, CSS Syntax Level 3,
  the HTML Standard's "Case-sensitivity of selectors" and the DOM Standard (decision 16 gives
  each text's date, what was read and what was not). Its findings are decisions 3, 4, 5 and
  15 below: they agree with the first draft in 24 places and differ in six (the case rule of
  a name, `ns|a`, `||`, the exception of a valid selector the slice does not support, the
  newline, and the cases of a scripted attribute name). The implementor reads each text once
  itself, for the parts the agent did not read (decision 16).

The reading story's decisions this story relies on: 10 and 18 (`[SameObject]` for four rows
only, none of this slice's), 17 and 18 (a kept collection is refilled all or nothing; this
slice keeps none), 7, 19 and 20 (charges), 14 and 15 (the test slice and the table).

## Decisions taken

1. The members are the nine methods of `Selectors` (`bindings.rs:3790-3881`), the nine rows of
   the Context. The title, the summary and acceptance 1 say 9.
2. The subset is the epic's (`epic:742-750 @ 929269e`): the type selector, the universal
   selector, `#id`, `.class`, attribute presence `[a]` and equality `[a=v]`, compound
   selectors, the descendant and the child combinator, and selector lists (a comma), with the
   lexical rules of decision 15 (escapes, comments, strings, quoted and unquoted attribute
   values). Whitespace around a combinator is allowed (Selectors Level 4, section 16: it is
   required between units only where the combinator between them is omitted) and so around a
   comma (section 4.1). Whitespace around the whole text is allowed too, by CSS Syntax's
   tokenizing, which the agent did not read for this point (decision 16). The head comment of
   `selectors.rs` gives the grammar,
   each production cited to its section of Selectors Level 4 with the date of the text read,
   and the case rules of decisions 4 and 16 cited to the DOM and HTML Standards. The four
   texts are not in the tree; the implementor reads each once, when the work starts, at the
   addresses of decision 16, and nothing is vendored. `:scope` is outside the subset, with
   every other pseudo-class.
3. What a member answers, from the DOM Standard. `querySelector` and `querySelectorAll` match
   against the tree of `this`'s root with `this` as scoping root and answer the descendants of
   `this`, never `this`, in tree order (`dom.html:754-755`, `:5130`, `:5137`); an ancestor
   outside `this` still counts for a combinator. `querySelector` answers the first or `null`.
   `closest` tests `this`, then its element ancestors upward, and answers the first match or
   `null` (`dom.html:11106-11129`). `matches` and `webkitMatchesSelector` are one method and
   answer whether `this` matches (`dom.html:11137-11157`). A parent that is not an element (the
   document, a fragment) is the end of every upward walk; the walk never crosses from a
   template's contents to the template; no script reaches the contents of a template (no
   `content` member is selected), so this is not testable by a script, and whether Selectors
   Level 4's parent step (section 17.3) counts a template's contents fragment as a parent was
   not read: the implementor reads it (decision 16). `this` may be in a tree the document does
   not hold (a removed subtree, a fragment): the match runs over what its root holds. The
   standards read agree with all of this: a scoped selector matches only descendants of the
   scoping root and the rest of it matches unrestricted (Selectors 3.3, 17.5), `closest` walks
   the inclusive ancestors in reverse tree order (DOM 4.9), and the three `matches` forms
   are one set of steps (DOM 1.3).
4. Case rules, from the HTML Standard's section "Case-sensitivity of selectors" (4.16.2), which
   Selectors Level 4 section 3.7 repeats for attributes made by script. Every document a script
   reaches is an HTML document: surf has no XML parser, and the case rules of an XML document are
   out of scope.
   - A type name and an attribute name of the selector. For an element in the HTML namespace the
     selector's name is put in ASCII lower case and then compared exactly to the element's local
     name or to the attribute's name. For any other element or attribute (SVG, MathML) the
     selector's name is compared exactly as given. So `[FOO]` finds an attribute `foo` and finds
     nothing on an attribute `FOO` made with `setAttributeNS(null, "FOO", "1")` (the text: such an
     attribute cannot be matched by Selectors); `DIV` matches a parser-made `div`, matches no
     parser-made SVG or MathML element, and matches no element of the HTML namespace whose local
     name is `DIV` (one made with `createElementNS`: it is testable, the test builds it in Rust,
     acceptance 3), and `div` does not match it either.
   - For each type name and each attribute name of the text the parser keeps the name as
     decoded and its ASCII lower-case form, made once, in the parse; the matcher takes the form
     the element's namespace calls for. The fold is bytes folded and is charged at the bytes'
     price once, at the parse (decision 7). The attribute record also holds a flag, set by the
     parse when the lower-case form is in the table of decision 16, which the compare of a
     value reads (decision 17).
   - `[a]` and `[a=v]` name an attribute in no namespace: `[href]` does not match `xlink:href`.
   - An attribute value is compared exactly, except the attributes of the HTML Standard's list
     (decision 16) on an HTML element, which compare ASCII case-insensitively.
   - `#id` and `.class` compare exactly, and in quirks mode ASCII case-insensitively (Selectors
     6.6 and 6.7; `dom.html:7903-7906` for the class rule of `getElementsByClassName`, modes at
     `dom.html:8021-8026`). The mode is the tree's: `Tree::quirks_mode()` (`tree.rs:338`),
     `QuirksMode::Quirks` for "quirks" (the other two, `NoQuirks` and `LimitedQuirks`, compare
     exactly), read through `reads::quirks` (`reads.rs:1022-1025`, no charge).
   - A class selector matches a token of the `class` attribute split at ASCII whitespace; an
     element without that attribute has no class. The splitting is the reading unit's rule
     (`by_class_name`); the agent did not read it in Selectors (decision 16). The reading unit
     keeps no callable class-token match: `by_class_name`'s match is inline in a closure
     (`reads.rs:454-506`), so this story writes its own, in `selector_reads.rs`, copied on
     purpose and not shared (decision 17), over the same `scan_for` charge. Names are compared
     after decision 15 has decoded them.
5. One exception with two kinds of message. The DOM Standard gives a caller of these members only
   `SyntaxError` (scope-match: "If selector is failure, then throw a "SyntaxError" DOMException",
   `dom.html:749`, `:11117`, `:11150`), and Selectors Level 4 section 3.9 says an agent treats as
   invalid every construct it has no usable support for. So every refusal of a text is a
   `SyntaxError` (name `SyntaxError`, code 12) and no member throws `NotSupportedError` for a
   selector. The two kinds differ by the message, which is surf's own text and is the record an
   agent reads. A message is made of fixed words and no text of the page:
   - Not valid: the text is not a selector list of Selectors Level 4's grammar. The message is
     `selector is not valid`.
   - Not supported: the text is a selector list of the grammar and uses a construct outside the
     subset, or reaches a bound of decision 6. The message is `selector is not supported by surf:
     ` and one fixed word for the construct or the bound: `pseudo-class`, `pseudo-element`,
     `sibling combinator`, `attribute matcher`, `attribute flag`, `namespace prefix`,
     `nesting selector`, `text over 4096 bytes`, `more than 512 simple selectors`.
   The deviation that stays is the subset itself: a browser runs these selectors. It is named
   beside the nine members (decision 11). The constructs outside the subset, valid and not
   supported: a pseudo-class (`:hover`, `:not(...)`, `:scope`, `:nth-child(...)`), a
   pseudo-element (`::before`), the combinators `+` and `~`, an attribute matcher other than `=`
   (`~=`, `|=`, `^=`, `$=`, `*=`), a flag after the value (`i`, `s`), a namespace prefix of `*` or
   an empty one (`*|a`, `|a`, `[*|a]`, `[|a]`) and the nesting selector `&` (the agent's read did
   not answer it: the implementor reads it, decision 16). Escapes and comments are in the
   subset (decision 15).
   - Not valid by the text read: `ns|a` and `[ns|a]` (a prefix that is an identifier is an
     undeclared namespace prefix: Selectors 3.9 and 3.8, and the DOM Standard says support for
     namespaces within selectors is not planned) and `a || b` (the draft of 10 November 2026
     has no `||`: its production is `<combinator> = '>' | '+' | '~'`; a later draft may add it,
     so the implementor reads the production again, decision 16).
   - The parser reads the whole text first, up to the bounds of decision 6. A text with any
     fault of the grammar is not valid whatever construct precedes it (`a:hover[` is not valid);
     the first construct outside the subset of a text without a fault is named. The two bounds
     are one rule: a text over 4,096 bytes is refused whatever it holds, and the 513th simple
     selector ends the parse whatever follows it, so a fault after it is not read (513 simple
     selectors then `[` get the message of the bound, not the one of an invalid selector; a
     construct outside the subset met before that selector is named first). An excluded
     pseudo-class or pseudo-element is recognised by its shape (a colon or two, an identifier,
     and an argument in balanced parentheses, skipped by token, so a string inside it is one
     token and a parenthesis in the string does not count); its name and argument are not
     validated, so `a:hovr` and `a:nth-child(x y z)` get the not-supported message where a
     browser says they are not valid: the name and the code are the same, the message differs,
     and the pseudo-class texts of Selectors were not read (decision 16). This is a second
     deviation, beside the subset: the second clause of the note of decision 11, tested by
     acceptance 4a, and held by the epic's deviation text since plan commit `4013bde`
     (decision 14). The end of the text closes what is open (decision 15): `a:not(b`, whose
     `(` the end closes, is a pseudo-class by shape, not supported, as `a:not(b)` is.
6. The bounds. No recursion: the parser is one loop, the matcher is loops over indexes and the
   walk keeps a stack of frames of its own (decision 17), so a hostile text or tree needs no
   stack beyond the frame. The selector text is at most 4,096 bytes of UTF-8 after the glue's
   copy, counted as given: an escape and a comment count their bytes. It holds at most 512
   simple selectors (a type, `*`, an id, a class or an attribute selector), counted after the
   parse: an escape is part of one identifier and a comment is nothing, so neither adds a simple
   selector. That also bounds the compound selectors, the complex selectors of a list and the
   combinators at 512. Nothing nests in the subset, so the nesting bound is 0: the one scan that
   balances parentheses (decision 5) is a counter. A text over 4,096 bytes is a `SyntaxError` of
   the not-supported message naming the bound (`text over 4096 bytes`), before any parse and
   whatever it holds, valid or not (the text may be a valid selector, which a browser would run:
   the message says surf does not support it, and a script is not told its selector is invalid);
   the 513th simple selector is the same kind of `SyntaxError` naming its bound (`more than 512
   simple selectors`): the parse ends where it meets that selector and reads nothing after it,
   so a later fault is not found, as the byte bound refuses a text whatever it holds (decision 5).
   The buffers the story's code makes, each with its bound and its capacity (a `Vec` that grows
   by doubling holds up to twice its length). None of them is on the engine's heap, so none is
   in `memory_bytes` (the engine's 64 MiB, which counts wrappers, host objects and collections:
   `docs/design/scripts.md:334-337 @ 83e6b8a` says the same of the bytes the host's string reads
   copy). Each is bounded by the number given here and, in steps, by its charge (decision 7):
   - The names an escape or a NUL decodes: `\0` (two bytes) and a literal U+0000 (one byte, made
     U+FFFD by decision 15) become three bytes, so a name is at most 3 times the bytes of its
     text. At most 12,288 bytes of names are held (3 x 4,096), and at most 12,288 more for the
     lower-case forms of the type and attribute names (decision 4: the fold keeps U+FFFD, so
     it keeps the length). Each name is a `String`: capacity at most 49,152 bytes in all.
   - The parsed list: at most 512 simple selectors, 512 compounds and 512 complex selectors,
     each a record of at most 128 bytes, in three `Vec`s of at most 512 records: capacity at
     most 393,216 bytes, and 442,368 with the names; under 524,288. It is made as the parser
     goes, and the parse's charge (decision 7) is made before each token it takes. No token
     list is built: the parser takes the lexer's tokens one at a time.
   - The walk's frames (decision 17): at most 513, each a node id and an index of 16 bytes,
     8,208 bytes, capacity at most 1,024 frames, 16,384 bytes. A walk copies no list of
     children.
   - The chain of an element's ancestors, read once for an element: at most 512 ids, 4,096
     bytes, capacity at most 4,096.
   - The memo of one complex selector at one element (decisions 7 and 8): `2 x compounds x
     chain` bytes, at most 2 x 512 x 512 = 524,288, charged before it is made, one alive at a
     time (a list makes and drops one memo for each complex selector in turn).
   - The result `Vec<NodeId>` of `querySelectorAll`: an id is pushed only for a node whose
     visit step was charged, so at most 1,048,575 ids; capacity at most 2^20 ids, 8,388,608
     bytes. While it grows by doubling the old buffer and the new one are both alive: at most
     4,194,304 + 8,388,608 = 12,582,912 bytes, the peak of this buffer. The glue then clones
     the list into the collection's data (`bindings.rs:1338`), a second buffer of up to
     8,388,608 bytes that is the glue's.
   - The glue's copy of the argument (the reading unit's, not changed here): the UTF-16 units
     of a String of at most 16,777,216 units, 33,554,432 bytes while the copy is made, and the
     UTF-8 `String` made from them, at most 3 bytes a unit, 50,331,648 bytes (its capacity is the
     glue's and is not read here), alive for the call. A text over 4,096 bytes is refused after
     this copy and before the slice reads it.
   The story's own buffers at the bounds add up to 8,388,608 + 524,288 + 524,288 + 16,384 +
   4,096 = 9,457,664 bytes, 13,651,968 at the result buffer's peak (the 4,194,304 of its old
   buffer more), and the glue's copy to up to 83,886,080 beside them while the copy is made.
   The report names these as outside the engine's budget.
7. What is charged and by whom, against the entry's 10,000,000 steps. The rules are the reading
   story's decision 20: one price for a byte, one step per `STRING_BYTES_A_STEP` bytes (16,
   `bindings.rs:1257 @ 83e6b8a`), for every byte the glue or the slice reads, compares, folds or
   copies, an argument's included; a step a piece beside its bytes (here a token of the
   selector, a node visited, a cell of the matcher, an ancestor, a class token); each charge
   before the work it pays; a walk whose end is not known charges as it goes, before it reads
   each node. `selector_reads.rs` writes no `16` and no `/ 16` of its own: it charges bytes
   through `crate::bindings::STRING_BYTES_A_STEP`, as the attributes story does. The base
   charges per 64 bytes (`bindings.rs:628`, `reads.rs:313`) until the reading story's price
   commit; this story's work starts after it (acceptance 1 shows it). The charges of this
   slice:
   - The glue: 1 step a call; the copy of the argument at one step per 16 bytes (at most
     3,145,728 steps for the longest String, 16,777,216 code units of at most 3 bytes,
     50,331,648 bytes); for a `NodeList`, 8 steps a result node (`COLLECTION_NODE_STEPS`) plus
     the wrappers on the engine's heap. The glue charges the copy and the slice charges the
     read of the same bytes: both, as two works (the reading unit charges
     `getElementsByClassName`'s argument in the slice beside the glue's copy,
     `reads.rs:462-463`); the two together are at most 257 steps a call more than the glue's
     alone.
   - The parse, before the work it pays, through `selector_reads` (decision 17): `1 + bytes /
     16` for the text within the bound, the bytes as given, escapes and comments included (257
     at the bound), charged before the lexer reads the first byte; one step a token, charged
     before the lexer takes it; and for each type name and each attribute name, one step per
     16 bytes of the decoded name (`name bytes / 16`), charged before the name is folded to
     ASCII lower case (decision 4), once. A token is one of CSS Syntax Level 3's tokens of a
     selector text: a run of whitespace, a comment, an identifier, a hash, a string, a
     delimiter, a bracket, a parenthesis, a comma. A text has at most 4,096 tokens, and its
     names at most 12,288 bytes, so a parse costs at most 257 + 4,096 + 768 = 5,121 steps. A
     parse that ends at a fault or a bound has paid for what it took. A text over the bound is
     refused before `selector_text`: the slice charges nothing for it.
   - The walk of `querySelector` and `querySelectorAll`: one step per node it visits, before the
     node is read, and no list of the tree or of a node's children is built or copied: the
     walk keeps frames, a parent and the index of its next child, at most 513 (decision 17).
     The rightmost compound of each complex selector, tested at each element (the gate), is
     the matcher's.
   - The matcher, for each complex selector of the list and each element it tests. Its
     algorithm and its price are one statement, so that a floor of acceptance equals a
     ceiling. Call the compounds `C1` to `Cm`, left to right, and the element's chain `P0`
     (the element) to `P(n-1)` (its outermost element ancestor), `n` at most 512. A compound
     of `s` simple selectors is tested in order and the test stops at the first simple
     selector that fails; one step is charged before each simple selector it tests: the first
     is the step of the caller (the gate's or the cell's, below) and each further one is
     `selector_reads::test_compound`'s own. So a compound test costs from 1 to `s` steps, and
     at least 1 a cell, before the bytes its simple selectors read (as below).
     1. The gate (`selector_reads::gate`): one step, then `Cm` is tested against `P0` (a compound
        test; a type name also costs one step per 16 bytes of the selector's type name for the
        form the element's namespace calls for, decision 4, charged before the compare; an id,
        class or attribute test reads through `selector_reads` as below). A complex selector of
        one compound is answered here. If the gate fails, nothing more is done for this
        selector at this element.
     2. The chain, read whole and once for an element (a later selector of the list at the
        same element uses it): `P1` onward through `next_ancestor`, one step before each read,
        until the read finds no element parent: the outermost element's read, which finds the
        document or a fragment, is charged like the others, so a chain of `n` elements costs
        `n` steps. Only then is the memo sized.
     3. The memo: one record of two bytes for each pair (compound `i`, position `p`): `yes`,
        the answer of `Ci` at `Pp` with everything left of it, and `some`, whether `yes` holds
        at `p` or at any later position (false past the chain, where there is no later
        position). It is `2 x m x n` bytes, charged `bytes / 16` steps before it is made
        (`selector_reads::memo`), made zeroed, dropped when the selector's answer is known; a
        list makes one memo for each complex selector, one after another, and never has two
        alive.
     4. The fill, with no recursion, rows `i` from 1 to `m` and, in a row, `p` from `n - 1`
        down to 0, every cell of the memo, through `selector_reads::cell`: one step before
        each cell is filled, and no more for a cell whose compound is not tested. If everything
        left of `Ci` can hold at `Pp` (`i` is 1; or `p + 1` is in the chain and the cell
        `(i - 1, p + 1)` is `yes` for a child combinator, `some` for a descendant one), `Ci` is
        tested against `Pp` (a compound test: the cell's step is its first simple selector's)
        and the cell is `yes` when it matches, `some` also when the cell `(i, p + 1)` is `some`
        (false past the chain); else the test is not run and the cell is no. The bytes a test
        reads (a type name, an attribute scan, a value, class tokens) are charged beside the
        steps, before the read, as below. A descendant step is one lookup of the `some` byte,
        a child step one lookup of the `yes` byte.
     5. The answer is the cell `(m, 0)`. The gate's test of `Cm` at `P0` is made once more
        by the fill and paid again: one more compound test.
     So a complex selector of `m > 1` compounds at an element that passed the gate costs `n`
     (the chain) + the cells + `2 x m x n / 16` (the memo) + the bytes the tests read, whether
     it matches or not. The cells cost `m x n` when each compound is one simple selector, and
     at most the text's simple selectors, 512, at each position when it is not (a pass over
     all the compounds at one position tests each simple selector once, at most), so they are
     at most `512 x n` whatever the split. Filling every cell is the choice: the cost is a
     function of the selector and the chain alone, apart from what the tests read of the
     element, so a floor of an acceptance equals the ceiling it is a floor of; a fill on
     demand would be cheaper for an ordinary page but cannot give the `some` lookup, and its
     floors would need a lower bound the algorithm only guarantees by reading the data. The
     price for an ordinary page is small: `div p` at a chain of 20 costs 20 + 40 + 5 = 65
     steps for a `p`, and nothing beyond the gate for an element that fails it.
   - An id, class or attribute test reads through `selector_reads`, over the items of
     `reads.rs` that both slices use (decision 17), at the price of the reading unit's scan.
     `selector_reads` charges one step per 16 bytes of the selector's attribute name for each
     attribute of the element's list (`attribute_count x (name bytes / 16)`, none for a name
     under 16 bytes however many attributes), before it calls the scan. The scan then charges
     as the reading unit has it: one step per 4 attributes of the list before the pass (none
     for a list of fewer than 4), one per 16 bytes of the element's value it reads or
     compares, before it is read (the compare reads no more than that value), and one per
     class token of a `class` value. In quirks mode the fold of a value is paid by the bytes'
     price. No existing charge of `reads.rs` changes.
   - How a charge that fails ends the member: `reads.rs`'s private `charge` answers a throw the
     engine does not deliver (the run is ending) and records no limit (reading decision 6);
     `charge` becomes `pub(super)` and `selector_reads` calls it, each function there that
     charges returns that `DomResult`, and `selectors.rs` passes it up with `?`.
   So the longest stretch of surf's own work between two charges is one test of one simple
   selector in one cell: the walk charges before it reads a node, the memo and the chain are
   charged before they are made, and a frame pops only after a node it holds was charged.
   Every end a script can reach through a selector is the engine's: `limit: steps-per-script`
   or `limit: memory`, or a throw.
8. The worst cases in numbers, which the story's tests hold. Walk: a tree of the node limit
   (1,048,576 nodes) costs at most 2 steps per node to search for a single selector that matches
   nothing (the visit, and the gate at an element), 2,097,152 at the limit: 4 calls fit in an
   entry and the 5th ends it (10,000,000 / 2,097,152 is 4.77). A loop of such calls ends
   `limit: steps-per-script` in at most 5 turns over a tree at the limit. A list of `k` complex
   selectors adds `k` gate steps an element. The walk's frames add no step: it copies no list
   (decision 17).
   Matcher: an element has at most 511 element ancestors, so its chain holds at most 512
   elements. For one element at the bounds, a text of 512 compounds (`m` = 512, one simple
   selector each), a chain of 512 (`n` = 512) and the names at their bound (12,288 bytes, at
   most 768 steps in one pass over every compound), the steps are:
   - 1 for the visit and 1 for the gate;
   - 768 for the type names of the gate (the pass over the rightmost compound alone cannot
     exceed the pass over all of them);
   - 512 for the chain (`n`: the read that finds no element parent is charged);
   - the memo, 2 x 512 x 512 = 524,288 bytes, charged 524,288 / 16 = 32,768 steps before it is
     made;
   - 512 x 512 = 262,144 for the cells;
   - 768 x 512 = 393,216 for the type names, one pass over every compound at each of the 512
     positions;
   in all 1 + 1 + 768 + 512 + 32,768 + 262,144 + 393,216 = 689,410 steps for elements without
   attributes (an attribute test adds the scan of decision 7, up to 256 steps a test on a list
   of 1,024 attributes; the engine's limit ends such a text, charged as it goes). A text of
   fewer compounds with more simple selectors in each costs less: the cells' steps are bounded
   by the 512 simple selectors a text holds, `512 x n` at most, which is the figure above, and
   the memo is smaller. A list of `k` complex selectors whose compounds add up to 512 shares
   the same memo bytes and cells and adds `k - 1` gates, at most 511: 689,921 steps. That is a
   ceiling no text reaches in full (the terms do not all reach their maxima in one text), for
   elements without attributes, and the entry holds 14 elements of the ceiling (14 x 689,921 =
   9,658,894) and ends in the 15th (15 x 689,921 = 10,348,815).
   The gate is why the memo is made late: for an element that fails the gate, nothing but the
   visit and the gate is paid, and a selector of one compound pays no memo (acceptance 15g).
   `closest` tests at most 512 candidates (`this` and 511 ancestors), each its own gate,
   chain and memo, one memo alive at a time. A candidate whose chain has `n` elements, for 512
   compounds all matching its gate, costs 1 (the step that reaches it, none for `this`) + 1
   (its gate) + `n` (its chain) + 64 n (the memo, 2 x 512 x n / 16) + 512 n (the cells) = 577 n
   + 2 steps before the names, for elements without attributes. The sum over the chains 512
   down to 1, with the step that finds no element parent past the outermost, is 577 x 131,328
   + 1,024 = 75,777,280 steps, 7.6 times the entry: the entry ends in the 36th candidate, or
   in the 35th when the glue and the parse of a long text have used more than 3,406 steps
   (35 candidates cost 577 x (512 x 35 - 595) + 69 = 9,996,594: 35 gates and 34 reaching
   steps), `limit: steps-per-script`, and the memos are never more than 524,288 bytes alive.
   Result: a `NodeList` costs 8 steps a node in the glue, charged for the whole list in one
   call before the collection object is made (`bindings.rs:1333`: the count times
   `COLLECTION_NODE_STEPS`), then the wrappers one by one. For `querySelectorAll("*")` a node
   costs 2 steps in the slice and 8 in the glue, 10 in all, so a list of 1,048,576 elements
   costs 10,485,760 steps and the entry's step limit ends the call in the glue's charge,
   before a wrapper is made. The heap ends the call (`limit: memory`, while it wraps, at about
   123,000 nodes, Context) only for a list of at least about 123,000 and below about
   1,000,000 nodes (from there the step limit ends the call first). The slice's own
   `Vec<NodeId>` is at most 8 MiB of capacity (decision 6), each id
   paid by its node's visit step. In the test profile a script runs 1,372,567 to 1,405,663
   steps per second (`docs/design/scripts.md:273-274 @ a14e3b0`, the same lines at `83e6b8a`;
   `:369 @ f463e24`, a commit not on `main`, holds the same text), so 10,000,000 steps take
   about 7.4 s there (7.358 s measured, `:271-272`) and the wall clock of 5 s
   (`docs/design/scripts.md:312 @ a14e3b0`; `:302` is the `steps_per_entry` row) ends a script
   first. A test gives limits of its own and asserts no time (acceptance 12).
9. The collections. Three members return a collection, the three `querySelectorAll`, a
   `NodeList` that is static as the standard says (`dom.html:5242-5243`), so the snapshot is no
   deviation. It is a `[NewObject]`, not a `[SameObject]` row: it has no variant of `SameObject`
   (`bindings.rs:2051`, four variants), the glue never asks `Wrap` for a kept object, and every
   call makes a new host object (`querySelectorAll("p") !== querySelectorAll("p")`). The object
   is not refreshed after a write and nothing needs the tree's write counter; a script that holds
   a list after a write reads the old nodes, as the standard says. The two members of the list
   that cost steps, `item` and `length`, and its iteration are the reading unit's, not changed
   here. The three `querySelector` and `closest` return one wrapper, which `Wrap` finds or
   makes.
10. Writes and limits. No selector member writes the tree: none raises `Tree::writes()`, none
    calls `charge_node`, none answers `DomEnd::Limit`, none calls `end_with_limit`
    (`bindings.rs:284, 512`), and no `WriteError` reaches them, so none becomes a
    `DOMException` here. There is no write of this slice to place or to charge (the reading
    story's decision 20 on writes does not apply), and the source test of decision 17 holds
    `tree_mut()` out of `selectors.rs` and of `selector_reads.rs`. Every case asserts the
    record of a limit is `None`
    through the helper of acceptance 1 (the reading unit's `recorded`,
    `dom_reading.rs:160-166`), except the cases that assert a limit's end.
11. The deviations are named beside the members, as the epic says (`epic:761 @ 929269e`: "each
    named beside its member in the selection"). Each of the nine rows gains one note of two
    clauses, `deviation: a valid selector outside the subset, and a selector over the bounds,
    throws a SyntaxError whose message says surf does not support it, where a browser runs it;
    deviation: a pseudo-class or pseudo-element is recognised by its shape, so an unknown name
    or a bad argument gets that message, not the one of an invalid selector`, and
    `crates/script/src/bindings.rs` is regenerated (the note is part of each member's doc
    comment, as at `bindings.rs:2512`). Both clauses are in the epic's deviation text, which
    holds them since plan commit `4013bde` (`epic:134-138`, `:743-750`, `:762-770 @ 929269e`;
    decision 14). The first clause is the epic's own deviation, the subset. The second clause is
    this story's reading, kept as a named deviation of its own: surf calls `a:hovr` and
    `a:nth-child(x y z)` not supported where a browser calls them not valid, because it does
    not validate the name or the argument of an excluded pseudo-class (decision 5); acceptance
    4a tests both texts, and acceptance 9 tests that the nine rows carry both clauses. The
    generator does not change: `git diff --stat "$(git merge-base main HEAD)"..HEAD` names
    nothing of `crates/webidl/src/`. No line of `crates/webidl/selection.txt` says
    `NotSupportedError` for a selector at `83e6b8a` (`git grep -n -i 'NotSupportedError'
    83e6b8a -- crates/webidl/selection.txt` prints nothing); the nine rows carry no note, so the
    note above is new and replaces no text.
12. The tests that write the tree. A case that needs a write between two reads (a static list
    after a write, a removed subtree, a fragment) builds its `Slices` with `dom::slices(info)`
    (`mod.rs:49`) and replaces the public field `mutation` with a test slice of this module, as
    the reading unit's module does (`dom_reading.rs:73-133`: `TestMutation`, private, not
    shared). This module holds its own copy, a few lines: `Node.appendChild` through
    `DomCx::tree_mut()` and `Tree::append_child` (`tree.rs:1098`), `Node.removeChild` through
    `Tree::remove_node` (`:1104`) and `Document.createDocumentFragment` through
    `Tree::create_document_fragment`, which a case needs to reach a fragment (no script can
    reach a template's contents: no `content` member is selected). `story:scripts-dom-mutation`
    writes the real slice.
13. A test that reads one of the nine as a member that still throws changes, in this story, to
    another member that no story turns on: `Element.attachShadow`, whose row is
    `crates/webidl/selection.txt:107`, slice `throws`, reason `shadow trees`; its call answers
    `NotSupportedError` with the message `Element.attachShadow is not supported: shadow trees`
    (`bindings.rs:2042-2047`, `:4949-4959`). The one such test is `other_slices_still_throw`
    (`crates/script/tests/dom_reading.rs:2399-2432`): its third line, `:2426`, reads
    `el.querySelector("p")` and becomes `el.attachShadow({})` with that message. No case of
    `crates/script/tests/bindings.rs` and none of the `adversary` modules reads one of the nine
    as throwing (`git grep -n -F 'Element.querySelector is not supported' 83e6b8a --
    crates/script/tests` prints that one line, `dom_reading.rs:2426`, and nothing after the
    story). The attributes story at plan commit `9e5b717` changes the line before it (`:2425`,
    `el.getAttribute("id")`, to `el.getAttributeNode("id")`) and says each of the three stories
    changes its own line: the three adjacent lines are `document.appendChild(el)` (`:2424`),
    `el.getAttribute("id")` and `el.querySelector("p")`.
14. The plan's text this story made false was corrected before the story was built, as plan
    work and not as this story's code. At plan commit `929269e` the epic
    (`.engineering/planning/epic/scripts-in-a-page.md`) says what the story says, and nothing
    of it is left to edit: the deviation line (`:134-138`) and the list of deviations
    (`:762-770`) say a `SyntaxError` whose message says surf does not support a selector
    outside the subset or over its bounds, and that a pseudo-class or pseudo-element
    recognised by its shape gets the same message also where a browser says not valid (the
    second clause of decision 11); candidate 11 (`:478-481`) says 9 members, a `SyntaxError`,
    and the files `selectors.rs` and `selector_reads.rs`; the slice "Selectors, 9"
    (`:742-750`) says 9, `scripts-dom-selectors`, the DOM Standard's `SyntaxError` and both
    clauses; and its section "Corrections of 2026-11-25" has two bullets for them (`:1001` for
    the error kind, `:1002` for the second deviation). The first clause was corrected at plan
    commit `0759016` and the second is in the epic since plan commit `4013bde`. The epic's
    corrections list also says what the other stories write beside their own files (`:1000`
    for the attributes story, `:1007-1018` for the mutation story); for this story it holds no
    such bullet, and a bullet for the files of the Change that candidate 11 does not name is
    plan work too, made in the plan store and not by this story. The attributes story at
    `9e5b717` asserts no member of another slice in its acceptance 8 (it says only which
    line of `other_slices_still_throw` each story changes, as its Change and Placement do), so
    it needs no edit.
15. Lexical rules, as CSS Syntax Level 3 defines them. The agent read the draft of 20 October
    2026 (sections 2.2, 3, 3.3, 4.2, 4.3.1 to 4.3.5, 4.3.7, 4.3.8, 4.3.12, 5.5.7 to 5.5.10,
    8.1) and found every line below in agreement, except the newline, which the first draft
    left out and which is the first bullet. Where a line rests on a part not read (decision
    16 lists them: how a block closed by the end meets the grammar's `]`, and 5.4), the text
    read wins where it differs: the implementor checks each such line against the text, reports
    the section and both outcomes, and corrects the line in the same change. The rules apply to
    the text as given.
    - A newline. CSS Syntax 3.3 (the preprocessing) replaces each CR, each FF and each CR LF pair
      by one LF before the text is tokenized: CR, FF and CRLF are each one newline. That holds
      for a bad string, for the continuation after a backslash and for the one whitespace
      consumed after a hex escape (below): a CRLF is one newline, so it is consumed whole.
    - An escape, in an identifier and in a string: a backslash, then up to six hex digits and
      one optional whitespace code point that is consumed with them, is the code point of the
      digits; the value 0, a surrogate or a value over U+10FFFF is U+FFFD; a backslash then any
      other code point that is not a newline is that code point; a backslash at the end of the
      text is U+FFFD in an identifier and nothing in a string. A U+0000 in the text is U+FFFD.
    - A backslash before a newline is not an escape in an identifier (it ends the identifier,
      and the text then has a fault) and is a line continuation in a string (nothing).
    - An identifier starts with a digit only through an escape: `#\31 23` is the id `123` and
      `#123` is a `SyntaxError`. An escaped colon is a name character: `#a\:b`, `.md\:flex`
      (the form real pages pass, the output of `CSS.escape`). Six hex digits and a space are
      one escape: `#\000061 b` is the id `ab`, where `#\61  b` (two spaces) is `#a` then the
      descendant combinator then `b`. `\0` and an escaped surrogate (`#\d800`) and a value over
      U+10FFFF (`#\110000`) are the id U+FFFD.
    - A string is quoted by `"` or `'`: `[a="x\"y"]` is the value `x"y`; a backslash then a
      newline inside it is a continuation (the value `xy`).
    - The end of the text inside a string is a parse error of CSS Syntax and is not a fault of
      the selector: the string is what was read up to the end (CSS Syntax 4.3.5: "This is a
      parse error. Return the <string-token>."). So `[a="b` has the value `b`, and `[a="x]` has
      the value `x]`, the string taking the `]`. Only an unescaped newline inside a string is a
      bad string (4.3.5: reconsume, create a bad-string-token), and the selector then has a
      fault: Selectors 6.1 wants a string or an identifier and 3.9 calls the selector invalid,
      so the message is the not-valid one.
    - The end of the text closes an open `[` or `(` block, as CSS Syntax does for a block at the
      end of its input (5.5.9, 2.2: "everything is automatically closed", which "doesn't make
      them invalid"). So `[a="b` matches as `[a="b"]` and `[a=b` matches as `[a=b]`, and
      neither is an error. How a block closed by the end meets the grammar's `]` is not stated
      by Selectors 16 and 17.1, and CSS Syntax 5.4 was not read: this rule is the story's
      reading, which the implementor checks (decision 16). A block that is empty or lacks what
      the grammar needs is still a fault: `a[` (closed as `a[]`), `[a=` (closed as `[a=]`) and
      `a:hover[` are not valid. An open `(` of an excluded pseudo-class is closed the same
      way (decision 5). A `)` or `]` with nothing open is a fault: `a b)` is not valid.
    - An attribute value is `[a=v]` with an identifier or `[a="v"]` and `[a='v']` with a string.
    - A comment `/* ... */` is dropped and what is around it stays: `a /* c */ b` is the
      descendant combinator and `a /* c */ > /* d */ b` the child combinator; an unterminated
      comment runs to the end of the text, so `a /* b` is the selector `a`.
    - Against the bounds of decision 6: an escape and a comment count their bytes in the 4,096
      and are charged by the parse's `1 + bytes / 16` as given; the 512 simple selectors are
      counted after the parse, so `\61 ,` 512 times then `\61` (513 selectors, 2,563 bytes) is
      over the second bound and a text of 4,097 bytes of comment around one `a` is over the
      first.
16. The case of attribute values in an HTML document, held as a table, and the four texts. For
    an element in the HTML namespace, `[a=v]` compares the value ASCII case-insensitively when
    the attribute's name is in the list of the HTML Standard's section "Case-sensitivity of
    selectors", and exactly otherwise; for an element outside the HTML namespace the value is
    compared exactly whatever the name, as that section says. The attribute's name here is the
    lower-case form of the selector's name (decision 4), which an HTML element's attribute must
    equal to be found. The list is one sorted table in `selectors.rs`, 46 names, in the text's
    order, which is the ASCII order: `accept`, `accept-charset`, `align`, `alink`, `axis`,
    `bgcolor`, `charset`, `checked`, `clear`, `codetype`, `color`, `compact`, `declare`, `defer`,
    `dir`, `direction`, `disabled`, `enctype`, `face`, `frame`, `hreflang`, `http-equiv`, `lang`,
    `language`, `link`, `media`, `method`, `multiple`, `nohref`, `noresize`, `noshade`, `nowrap`,
    `readonly`, `rel`, `rev`, `rules`, `scope`, `scrolling`, `selected`, `shape`, `target`,
    `text`, `type`, `valign`, `valuetype`, `vlink`. An agent took the list from the page of the
    HTML Standard of 25 November 2026 in two reads that gave the same names and order. The
    implementor compares the list with the page once and reports a difference. The table's
    length is 46 and its names are checked by a test against a copy of the list written in the
    test (two typings of one list). The flags `i` and `s` of `[a=v i]` stay out (decision 5);
    the flag identifiers themselves are ASCII case-insensitive (Selectors 6.3), so `[a=b I]` is
    a flag and not supported.
    The four texts, each with the address, the text's date, and what the agent read and did not
    read. The agent read through a fetch tool that paraphrases sentences over 125 characters, so
    the quoted words in this story are at most 125 characters as it returned them; the
    implementor reads each text once itself, when the work starts, and these are the only
    network requests of the work: each goes into its own new empty directory under the tree's
    ignored `.out/`, nothing is run from it, nothing is vendored, and no test makes a request.
    The implementor says in the report if an address has moved or a text differs.
    - Selectors Level 4: `https://drafts.csswg.org/selectors-4/` (the address `dom.html:744`
      links, `#parse-a-selector` and `#match-a-selector-against-a-tree`; the bibliography,
      `dom.html:17374-17375`, gives `https://drafts.csswg.org/selectors/`). Opened: the Editor's
      Draft of 10 November 2026; read: sections 3.3, 3.7, 3.9, 4.1, 5, 6.1, 6.3, 6.4, 6.6, 6.7,
      14 (the list only), 16, 17.1, 17.3 and 17.5. Not read, the implementor's: 3.4 to 3.6, the
      texts of the pseudo-classes (for `a:` and an open `(`), the nesting selector `&`, whether
      section 17.3's parent step counts a template's contents fragment as a parent, how a block
      closed by the end of the text meets the grammar's `]` (16 and 17.1 do not say), class
      tokens split at ASCII whitespace, whitespace around the whole text, and the production of
      `<combinator>` again (the draft has no `||`).
    - CSS Syntax Level 3: `https://drafts.csswg.org/css-syntax-3/` (the same text stands at
      `https://www.w3.org/TR/css-syntax-3/`). Opened: the Editor's Draft of 20 October 2026;
      read: sections 2.2, 3, 3.3, 4.2, 4.3.1 to 4.3.5, 4.3.7, 4.3.8, 4.3.12, 5.5.7 to 5.5.10
      and 8.1. Not read, the implementor's: 5.4 to 5.4.9 (the algorithms that parse according
      to a grammar: how a block closed by the end of the input reaches the grammar).
    - The HTML Standard, section 4.16.2 "Case-sensitivity of selectors":
      `https://html.spec.whatwg.org/multipage/semantics-other.html#case-sensitivity-of-selectors`.
      Opened: "Living Standard, Last Updated 25 November 2026"; read: the section. No commit
      snapshot address of the section came back: the implementor reads the section at a commit
      snapshot address (the repository's snapshot of the page's IDL is `crates/webidl/data/
      whatwg/SOURCE`, file `html-idl.html`, commit
      `546d834701d171e3b164da8463d62878deb1e954`, address
      `https://html.spec.whatwg.org/commit-snapshots/546d834701d171e3b164da8463d62878deb1e954/`;
      the implementor takes a newer commit's if the section moved) and cites the commit.
    - The DOM Standard: `https://dom.spec.whatwg.org/`. Opened: "Living Standard, Last Updated
      25 November 2026", snapshot `ddfb52fde44069e2f77b839bdf114eeefc87e3f7`; read: 1.3 (match a
      selector, scope-match), 4.2.6, 4.2.10 and 4.9 (`closest`). Not read: the last 9,918
      characters of the page. The repository's own copy, which this story cites as `dom.html`,
      is the snapshot of 15 November 2026, commit `e4f4f4f0ace83fe6c52ab68e047351ce7f2dcd69`
      (`crates/webidl/data/whatwg/SOURCE`): the implementor compares the steps of scope-match
      and of `closest` in the two once.
17. Where the reads live: one file for each slice's reads. The functions of this story go into
    a new file, `crates/script/src/dom/selector_reads.rs`, declared in
    `crates/script/src/dom/mod.rs` by `mod selector_reads;` and `pub use
    selector_reads::Frames;` (the attributes story's `attribute_reads.rs` is a private `mod`
    only; the one public item here is the walk's stack, so that a test of `tests/` can count
    its frames: the crate's own test target is off, `crates/script/Cargo.toml` `[lib] test =
    false`). The file charges before the work and holds every call of `tree()` this slice
    makes. `selectors.rs` holds the lexer, the parser, the types of the parsed list
    (`Compound`, a complex selector, the list), the table of decision 16, the matcher over
    those types and the nine methods, and holds no call of `tree()`, no `tree_mut()`, no
    `charge(` and no `charge_steps(`; `selector_reads.rs` holds no `tree_mut()` and no
    `charge_steps(` (it charges through `charge` of `reads.rs`). The two modules use each
    other: `selector_reads.rs` takes the parser's types (`Compound` and the attribute record)
    from `selectors.rs`, and `selectors.rs` calls the functions of `selector_reads.rs`; the
    test of strings below does not see this and does not need to. A test reads both sources
    with `include_str!` and fails when one of those strings is added to the file that must not
    hold it (the form of `dom_reading_cost.rs:593-598`).
    `reads.rs` changes by visibility and by one widening, and in no other way: `pub(super)` on
    what both slices use, `charge`, `ATTRIBUTES_A_STEP` and `attribute_count` (the attributes
    story makes the same three `pub(super)`) and the attribute scan with its helpers, `Reads`,
    `scan_for`, `positions` and `value_at`; and the scan's name widened to `&str`: `Reads`
    gains a lifetime and its `Name` and `Value` take a `&'a str` where they take a
    `&'static str` at `83e6b8a`. No existing function changes its body, its charge or its
    answer. The functions of `selector_reads.rs`, each `pub(super)` and each with a doc comment
    that says what it charges (the names are the plan's and may change with a reason in the
    report); `reads::quirks` (no charge) is the one other function the members use:
    - `selector_text(cx, bytes)`: `1 + bytes / STRING_BYTES_A_STEP`, before the lexer reads the
      text.
    - `selector_token(cx)`: 1 step, before the lexer takes a token.
    - `selector_name(cx, bytes)`: `bytes / STRING_BYTES_A_STEP`, before a type name or an
      attribute name is folded to ASCII lower case (decision 4); once for each name, in the
      parse.
    - `Frames` (the one `pub` item): the walk's stack of frames, each a parent and the index of
      its next child, and no more: `Frames::new(scope)`, `Frames::next(&mut self, tree:
      &Tree) -> Option<NodeId>`, `len` and `most` (the most frames it has held). `next` takes
      the next child of the top frame through `Tree::children(parent)[index]`, a slice read in
      constant time. `Tree` has no link to a sibling or to a first child: its reads of a
      node's neighbours are `parent` (`tree.rs:624 @ 83e6b8a`) and `children` (`:665`), and
      finding a sibling from a node would search its parent's list, so the walk keeps the
      index itself. A node that has children is pushed as a frame; an exhausted frame is
      popped. The frames are at most the tree's depth plus 1: the root and at most 512
      element levels, 513. No child list is copied, and `Frames` charges nothing and holds no
      `cx`.
    - `descendant_elements(cx, scope, visit)`: the elements under `scope`, `scope` left out, in
      tree order; each call of `Frames::next` is preceded by one step, so a node is charged
      before it is read (the step that finds the end is paid too: at most one step more for
      the whole walk, and a pop never reads a node); `visit(cx, element)` is the matcher of
      `selectors.rs` and gets `cx`, not a `&Tree`, so `descendant_elements` takes `cx.tree()`
      afresh for each `next`; the walk goes on until `visit` says stop. The reading unit's
      `find` (`reads.rs:349-366`) copies a node's children before it charges any of them; that
      is the reading unit's to change, in its own fix round, and this story does not edit it.
    - `next_ancestor(cx, node)`: the parent of `node` when it is an element, else none; one
      step, before the read, so the read that finds no element parent is charged too (a chain
      of `n` elements costs `n` steps, decision 7).
    - `memo(cx, compounds, chain)`: a `Vec<u8>` of `2 x compounds x chain` zeroed bytes;
      `bytes / STRING_BYTES_A_STEP` steps, before it is allocated (decision 7).
    - `gate(cx, node, compound, quirks)`: one step, then `test_compound` of the rightmost
      compound at the element itself: the step the matcher pays for each element and each
      complex selector before anything else.
    - `cell(cx, node, compound, quirks, left)`: one step; when `left` holds, then
      `test_compound` and its answer, else no. The one step before each cell of the fill.
    - `test_compound(cx, node, compound, quirks)`: whether the element `node` matches the
      compound; the simple selectors in order, stopping at the first that fails: one step
      before each simple selector after the first (the first one's step is the gate's or the
      cell's), and for a type name one step per 16 bytes of the name, before the compare, then
      the id, class and attribute tests below.
    - `has_class`, `has_id`, `attribute_equals` and `has_attribute`, called by `test_compound`:
      each charges `attribute_count x (name bytes / 16)` for the selector's attribute name (the
      class and id names are the fixed `class` and `id`: none), then calls `scan_for` for its
      charge of one step per 4 attributes and of the value's bytes before the compare, and one
      step per class token before the first token is looked up; they use `positions` and
      `value_at` (`reads.rs:267-299`) to find and read the value. `attribute_equals` takes the
      attribute record of the parse, which holds the name as decoded, its lower-case form and
      a flag, set by the parse when the lower-case form is in the table of decision 16: the
      table is in `selectors.rs` and the compare, case-insensitive or exact by the flag, is
      here.
    Three rules of `reads.rs` are copied on purpose, because sharing them would change the body
    of a function the reading unit built and this story changes `reads.rs` by visibility only:
    the split of a `class` value into tokens and the fold in quirks mode (`has_class` and the
    closure of `by_class_name`, `reads.rs:454-506`); the type-name rule, a name lower-cased for
    an HTML element and exact otherwise (`test_compound` and `by_tag_name`,
    `reads.rs:395-410`); and the step to an element parent (`next_ancestor` and the private
    `parent_element_in`, `reads.rs:578`, which has no charge). Each copy's doc comment names
    the place it copies, and the place it copies is not edited. A later story may share them.
    `reads.rs` is also written by the stories beside this one (Placement): each makes its own
    visibility changes and adds its functions to its own file.
18. Nothing is kept. `SurfSelectors` stays a unit struct with no field: no cache of parsed
    selectors, no table, no kept collection, no answer remembered from one call to the next
    (a cache is out of scope). So no write of the tree changes anything this slice holds, the
    refill rule of the reading story's decisions 17 and 18 (put back on a throw, the record of a
    cut refill on an end of the run) has nothing to apply to, and `DomState::swap_tree`, which
    `story:scripts-parse-run` adds, is nothing to this slice. None of the nine rows is a
    `[SameObject]` attribute: the generated enum `SameObject` keeps its four variants, and the
    generator's refusal of a `[SameObject]` row it does not hold never meets one of the nine.
19. The cost module. A module `cost` inside `crates/script/tests/dom_selectors.rs` (no new file,
    no new line of `tests/main.rs`; its few helper lines, `call`, `report` and the floor check,
    follow `dom_reading_cost.rs:136-190`, which are private there) holds one case for each
    member that reads more than its own node, on its worst shape, and each case asserts a floor
    of steps from decisions 7 and 8 and no time (the folder's rule: a test bounds a time only
    by a wait for the event with a limit at least 10 times the time it takes alone, and proves
    a cost by a count, `AGENTS.md:1966 @ a14e3b0`). The members are all nine. Each case prints
    its steps. A case over 5 s is named with its time in the report. The shapes are in
    acceptance 17.

## Change

- `crates/script/src/dom/selectors.rs`: the lexer (escapes, strings, comments, newlines:
  decision 15), the parser of the subset, the matcher over the parsed list with its memo and
  its fill (decision 7), the nine methods of `SurfSelectors` (decisions 1 to 10), the table of
  the 46 HTML attribute names of decision 16, the grammar in its head comment. It holds no call
  of the tree and no charge. `SurfSelectors` stays a unit struct.
- `crates/script/src/dom/selector_reads.rs` (new): the functions of decision 17, which charge
  before the work and hold the tree calls, and `Frames`, the walk's stack.
  `crates/script/src/dom/mod.rs`: two lines, `mod selector_reads;` and `pub use
  selector_reads::Frames;`.
- `crates/script/src/dom/reads.rs`: visibility only (`pub(super)` on `charge`,
  `ATTRIBUTES_A_STEP`, `attribute_count`, `Reads`, `scan_for`, `positions`, `value_at`) and the
  scan's name widened to `&str`; no function changes its body, its charge or its answer.
- `crates/script/tests/dom_selectors.rs`: the module `dom_selectors` (declared at
  `tests/main.rs:21`; `main.rs` is not edited): `Case { row, expression, expected }`, `const
  CASES` and the helper `run_case` as `dom_reading.rs:252-268` and `:841-863` have them (private
  there, so copied), the test `mutation` slice (decision 12), the module `cost` (decision 19),
  the cases of acceptances 13 and 14, the source test of decision 17, the table's test with
  its own copy of the list, the test of `Frames` (acceptance 15i) and the test
  `the_rows_carry_the_note` (acceptance 9).
- `crates/script/tests/dom_reading.rs`: the line of decision 13 (`:2426`).
- `crates/webidl/selection.txt`: the note of decision 11 on the nine rows;
  `crates/script/src/bindings.rs`: regenerated with `slot cargo run -p surf-webidl --bin
  bindings`; its header's digest of `selection.txt` and its own last line change, and nothing
  else but the nine doc comments.
- `docs/design/scripts.md`: one row of the table of Q4 (`:300-320`) for the selector's bounds
  (text, simple selectors, who charges), placed after the row `parser: attributes per element`
  (`:316 @ 83e6b8a`), away from the rows the stories beside this one edit (Placement), with the
  value marked "chosen", naming `story:scripts-dom-selectors`.
- `AGENTS.md`: the entry of `crates/script` (`AGENTS.md:365-369`: it names the slice `reading`
  and its story, the dependencies including `surf-url`, and `29 tests` with per-file counts
  of three files, which the reading story's last commit replaces by the log's): this story names
  the slice `selectors` and cites `story:scripts-dom-selectors` beside it, as the entry cites
  `story:scripts-dom-reading` today (`AGENTS.md:367 @ 83e6b8a`), and sets the count of tests
  from the log of its own tip.
- In the last commit, after merging `main`, from one gate log, the public surfaces of
  acceptance 10: `README.md`, `docs/badges/make.sh` (edit `:38`, then run `sh
  docs/badges/make.sh .`: it writes `docs/badges/tests.svg`, which is its output and not
  edited by hand), `website/docs/how-it-is-built.md`, `website/docs/status.md` and
  `website/docs/what-works.md`. By the epic's rule (`epic:955-961 @ 929269e`) these are not in
  the story's recorded scope: `README.md`, `docs/badges` and `how-it-is-built.md` never are,
  and `status.md` and `what-works.md` are only where a story changes another sentence of
  theirs, which this story does not (it changes their test counts, `status.md:31` and
  `what-works.md:17`, and nothing else).

## Acceptance

Every `git grep` below is run at the tip of the work; the report records what each prints. A
`git diff` names its base: `$(git merge-base main HEAD)..HEAD`, which is the reading unit's
merge commit or a later one, because acceptance 1a holds the work until the reading unit is on
`main`.

1. The base, the rows and the table.
   a. The base. Before any change, and again at the tip: `git ls-files --error-unmatch
      crates/script/src/dom/reads.rs` exits 0 (the reading unit's file is there), and `git
      grep -n -F '/ 64' -- crates/script/src/dom/reads.rs` and `git grep -n -F 'units / 64'
      -- crates/script/src/bindings.rs` print nothing (the reading unit's price commit, one
      step per 16 bytes, is on the base). Else the work waits for it: the floors of 15d, 15e
      and 17a to 17h rest on that price. Today the gate fails: at `83e6b8a` the file is there,
      the first `git grep` prints 16 lines (`reads.rs:135` to `:1249`, among them `:313`) and
      the second prints `bindings.rs:628`; at `a14e3b0` the file is not there (the first
      `git grep` prints nothing for that reason) and the second prints `bindings.rs:561`.
   b. `dom_selectors::cases_are_the_rows` reads `crates/webidl/selection.txt` as `selection()`
      does (`crates/script/tests/dom_reading.rs:880-894`, the same as
      `tests/bindings.rs:258-275`) and asserts that the set of row names of `const CASES`
      equals the 9 rows with the slice `selectors` (`Element.closest`, `Element.matches`,
      `Element.webkitMatchesSelector`, `Element.querySelector`, `Element.querySelectorAll`,
      `Document.querySelector`, `Document.querySelectorAll`, `DocumentFragment.querySelector`,
      `DocumentFragment.querySelectorAll`), and that each entry has a non-empty expression and
      expected value. Each entry runs through `run_case`: a runner of its own, `expression ===
      expected` thrown on a mismatch, the end `ScriptEnd::Ran`, and the recorded limit `None`
      (`runner.document().and_then(|dom| dom.limit())`). The entries together hold a named
      case for each production of the subset (type, universal, `#id`, `.class`, `[a]`, `[a=v]`,
      compound, descendant, child, list), each comparing the nodes found with what the
      standards say over a parsed tree, the section cited in the entry's comment.
2. The meaning of decision 3: `qS` and `qSA` answer descendants only and never `this`, in tree
   order; a combinator sees an ancestor outside `this`; `closest` returns `this` when it matches
   and `null` past the root; `matches` and `webkitMatchesSelector` agree; a subtree removed with
   `removeChild` and a fragment made with `createDocumentFragment` and filled with
   `appendChild` (the test slice of decision 12) are searched as their own trees, so a selector
   that needs an ancestor they no longer have finds nothing; for
   `<template><p id="t"></p></template>`, `document.querySelector("#t")` is `null` and
   `document.querySelectorAll("*")` holds no element of the template's contents; and
   `Element.prototype.closest.call(document, "a")` throws a `TypeError` (a member called on an
   object of another brand, epic acceptance 3: the glue's check, one case for the nine).
3. The case rules of decision 4. Each case is a row of `const CASES` over a parsed page unless
   it says Rust, where the case builds its tree with the tree's write surface before the script
   runs (`Tree::create_element_ns`, `Tree::append_child`, `Tree::set_attribute_ns`:
   `tree.rs:1027`, `:1098`, `:1557 @ 83e6b8a`) and gives it to the runner as `document(tree,
   info)` does.
   a. `DIV` finds the parser-made `div`; `[CLASS]` finds an element that has `class`.
   b. In a parser-made `<svg>`: `svg` finds the element and `SVG` finds nothing; `foreignObject`
      finds the `foreignObject` element and `foreignobject` finds nothing; `DIV` finds no
      element of an SVG or a MathML subtree.
   c. Rust: an element of the HTML namespace made with local name `DIV` and appended: neither
      `DIV` (folded to `div`, which is not `DIV`) nor `div` finds it. An element of the SVG
      namespace made with local name `DIV`: `DIV` finds it (compared as given) and `div` does
      not.
   d. Rust: a `p` given the attribute `FOO` with `set_attribute_ns(p, None, "FOO", "1")`: `[FOO]`
      finds nothing on it (the selector's name is folded to `foo`) and `[foo]` finds nothing
      either; a `p` with the parser-made attribute `foo` is found by `[foo]` and by `[FOO]`.
      Parser-made SVG `viewBox`: `[viewBox]` finds the element and `[viewbox]` does not.
   e. `[href]` does not match an `xlink:href`.
   f. `#A` and `.A` match `id="a"` and `class="a"` in a page without a doctype
      (`QuirksMode::Quirks`) and not in a page with `<!DOCTYPE html>` (`NoQuirks`) and not in one
      whose doctype gives limited-quirks mode (`LimitedQuirks`; the doctype is the XHTML 1.0
      Transitional one of `dom_reading.rs:1287-1292`, which `compatMode` reads as `CSS1Compat`).
   The case of attribute values is acceptance 14. The Rust cases name the types they use
   (`Namespace`), which the draft `story:html-open-namespace` changes (Placement).
4. The one exception and its two messages (decisions 5 and 6). Each refusal is a `DOMException`
   with name `SyntaxError` and code 12, and no case for a selector expects `NotSupportedError`:
   `git grep -n -F 'NotSupportedError' -- crates/script/tests/dom_selectors.rs` prints nothing,
   today and after (the one case of a member that throws is in `dom_reading.rs`, decision 13),
   and cases 4a and 4b are the proof that a refusal is the other name. Each member of the nine
   has one case of each kind (every one parses its argument first).
   a. Not supported: the message starts `selector is not supported by surf: ` and ends with the
      word of the construct, for `a:hover`, `a::before`, `:scope > a`, `a + b`, `a ~ b`,
      `[a^=b]`, `[a=b i]`, `[a=b s]`, `[a=b I]`, `*|a`, `|a`, `[*|a]`, `a:not(b)` and `a:not(b`
      (the `(` closed by the end of the text), and, as the second clause of decision 11, for
      `a:hovr` and `a:nth-child(x y z)`: each ends with the word `pseudo-class`, and the case's
      comment says a browser calls these two texts not valid.
   b. Not valid: the message is `selector is not valid`, for `""`, `a[`, `a:`, `a,`, `a >`,
      `> a`, `a:hover[`, `a b)`, `[a=`, `[a="x` newline `y"]`, `ns|a`, `[ns|a]` and `a || b`
      (the draft of 10 November 2026 has no `||`; the implementor reports it if the text read
      now has one).
   c. A test tells the two kinds apart: every message of 4a starts with the words of 4a and
      none of 4b does, and none of 4a equals the message of 4b. A message holds no text of the
      page: a case passes a not-valid text of 2,000 characters and a not-supported one with a
      pseudo-class of a 2,000-character name, and asserts each message equals its fixed words
      and is under `EXCEPTION_TEXT_BYTES`.
5. Bounds (decision 6). A text of 4,096 bytes of valid selectors runs; one of 4,097 bytes is a
   `SyntaxError` of the not-supported kind ending `text over 4096 bytes`, and the steps of the
   script that makes the call (the text made by an earlier one) are at most 1 + 4,097 / 16 +
   20 = 277: the glue's step and copy, 257, and 20 for the script's own few steps (the slice
   charges nothing before refusing; `selector_text` alone would add 257); 512 simple
   selectors run and 513 are a `SyntaxError` of the not-supported kind ending `more than 512
   simple selectors`, and so are 513 followed by `[`, where the bound ends the parse and the
   fault after it is not read; a text of 1,048,576 bytes, made by one script and
   passed by a second, is refused with the steps of the second at least 65,536 (the glue's copy
   at one step per 16 bytes: 1,048,576 / 16) and under 70,000, so the case fails at the old
   price of 64 bytes a step, which gives 16,384. The report records the steps measured.
6. Steps and worst cases (decisions 7 and 8). The implementor measures first and records, in the
   cases' comments, the steps of each case; the assertions below are the floors, the ceilings
   and the ends. The page of 6b, 6d and 17a is `<div>` 500 times and `<span></span>`, no
   doctype: the parser adds `html`, `head` and `body`, so the page has 504 elements and the
   `span` has a chain of 503.
   a. `document.querySelector("zzz")` over a document of 1,000 elements parsed from text raises
      `runner.steps()` by at least 2,000 (a visit and a gate an element); without the walk's
      step, or without the gate's, the rise is about 1,010.
   b. Over the page, `document.querySelector("x div div div div div div div div span")` (10
      compounds, no `x` exists) ends `Ran`, answers `null`, and its steps are at least 7,000
      and under 8,000: 504 visits, 504 gates, 503 for the chain, the memo of 2 x 10 x 503 =
      10,060 bytes, 628 steps, and 10 x 503 = 5,030 cells, 7,169 in all, and about 30 for the
      glue and the parse. A matcher that backtracks does not end. The selector `div div div div
      div div div div span` (9 compounds) answers the `span`, with the steps at least 6,500
      and under 8,000: 504 + 504 + 503 + 565 (9,054 bytes) + 4,527 cells = 6,603, and about 30.
   c. A document of 100,000 elements parsed from text (`document(tree, info)`, not built by the
      script, whose steps would count against the limit) and a script `var n = 0; for (;;) {
      n++; document.querySelector("zzz"); }` under a step limit of the test's own (1,000,000)
      ends `limit: steps-per-script`, and a later `runner.evaluate("n")` is at most 5 (a call
      costs at least 200,000 steps, so no more than 5 calls end in it). The case asserts the
      end and the count, no time.
   d. Over the page, `document.querySelector` of a selector of 512 simple selectors (511 times
      `div `, then `span`), under a step limit of the test's own (1,000,000), ends `Ran` (the
      chain holds 500 `div`, not 511), answers `null`, never a stack overflow, and its steps
      are at least 280,000 and under 300,000: 504 + 504 + 503 for the walk, the gates and the
      chain, the memo of 2 x 512 x 503 = 515,072 bytes, 32,192 steps, and 512 x 503 = 257,536
      cells, 291,239 in all, and 1,281 for the glue and the parse (129 + 1 + 128 + 1,023),
      292,520. Without the memo's charge the steps are about 260,300 and without the cells'
      steps about 35,000, so the floor fails for each. It asserts the end and the floor, no
      time.
   e. `for (;;) document.querySelectorAll("*")` over 1,000 elements under a memory limit of
      1 MiB ends `ScriptEnd::Limit` with `memory`, and a later `runner.evaluate("1 + 1")` ends
      `Ran`. By the arithmetic of the Context: the realm's 202,424 bytes, 1,003 wrappers of 423
      bytes, and about 121,700 bytes for each list of 1,003 nodes, so the fourth call passes
      1,048,576; a call costs about 10,000 steps, so no step limit is near. The case pins the
      end it measures.
   f. `closest` and `matches` over 500 nested `div` end `Ran` on the runner's thread, and so
      does `p.querySelectorAll("p > i")` over a `<p>` with 20,000 `<i>` siblings (its 20,000
      wrappers cost about 10.9 MB, 544 bytes a node as the reading unit measured, under the
      64 MiB heap).
   g. The case's end carries no recorded limit: every case that does not assert a limit asserts
      `runner.document().and_then(|dom| dom.limit())` is `None`.
7. Collections (decision 9): `querySelectorAll("p")` is a `NodeList` whose `length`, `item`, index
   properties, `for (x of list)` and `forEach` work (the reading unit's glue); `a.querySelectorAll
   ("p") !== a.querySelectorAll("p")`; after a write through the test slice of decision 12 an old
   list still holds the old nodes and a new call finds the new ones; the node a `querySelector`
   returns is the same object as the one a reading member returns for it
   (`document.querySelector("p") === document.getElementsByTagName("p")[0]`).
8. `document.querySelector("table")` on the tree `parse_document` builds from each of the two
   cases of `crates/html-conformance/data/wpt/html/syntax/parsing/resources/scripted_foster01.dat`
   (read with `include_str!`, no hook, the script text not run) finds its `table`. Running those
   scripts is `story:scripts-html-conformance`'s.
9. The files and the other tests. `slot task webidl-check` exits 0 (the header of `bindings.rs`
   holds the new digests); `slot task webidl-slow` regenerates `bindings.rs` byte for byte; the
   test `dom_selectors::the_rows_carry_the_note` reads the nine rows with the slice `selectors`
   from `selection.txt` (as `cases_are_the_rows` does, acceptance 1b) and asserts that each
   holds both clauses of the note of decision 11 (the words `a valid selector outside the
   subset` and `recognised by its shape`); `git diff --stat "$(git merge-base main HEAD)"..HEAD`
   names nothing under `crates/webidl/src/`; `git diff -U0 "$(git merge-base main
   HEAD)"..HEAD -- crates/script/src/bindings.rs` holds hunks only of the header's digest
   lines, the nine members' doc comments and the last line; the `TestSelectors` types of
   `tests/bindings.rs`, `adversary_3.rs`, `adversary_4.rs` and `adversary_5.rs` compile
   unchanged; a member of another slice still throws `NotSupportedError` naming the member
   (`Element.attachShadow`, the message of decision 13); `git grep -n -F 'Element.querySelector
   is not supported' -- crates/script/tests` prints nothing (it prints `dom_reading.rs:2426` at
   `83e6b8a`); `slot task check` exits 0 and `git diff --stat "$(git merge-base main
   HEAD)"..HEAD` names only the files the Change names (not the scope list: the Change also
   names the public surfaces that the epic keeps out of a story's recorded scope).
   The change of `reads.rs` is visibility and the lifetime only (decision 17). This command,
   run in zsh or bash at the tip, prints nothing and exits 0: the lines the diff removes equal
   the lines it adds once `pub(super)` and `<'a>` are taken out of the added lines, `Reads<'_>`
   is read as `Reads` and `&'a str` as `&'static str`, and both sides lose their spaces, newlines
   and commas, so that the layout does not count:
   ```
   b=$(git merge-base main HEAD); f=crates/script/src/dom/reads.rs
   diff <(git diff -U0 "$b"..HEAD -- $f | sed -n 's/^-\([^-]\)/\1/p' | tr -d ' \n,') \
        <(git diff -U0 "$b"..HEAD -- $f | sed -n 's/^+\([^+]\)/\1/p' \
          | sed -e 's/pub(super)//g' -e "s/<'a>//g" -e "s/Reads<'_>/Reads/g" \
                -e "s/&'a str/\&'static str/g" | tr -d ' \n,')
   ```
   Why: `reads.rs:269` (`positions`) and `:303` (`scan_for`) are 97 and 93 columns at `83e6b8a`
   and pass 100 with `pub(super) ` added, so `cargo fmt` wraps them, one parameter a line with
   a comma after the last; a comparison of lines would print them. The command reduces the old
   and the wrapped signature of each to the same text (for `scan_for`:
   `fnscan_for(cx:&mutDomCx<'_'_>node:NodeIdreads:Reads)->DomResult<Option<usize>>{`), and a
   change of a token (a name, a type) shows. At `83e6b8a` the lines it can touch are
   `reads.rs:120, 236, 241, 244, 252, 255, 269, 287` and `303`; against a base that already
   holds some of them (the attributes story's three tokens, if it merged first) the diff is
   smaller and the command still prints nothing.
10. Public surfaces: the story changes, in its last commit and after merging `main`, every public
    surface its change makes false, each number taken from the `slot task check` log of its tip.
    These files change in the last commit, after merging `main`, from one gate log, and by the
    epic's rule (`epic:955-961 @ 929269e`) they are not in the story's recorded scope; the
    Change names each of them and acceptance 9's `git diff --stat` check reads the Change.
    a. The old strings are read first, after the last merge of `main` (the reading story's merge
       included), by a command run before any change: `git grep -n -e 'passed' -e 'test files'
       -e 'tests:' -- README.md website/docs docs/badges`; the report records its output (59
       lines at `83e6b8a` and at `a14e3b0`).
    b. After the last commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges`
       prints nothing for each old string of 10c, except lines of the paragraph that 10d keeps
       (the run's history: at `83e6b8a` `README.md:661` and `website/docs/how-it-is-built.md:
       105` hold `230 test files`); the report names each such line as `<file>:<line>`, and
       each new number equals the one in the log. The lines of the gate block and of the
       headline strings (`README.md:11`, `:285`, `:612`, `:626`; the others of 10c) print
       nothing for their old strings.
    c. The values of `a14e3b0`, and of the reading unit at `83e6b8a`, which are the same, for
       reading only (the base is later: the reading story's last commit changes them): the test
       count `2394 passed` (`README.md:11`, `docs/badges/make.sh:38`, `docs/badges/tests.svg:1`,
       `:2`, `:10`); `2,397` and `1,845, and 552` (`README.md:285`, `website/docs/what-works.md:
       17`); `2,394 passed and 3 ignored: 1,842 passed` (`website/docs/status.md:31`); the line
       `1845 tests: 1842 passed, 0 failed, 3 ignored` (`README.md:626`,
       `website/docs/how-it-is-built.md:70`); `230 test files` (`README.md:612`,
       `website/docs/how-it-is-built.md:56`, and in the history paragraph `README.md:661`,
       `website/docs/how-it-is-built.md:105`).
    d. The gate block (`README.md:596-645`, `website/docs/how-it-is-built.md:40-89`) holds the
       lines of the log, and the paragraph after it that names the gate run's commit
       (`README.md:658`, `website/docs/how-it-is-built.md:102`) names this story's run beside the
       earlier ones, and keeps the earlier run's numbers as its history.
11. Housekeeping: `git grep -n -F 'scripts-dom-selectors' -- docs/design/scripts.md` prints
    line 5 and the new row of Q4; `git grep -n -F 'scripts-dom-selectors' -- AGENTS.md` prints
    one line, the entry of `crates/script`, which cites `story:scripts-dom-selectors` beside the
    slice `selectors` (it prints nothing at `83e6b8a`), and that entry's test count equals the
    tests of that crate in the log of the tip. The epic is not in the story's scope list and
    nothing of it is left to edit: its correction is plan work, and the epic holds both
    clauses of the deviation since plan commit `4013bde` (decision 14).
12. The folder's bounds hold. The test step ends within 120 s and no test runs over 60 s; a test
    over 5 s is named with its time in the report, which also gives the time of `dom_selectors`
    as a whole; no test asserts a time (the only bound on a time a test may hold is a wait for
    an event with a limit at least 10 times the time the test takes alone, `AGENTS.md:1966 @
    a14e3b0`, and a cost is proved by a count of steps); each case makes its own runner and
    parses a few lines, apart from 6c to 6f, 15c to 15g, 15i and the cost module, which build
    their trees in the script, from text or in Rust; no model call and no network request in a
    test; no Python in anything that stays in the repository; no generator with a committed
    output is added (the existing one gains no source: its time is not measured again); a
    build or test job in the
    pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes, and
    is not started again unchanged; every compiling or testing command carries `slot`, and a
    script that measures or waits starts without the prefix; an expensive product (a large
    tree) is built once per case set, not per case, where a case set shares it.
13. The lexical rules (decision 15). Each case is a row of `const CASES` or one test, over a tree
    the case builds in Rust where a name holds a code point the script text cannot carry. The
    outcomes are the rules of decision 15, which the agent's read of CSS Syntax agrees with;
    for the parts it did not read (13f's closing of a block by the end of the text: decision
    16), the text read wins where it differs: the report names the line, the section and both
    outcomes, and the line is corrected in the same change.
    a. `#\31 23` finds the element with `id="123"`; `#123` is a not-valid `SyntaxError`.
    b. `#a\:b` finds `id="a:b"` and `.md\:flex` finds `class="md:flex"`.
    c. `#\000061 b` finds `id="ab"`; `#\61  b` (two spaces after the digits) finds a `b` inside
       the element with `id="a"`, and not `id="ab"`.
    d. `#\0`, `#\d800` and `#\110000` each find the element whose id is U+FFFD.
    e. `[title="x\"y"]` finds `title='x"y'`; a string holding a backslash then a newline
       (`[title="x\` newline `y"]`) finds `title="xy"`; `[title='x']` and `[title=x]` find
       `title="x"`; `[title=x\:y]` finds `title="x:y"`.
    f. The end of the text inside a string and an open block, neither a fault: `[title="x`
       finds what `[title="x"]` finds and `[title=x` finds what `[title=x]` finds (both
       `title="x"`), with no exception; `[title="x]` finds `title="x]"`, the string taking the
       `]`. A string with an unescaped newline, `[title="x` newline `y"]`, is a not-valid
       `SyntaxError`, and so is `a\` newline `b`. The block's closing by the end is the part
       of this line the agent could not check (decision 16).
    g. A backslash at the end: `#\` finds the element whose id is U+FFFD and `a\` is a type
       selector for the name `a` then U+FFFD: both end without an exception.
    h. `a /* c */ b` and `a /* c */ > /* d */ b` give the nodes of `a b` and `a > b`; `a /* b`
       gives the nodes of `a`.
    i. Bounds: 4,096 bytes of `a` and comment run and 4,097 bytes are the not-supported
       `SyntaxError` ending `text over 4096 bytes`; 4,097 bytes that end in an unterminated
       comment are the same; 512 times `\61 ,` then `\61` is the not-supported `SyntaxError`
       ending `more than 512 simple selectors`, and so is the same text followed by `[`: the
       513th simple selector ends the parse and the fault after it is not read (decision 5),
       where `a[` alone is not valid.
    j. Newlines (decision 15), each of CR, FF and CRLF written in the script as `\r`, `\f` and
       `\r\n`: (1) `[title="x` NL `y"]` is a not-valid `SyntaxError`; (2) `[title="x\` NL
       `y"]` finds `title="xy"`, where a CRLF taken as two newlines would leave one in the
       string and make it a bad string; (3) `#\61` NL `b` finds `id="ab"` (the one
       whitespace after the escape is consumed, a CRLF whole), and `#\61` NL NL `b` finds a `b`
       inside the element with `id="a"`; (4) `a\` NL `b` is a not-valid `SyntaxError`. Each of
       the four for each of the three newlines: twelve cases.
14. The HTML case table (decision 16). a. The first name of the table, as a selector `[name="V"]`
    over an HTML element in an HTML document whose attribute holds `v`, matches, and the same
    with the cases swapped. b. `[data-surf-x="V"]` against `data-surf-x="v"` does not match.
    c. The first name of the table on an SVG element, and on any element not in the HTML
    namespace, matches exactly: `V` does not find `v`. d. `[a=b i]`, `[a=b I]` and `[a=b s]` are
    the not-supported `SyntaxError` ending `attribute flag`. e. The table test asserts the table
    is sorted, that its length is 46 and that its names equal the test's own copy of the 46
    names of decision 16; the doc comment of the table names the date of the text read (25
    November 2026) and, if the implementor found one, its commit. f. After the work, `git
    status --short --ignored -- .out` names none of the directories this work made for the
    four fetched texts (the report names them, and they are removed: `.out/` is ignored, so
    `git status --short` alone shows nothing either way), and `git diff --stat
    "$(git merge-base main HEAD)"..HEAD` names no file that holds text of the four standards;
    the report gives each address read, with the date of the text, the commit where there is
    one, the count of the list and each difference from decision 16.
15. Reads and charges (decisions 7 and 17), in `dom_selectors`.
    a. `the_selectors_read_the_tree_only_through_the_reads` reads the source of
       `crates/script/src/dom/selectors.rs` and fails when it holds `tree()`, `tree_mut()`,
       `charge(` or `charge_steps(`, as `dom_reading_cost.rs:593-598` does for `reading.rs`;
       it reads `selector_reads.rs` too and fails when that holds `tree_mut()` or
       `charge_steps(`, or when `children(` occurs in it anywhere but inside `Frames::next` (the
       test cuts the text of `Frames::next`, from its `fn next(` to the first line that is
       four spaces and a closing brace, asserts it holds `children(`, and asserts the rest of
       the file holds none, so the comments of `selector_reads.rs` never write that call: they
       say "the child list"), so that `descendant_elements` cannot copy a child list itself. The
       implementor shows each failure red once, with a string added, and reports the output.
    b. Two pairs of texts, each pair valid and within the 512 simple selectors. First, one
       comment around `a` (`/*`, filler, `*/`, `a`), padded to 4,096 bytes in one text and to 16
       bytes in the other: both hold the same two tokens, so only the bytes differ. Through
       `document.querySelector`, the text of 4,096 bytes charges at least 500 steps more than
       the text of 16: the glue's copy differs by 4,096 / 16 - 16 / 16 = 255 and
       `selector_text` by (1 + 256) - (1 + 1) = 255, 510 in all (both byte counts are
       multiples of 16, so nothing is lost to rounding). Without `selector_text`'s charge the
       difference is 255 and the floor fails. Second, 999 comments `/**/` then `a` (3,997
       bytes, 1,000 tokens) against one comment of 3,996 bytes then `a` (3,997 bytes, 2
       tokens): the bytes are equal, so the glue's and `selector_text`'s charges are equal and
       the first text charges at least 990 steps more (998 for the tokens). Without
       `selector_token`'s charge the difference is 0.
    c. `document.querySelector("[a]")` over 100 elements of 1,024 attributes each (none named
       `a`) charges at least 100 x 256 steps for the attribute scans beyond the same selector
       over bare elements; `document.querySelector` of `[`, a name of 4,000 bytes and `]` over
       10 elements of 100 attributes each charges at least 10 x 100 x 250 = 250,000 steps (the
       name's bytes / 16 for each attribute passed; the scans add 10 x 25 and the visits and
       gates 20), and without that charge it charges about 1,030. The same for equality
       (`attribute_equals`): `document.querySelector` of `[`, a name of 4,000 bytes and `="v"]`
       over the same 10 elements charges at least 250,000 steps, as no element has the
       attribute and its value is never read; without the name's charge it charges about 1,030.
    d. `document.querySelector(".a")` over 100 elements whose `class` is 64 KiB of one-letter
       tokens (none is `a`) charges at least one step per 16 bytes and one per token of each
       value (at least 100 x (4,096 + 32,768) = 3,686,400); in quirks mode the same, the fold
       paid by the bytes' charge. At the old price of 64 bytes a step the floor fails (100 x
       (1,024 + 32,768)).
    e. `document.querySelector('[a="v"]')` over 100 elements whose `a` is 64 KiB (never `v`)
       and that carry a second attribute `b` of 64 KiB charges at least 100 x 4,096 = 409,600
       steps (one per 16 bytes of the value of `a`, before the compare) and under 100 x 4,096 +
       1,000 = 410,600: each element costs its visit and gate and the 4,096, about 409,820 in
       all, and a charge for the value of `b` would add 409,600 and fail the ceiling. At the
       old price the floor fails (100 x 1,024).
    f. The fold of a name (decision 4), charged once at the parse: over a document of the three
       elements `html`, `head` and `body`, `document.querySelector` of one type name of 4,000
       bytes charges at least 990 steps more than the same length made of a comment around `a`:
       the parse alone differs by 249 (250 for the name's bytes, less the one token more the
       comment form has), and each of the 3 elements tested adds 250 for the type name in the
       matcher (249 + 3 x 250 = 999). The fold is charged once, so the same text on a second
       call charges the same as on the first. Without `selector_name`'s charge the difference is
       749 and without the matcher's charge for the name 249: the floor fails for each.
    g. The memo is made late. Over 1,000 `div` siblings in `body` (1,003 elements), through
       `document.querySelector`, a selector that matches nothing (`zzz`) and `x y` (no `y`)
       each charge at least 2,006 and under 2,100 steps (the visits and the gates, and the
       glue and the parse: no chain, no memo); `x div` (no `x`, so it matches nothing either)
       charges at least 7,500 more than `zzz` does: each `div` passes the gate, its chain is
       `div`, `body`, `html` (3 steps), its memo is 2 x 2 x 3 = 12 bytes (0 steps) and its
       cells are 2 x 3 = 6, so 9 more steps a `div`, 9,000 in all.
    h. Each charge of decision 17 that a floor can see is held by a case that fails without
       it, tried by hand (`cargo-mutants` is not installed on the host, so there is no tool
       for it): for each charge of `selector_reads.rs`, one line changed so that it charges
       nothing, the test that then fails, run with `slot cargo test -p surf-script --test main
       dom_selectors`, and the change removed. Each charge, the case that goes red and the
       arithmetic that shows it:
       - `selector_text`: 15b, first pair: the difference falls from 510 to 255, under 500.
       - `selector_token`: 15b, second pair: from 998 to 0, under 990.
       - `selector_name`: 15f: from 999 to 749, under 990.
       - the type-name bytes of `test_compound`: 15f: from 999 to 249.
       - the walk's step for each node: 6a: about 1,010 for 1,000 elements, under 2,000.
       - `gate`'s step: 6a: the same, about 1,010.
       - `next_ancestor`: 6b: 7,169 and about 30 for the glue and the parse, less the 503 steps
         of the chain, is about 6,696, under 7,000.
       - `memo`: 6d: 292,520 less 32,192 is 260,328, under 280,000; and 17h: about 530,900,
         under 560,000.
       - `cell`'s step: 6d: 292,520 less the 257,536 cells is 34,984, under 280,000.
       - the further simple selectors of `test_compound`: 17i: 3,700 for 1,000 elements,
         under 500,000.
       - `attribute_count x (name bytes / 16)` of `has_attribute`: 15c, second case: from about
         251,300 to about 1,030, under 250,000.
       - the same charge of `attribute_equals`: 15c, third case: the same figures. (The charge
         is a line of its own in each function, so each has its case.)
       Two charges are one step in a whole walk or chain, and no floor of this
       story is that close to its arithmetic: the step that finds the end of the walk, and the
       read of `next_ancestor` that finds no element parent. The report says they are not held.
       The scans of `reads.rs` are the reading unit's and are held by its tests. After the last
       removal `git diff --quiet` exits 0. The report lists, for each charge, the line, the
       test that failed (6a, 6b, 6d, 15b, 15c, 15f or 17h and 17i, by name) and its steps before
       and after.
    i. `Frames` (decision 17), a test over a tree parsed from text: a `body` with 100,000 `<b>`
       children, walked from the document with `Frames::next` until it ends, yields every node
       under the document (100,003: `html`, `head`, `body` and the `b`) in tree order and
       holds `most()` at 513 or less (it is 3); a `body` holding 510 nested `div` yields its
       nodes with `most()` at 513 or less. A walk that copied a parent's children would hold
       100,000.
16. Nothing is kept (decision 18). `git grep -n -F 'pub struct SurfSelectors;' --
    crates/script/src/dom/selectors.rs` prints a line (a guard: it prints `selectors.rs:6` at
    `83e6b8a` and at `a14e3b0`); the generated enum `SameObject` has the four variants it
    has at `83e6b8a` (`bindings.rs:2051`) and none is one of the nine; `querySelectorAll("p") !==
    querySelectorAll("p")` holds after no write between the calls, the one thing here that is new.
17. The cost module (decision 19), each case at its worst shape with a floor of steps and no
    assertion on a time, printing its steps; a case over 5 s is named with its time in the
    report. Each floor is under the exact arithmetic written with it (the glue and the parse add
    a few steps), and the arithmetic is the prices of decisions 7 and 8. The nine members each
    have a case, and each case names its member:
    a. `Element.querySelector("[zzz]")` from `document.body` over 5,000 elements of 100
       attributes each: floor 5,000 x (2 + 25) = 135,000 steps (a visit, a gate and 100 / 4 for
       the scan). And, over the page of acceptance 6 with `this` = `document.body`,
       `Element.querySelector` with the selector of 6b (the `span` in the innermost, no `x`):
       floor 7,000: 501 visits, 501 gates, 503 for the chain, 628 for the memo, 5,030 cells,
       7,163 in all.
    b. `Element.querySelectorAll("*")` from `document.body` over 50,000 elements: floor
       50,000 x (2 + 8) = 500,000.
    c. `Element.closest("x div div div div div div div div span")` from the `span` of the page of
       acceptance 6: floor 7,000, ends `Ran`, answers `null`: the gate 1, the chain 503, the
       memo 628, the cells 5,030, then for each of the 502 ancestors one step to reach it and
       one for its gate, 1,004, and the step that finds no element parent past `html`, 1:
       7,167 in all. And `Element.closest` of 512 times `div` (`div div ... div`) from the
       innermost `div` of 500 nested `div`, under a step limit of the test's own (1,000,000):
       ends `limit: steps-per-script`, in the fourth candidate (the chains are 502, 501, 500
       and 499 elements and a candidate costs 577 x its chain + 2, the first one 1 less: 289,655,
       289,079, 288,502 and 287,925, 867,236 for three and 1,155,161 for four), and a later
       `runner.evaluate("1 + 1")` ends `Ran`.
    d. `Element.matches` of an element that has the attributes `a`, `b`, `c` and 1,021 more,
       against `[a][b][c]`: floor 3 x 256 = 768 steps (the exact arithmetic adds the gate's and
       the two further steps, 3).
    e. `document.querySelector` of a list of 512 selectors `a,a,...,a` over 1,000 elements named
       `b`: floor 1,000 x 512 steps, ends `Ran`; over 10,000 elements under a step limit of the
       test's own (1,000,000) it ends `limit: steps-per-script` (the case asserts the end, no
       time).
    f. `document.querySelector` of a selector text at the 4,096-byte bound made of the most
       tokens the grammar admits within the 512 simple selectors (the case counts its tokens):
       floor of 514 + its tokens (the glue's 257, `selector_text`'s 257 and a step a token), and
       it ends `Ran`.
    g. One case for each of the five other members, each over a shape and with a floor written
       here, the exact arithmetic a few steps above it:
       - `Document.querySelector("[zzz]")` over 5,000 elements of 100 attributes each: floor
         135,000, as 17a.
       - `Document.querySelectorAll("*")` over 50,000 elements: floor 50,000 x (2 + 8) = 500,000.
       - `DocumentFragment.querySelector("[zzz]")` over a fragment of 1,000 elements of 100
         attributes each, which the test's `createDocumentFragment` answers already filled
         (built in Rust, decision 12): floor 1,000 x (2 + 25) = 27,000.
       - `DocumentFragment.querySelectorAll("*")` over a fragment of 10,000 elements built the
         same way: floor 10,000 x (2 + 8) = 100,000.
       - `Element.webkitMatchesSelector` over the element and the selector of 17d: floor 768.
    h. The memo (decisions 7 and 8): a chain of 100 nested `div` in `body`, and
       `Element.querySelector` from `body` of the selector `x > x > ... > x > div` (99 times
       `x >`, then `div`: 100 compounds, one simple selector each). Each `div` passes the gate;
       for the `div` at depth k the chain is n = k + 2 elements (itself, k - 1 `div`, `body`,
       `html`), and it costs a visit and a gate (2), the chain (n), the memo
       (floor(2 x 100 x n / 16)) and 100 n cells. Over k = 1 to 100 the n add up to 5,250: 200 +
       5,250 + 65,600 + 525,000 = 596,050 steps, and about 450 for the glue and the parse. Floor
       560,000, ceiling 650,000: without the memo's charge the steps are about 530,900 and the
       floor fails. It ends `Ran`.
    i. The further simple selectors of a compound (decision 7): `Element.querySelector` from
       `document.body` of 511 times `[a]` then `[zzz]` (one compound of 512 simple selectors,
       1,538 bytes) over 1,000 `<b a>` siblings (one attribute each, so the scan of an element
       and the name's bytes charge nothing): no element matches and each is tested through all
       512 simple selectors, a visit and a gate and 511 further steps, 513 an element, 513,000
       in all and about 1,700 for the glue and the parse. Floor 500,000, ceiling 520,000, ends
       `Ran`; without the 511 further steps the steps are about 3,700.

## Out of scope

Selectors beyond the subset, `:scope` and every other pseudo-class among them, the sibling
combinators, namespace prefixes of `*` and none and the attribute flags `i` and `s` (they throw
the not-supported `SyntaxError` of decision 5); the case rules of XML documents (an element
outside the HTML namespace matches exactly); a live `NodeList` (the member returns a static
one, which is what the standard says); a cache of parsed selectors; any write to the tree; any
change of the generator; the real `mutation` and `attributes` slices; vendoring any of the
four texts of decision 16; a change of the glue's prices, which are the reading story's.

## Placement

After `story:scripts-dom-reading`, which this story needs for `Window.document`, the wrapper
table, the quirks mode, `reads.rs`, the price of a byte and the shape of the cases (acceptance 1a
holds the work until its merge is on `main`); `story:webidl-bindings` and
`story:html-script-hook` are on `main`. Beside `story:scripts-dom-mutation` and
`story:scripts-dom-attributes`, both under revision; the lists of their scopes and their
Placement are read at plan commit `929269e`, where the attributes story's text is that of
`9e5b717` and the mutation story's that of `439fd32`. The attributes story lists
`selection.txt`, `bindings.rs`, `reads.rs`, `dom/mod.rs`, `tests/dom_reading.rs`,
`docs/design/scripts.md`, `AGENTS.md` and the public surfaces, as this story does, and also
`tests/main.rs`, which this story does not edit. The mutation story lists its own three files
of the slice and two test files, and of this story's files `dom/mod.rs`, `reads.rs`,
`bindings.rs`, `selection.txt`, `tests/dom_reading.rs`, `docs/design/scripts.md`, `AGENTS.md`
and the public surfaces, and also `tests/main.rs`; it depends on the attributes story and has
no edge to this one. No edge to `story:scripts-parse-run`: that story calls the slices
through `dom::slices(info)` and adds `DomState::swap_tree` to `prelude.rs` and `bindings.rs`,
which this slice does not use.

The files shared with a story beside it or one the drafts below name:

- `crates/script/src/bindings.rs` is generated and never merged by hand. This story, the
  attributes story, the mutation story, `story:scripts-parse-run` and
  `story:scripts-utf16-length` all change it or its generator: this one the nine doc comments
  and the digest lines; the attributes story the fields `stored`, `live` and `charged` of
  `DomState`, `charge_stored`, `settle_stored`, four accessors, the change of `strings_value`
  in `prelude.rs` and a doc comment (the `setAttributeNS` note, row 96); the mutation story
  `charge_nodes`, the charge for each item of a variadic list and the charge of a refill, in
  `prelude.rs`; parse-run `DomState::swap_tree`; utf16-length `string_length`. The edits to
  `prelude.rs` are in one region and are merged by hand by the story that merges later;
  `bindings.rs` is regenerated by whichever merges later (`slot cargo run -p surf-webidl --bin
  bindings`), and `slot task webidl-check` and `slot task webidl-slow` hold the result. The
  same holds for the glue's prices if they merge after this story. The draft
  `story:scripts-dom-collection-names` is a further editor of the glue (it names the glue's
  refill and the `HTMLCollection` notes, and `NodeList` goes through the same glue); that it
  edits `bindings.rs` beyond the regeneration is inferred, not stated. The mutation story's
  text still calls `undo_stored`, which the attributes story renamed `settle_stored`: that is
  for the mutation story to revise, and this story uses neither.
- `crates/webidl/selection.txt`: this story changes the nine rows of its slice (lines 110-112,
  126-127, 175-176 and 231-232 at `83e6b8a`); the attributes story also lists the file and
  edits the `setAttributeNS` note at `:96`, not adjacent to any of them. Both regenerate
  `bindings.rs` from it, so the header's digest line of `bindings.rs` and its last line
  conflict as text: the second to merge runs the generator again and takes no side. The rows
  125, 174 and 230 (`moveBefore`, slice `mutation`) are adjacent to rows 126, 175 and 231: the
  mutation story at `439fd32` changes the notes of rows 63, 151 and 157 and none of those
  three; a note added there later is a conflict by hand, resolved by keeping both.
- `crates/script/src/dom/reads.rs`: the reading unit's file. Each slice has its own file for its
  reads: this story's are in the new `selector_reads.rs` and the attributes story's in
  `attribute_reads.rs`. This story changes `reads.rs` by visibility and one widening:
  `pub(super)` on `charge` (`:120 @ 83e6b8a`), `ATTRIBUTES_A_STEP` (`:252`), `attribute_count`
  (`:255`), `Reads` (`:236`), `scan_for` (`:303`), `positions` (`:269`) and `value_at`
  (`:287`), and `Reads`'s lifetime with `&'a str` in `Name` (`:241`) and `Value` (`:244`).
  The attributes story makes `charge`, `ATTRIBUTES_A_STEP` and `attribute_count`
  `pub(super)` too (three of the same tokens, on the same lines), so the two stories' edits of
  those lines are the same edit, and the story that merges second resolves it by keeping the
  one. There is no edge between the two stories. The mutation story makes `charge`, `find`
  (`:349`), `Reads` and `html_element` (`:218`) `pub(super)`: `charge` is the same edit again,
  but `enum Reads` is not: this story writes `pub(super) enum Reads<'a>` and the mutation story
  `pub(super) enum Reads`, two hunks on one line that conflict as text, resolved by hand by
  keeping the `<'a>` form (`Name` and `Value` are this story's alone). `find` and
  `html_element` are the mutation story's alone, but `find` takes `reads: Reads`, so the later
  merger checks that it still compiles. The two drafts `story:scripts-dom-sibling-positions`
  and `story:scripts-dom-collection-names` name no file of this slice's: the first names no
  file at all, and the second only `selection.txt` and the glue; whether either changes
  `reads.rs` is inferred, not stated, and a conflict there is a conflict of position,
  resolved by keeping both.
- `crates/script/src/dom/mod.rs`: two lines, `mod selector_reads;` and `pub use
  selector_reads::Frames;`; the attributes story adds two (`mod attribute_reads;`, `mod
  attribute_writes;`) and the mutation story two `mod` lines, in other places of the lists
  of `mod` and `pub use` lines, so the later merge keeps all of them.
- `crates/script/tests/main.rs`: this story does not edit it (`mod dom_selectors;` is declared at
  `:21` already). The attributes story adds `mod dom_attributes_cost;` after `mod
  dom_attributes;` (`:17`) and the mutation story adds `mod dom_mutation_cost;` after `mod
  dom_mutation;` (`:18`), each in its Change; this story adds no line there, so it shares
  none.
- `crates/script/tests/dom_reading.rs`: the case `other_slices_still_throw` holds three lines,
  one for each slice, at `:2424-2426`; this story changes the third; the attributes story, at
  plan commit `9e5b717`, changes the second (to `el.getAttributeNode("id")`) and names the
  three in its Change and Placement. The first, `document.appendChild(el)`, is the mutation
  slice's: the mutation story at `439fd32` lists `dom_reading.rs` and changes that line (its
  acceptance 18), but its scope there names the file only for that line. The later merge keeps
  all three changes.
- `crates/script/tests/dom_selectors.rs` uses `Namespace`, `Tree::create_element_ns` and
  `Tree::set_attribute_ns` in the Rust cases of acceptance 3; the draft
  `story:html-open-namespace` changes `Namespace` (it depends on the attributes story): when it
  merges first, the cases take its form of the type.
- `docs/design/scripts.md` (also the stories beside it): this story adds one row to the table
  of Q4 after the row `parser: attributes per element` (`:316 @ 83e6b8a`). The places the
  others name: `story:scripts-parse-run` the rows `wall clock per document` (`:312`) and
  `after Terminated(MemoryBudget)` (`:320`); `story:scripts-dom-mutation` at `439fd32` Q1's row
  (`:86`), the row `parser: steps` (`:318`) and sentences in the paragraphs at `:322-332` and
  `:902-904`; `story:scripts-dom-attributes` the paragraphs at `:322-332` and `:902-904`;
  `story:swell-0-9-0` (decision 13, `story/swell-0-9-0.md:565-592`) the Q4 rows `:302`,
  `:303`, `:307` and `:320`, the paragraphs `:255-265` and `:338-341`, and Q11 `:902-909`.
  None of the swell story's places is adjacent to the new row after `:316`; `:320` is also
  parse-run's. The new row is two lines from the mutation story's `:318` with `:317`
  unchanged between them, so git should merge the two clean; the later merger checks anyway
  and keeps both.
- `AGENTS.md`, `README.md`, `docs/badges/make.sh`, `docs/badges/tests.svg` and the other public
  surfaces, which the stories beside it rewrite too (by the epic's rule `README.md`,
  `docs/badges` and `how-it-is-built.md` are in no story's recorded scope, and `status.md` and
  `what-works.md` only where a story changes another sentence of theirs; the stories still
  change them): `story:swell-0-9-0` (active; it lists README, `docs/badges/make.sh`,
  `tests.svg`, `how-it-is-built.md`, `status.md`, `what-works.md`, `AGENTS.md` and
  `scripts.md`), `story:scripts-dom-attributes`, `story:scripts-dom-mutation`,
  `story:scripts-parse-run`, `story:test-step-holds-with-tls` and `story:tls-tests-as-tables`
  (they change the count of tests the TLS step prints, the `552` of acceptance 10c, and list
  `AGENTS.md`), `story:session-argument-checks` and `story:conformance-lock-and-lints` (each
  lists `AGENTS.md`; the second also `website/docs/status.md`) and
  `story:scripts-document-port` (it lists `docs/design/scripts.md`, `website/docs/status.md`
  and `website/docs/what-works.md` and cites `README.md:285`, `status.md:23` and
  `what-works.md:17`; it has no edge to this story, and comes after it through
  `story:scripts-html-conformance`, which it depends on and which waits for this story).
  Whichever merges later merges `main` first and takes its numbers from its own log, as the
  reading story's Placement says of its pair with `story:swell-0-9-0`. `AGENTS.md:365-369` is
  one entry of five lines that each of them rewrites, so the second merge resolves its text by
  hand. `docs/badges/tests.svg` is written by `docs/badges/make.sh`: the story edits
  `make.sh:38` and runs the script, and the second merger runs it again after taking the
  other's `make.sh`. The public-surface lines the last commit rewrites are those of acceptance
  10 (`README.md:11, 285, 596-645, 612, 626, 658`, `docs/badges/make.sh:38`,
  `docs/badges/tests.svg:1, 2, 10`, `website/docs/how-it-is-built.md:40-89, 56, 70, 102`,
  `website/docs/status.md:31`, `website/docs/what-works.md:17`).

This story changes `selection.txt` and `bindings.rs`, so it lists both; the reading unit's
merge makes them its base. `story:scripts-html-conformance` waits for this story.

## Why `epic:scripts-in-a-page`

The selector members the epic's acceptance 3 names, and the `querySelector("table")` of the
`needs-script` cases of web-platform-tests (`scripted_foster01.dat`, `epic:396-398`).
