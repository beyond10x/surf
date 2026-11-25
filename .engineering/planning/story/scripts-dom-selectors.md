---
format: aep.planning-md/3
id: story:scripts-dom-selectors
kind: story
status: proposed
title: The 9 selector members over a subset of Selectors Level 4, bounded in text, count and steps
summary: 'querySelector, querySelectorAll, closest, matches and webkitMatchesSelector on Element and the two of Document and DocumentFragment, for the subset of type, universal, #id, .class, [a], [a=v] (CSS escapes, strings, comments and the newlines CR, FF and CRLF included; an unterminated string or an unclosed block at the end of the text is closed, not an error), compounds, descendant and child combinators and lists, with the case rules of the HTML Standard (a type name and an attribute name lower-cased for an HTML element and compared exactly, the 46-name list of case-insensitive attribute values held as a table) and the tree''s quirks mode, every refusal a SyntaxError (a text that is not valid says so, a valid selector outside the subset or over the bounds of 4,096 bytes and 512 simple selectors says surf does not support it, a named deviation beside the nine rows), every read of the tree in a file of its own (selector_reads.rs) that charges before the work at one step per 16 bytes and a step a token, an element, a compound and an ancestor against the entry''s 10,000,000 steps, the matcher''s memo charged before it is made, and querySelectorAll a static NodeList that is not a [SameObject] and keeps nothing.'
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-dom-reading
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
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
  path: docs/badges/make.sh
- confidence: cited
  path: docs/badges/tests.svg
- confidence: cited
  path: docs/design/scripts.md
- confidence: cited
  path: website/docs/how-it-is-built.md
- confidence: cited
  path: website/docs/status.md
- confidence: cited
  path: website/docs/what-works.md
revision: 30
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 13}
---
## Context

The epic's eleventh candidate (`.engineering/planning/epic/scripts-in-a-page.md:475-477 @
20b206b`), the slice "Selectors, 7" (`:738-742`) and the deviation line (`:134-135`):
`querySelector`, `querySelectorAll`, `closest`, `matches` and `webkitMatchesSelector` for a
subset of Selectors Level 4. The epic says a valid selector outside the subset throws
`NotSupportedError`; this story rules otherwise (decision 5: the DOM Standard gives a caller
only `SyntaxError`), and decision 14 corrects the epic's three places and its list of
deviations. The epic's "7" counts the members on `Element` and `Document`; the selection
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
(one file of reads) and its decision 20's prices are ruled and not built yet; this story
takes the prices from that text (decision 7). What is there:

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
  `NodeData::Element { namespace, local_name, attributes, .. }` (`:135`) and `Attribute`
  (`:111`); the members reach them only through `reads.rs` (decision 17).
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
  does not deliver, no recorded limit), a walk with its own stack `find` (`:349-366`) and
  `select` (`:370-384`) whose callback gets `&Tree` and no `cx`, `scan_for` (`:303-322`: one
  step per 4 attributes of the list before the one pass, then the value's bytes, then one per
  class token) and `quirks(cx)` (`:1022-1025`), `positions` and `value_at` (`:267-299`),
  `ATTRIBUTES_A_STEP` (`:252`) and `attribute_count` (`:255`), all private, and the enum
  `Reads` (`:236`, `Name` and `Value` take a `&'static str`). `SurfWrap` keeps one wrapper per node
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
2. The subset is the epic's (`epic:738-742 @ 20b206b`): the type selector, the universal
   selector, `#id`,
   `.class`, attribute presence `[a]` and equality `[a=v]`, compound selectors, the descendant
   and the child combinator, and selector lists (a comma), with the lexical rules of decision 15
   (escapes, comments, strings, quoted and unquoted attribute values). Whitespace around a
   combinator is allowed (Selectors Level 4, section 16: it is required between units only
   where the combinator between them is omitted) and so around a comma (section 4.1). Whitespace
   around the whole text is allowed too, by CSS Syntax's tokenizing, which the agent did not
   read for this point (decision 16). The head comment of `selectors.rs` gives the grammar,
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
     price once, at the parse (decision 7).
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
     (`reads.rs:454-506`), so this story writes its own, in `selector_reads.rs` (decision 17),
     over the same `scan_for` charge. Names are compared after decision 15 has decoded them.
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
   - The parser reads the whole text first. A text with any fault of the grammar is not valid
     whatever construct precedes it (`a:hover[` is not valid); the first construct outside the
     subset of a text without a fault is named. An excluded pseudo-class or pseudo-element is
     recognised by its shape (a colon or two, an identifier, an argument in balanced parentheses
     that holds strings and no more); its name and argument are not validated, so `a:hovr` and
     `a:nth-child(x y z)` get the not-supported message where a browser says they are not valid:
     the name and the code are the same, the message differs, and the pseudo-class texts of
     Selectors were not read (decision 16). The end of the text closes what is open (decision
     15): `a:not(b`, whose `(` the end closes, is a pseudo-class by shape, not supported, as
     `a:not(b)` is.
6. The bounds. No recursion: the parser is one loop and the matcher walks with indexes, so a
   hostile text or tree needs no stack beyond the frame. The selector text is at most 4,096
   bytes of UTF-8 after the glue's copy, counted as given: an escape and a comment count their
   bytes. It holds at most 512 simple selectors (a type, `*`, an id, a class or an attribute
   selector), counted after the parse: an escape is part of one identifier and a comment is
   nothing, so neither adds a simple selector. That also bounds the compound selectors, the
   complex selectors of a list and the combinators at 512. Nothing nests in the subset, so the
   nesting bound is 0: the one scan that balances parentheses (decision 5) is a counter. A text
   over 4,096 bytes is a `SyntaxError` of the not-supported message naming the bound
   (`text over 4096 bytes`), before any parse and whatever it holds, valid or not (the text may be
   a valid selector, which a browser would run: the message says surf does not support it, and
   a script is not told its selector is invalid); the 513th simple selector is the same kind
   of `SyntaxError` naming its bound (`more than 512 simple selectors`), where the parser is
   when it reaches it. The buffers the story's code makes, each bounded by a number stated
   here or charged where it is made:
   - The names an escape decodes are at most 1.5 times the bytes of their text (`\0`, two
     bytes, becomes U+FFFD, three), so at most 6,144 bytes of names are held, and at most
     6,144 more for the lower-case forms of the type and attribute names (decision 4), the fold
     charged at the parse (decision 7).
   - The parsed list: at most 512 simple selectors, 512 compounds and 512 complex selectors,
     each a record of a few fields, and the names above; it is made as the parser goes and the
     parse's charge (decision 7) is made before each token it takes. It is bounded by the
     4,096-byte text, so it is a few tens of KiB at most. No token list is built: the parser takes
     the lexer's tokens one at a time.
   - The chain of the element's ancestors, and the memo (decision 8): the chain at most 512
     ids (4,096 bytes), the memo charged before it is made, at one step per 16 bytes.
   - The walk's stack: at most the nodes of the tree, 1,048,576 ids, 8 MiB (a wide parent
     pushes all its children at once); each node on it is charged when it is taken.
   - The result `Vec` of `querySelectorAll`: an id is pushed only for a node whose visit step was
     charged, so at most 1,048,576 ids, 8 MiB.
7. What is charged and by whom, against the entry's 10,000,000 steps. The rules are the reading
   story's decision 20: one price for a byte, one step per 16 bytes, for every byte the glue or
   the slice reads, compares, folds or copies, an argument's included; a step a piece beside its
   bytes (here a token of the selector, an element tested, a compound tested, a class token);
   each charge before the work it pays; a walk whose end is not known charges as it goes,
   before it reads each node. The charges of this slice:
   - The glue: 1 step a call; the copy of the argument at one step per 16 bytes (at most
     3,145,728 steps for the longest String, 16,777,216 code units of at most 3 bytes,
     50,331,648 bytes; the glue charges per 64 bytes at `83e6b8a` and the implementor reads
     `Copies::copy` first and says which price the base holds); for a `NodeList`, 8 steps a
     result node (`COLLECTION_NODE_STEPS`) plus the wrappers on the engine's heap. The glue
     charges the copy and the slice charges the read of the same bytes: both, as two works
     (the reading unit charges `getElementsByClassName`'s argument in the slice beside the
     glue's copy, `reads.rs:462-463`); the two together are at most 257 steps a call more than
     the glue's alone.
   - The parse, before the work it pays, through `selector_reads` (decision 17): `1 + bytes /
     16` for the text within the bound, the bytes as given, escapes and comments included (257
     at the bound), charged before the lexer reads the first byte; one step a token, charged
     before the lexer takes it; and for each type name and each attribute name, one step per
     16 bytes of the decoded name (`name bytes / 16`), charged before the name is folded to
     ASCII lower case (decision 4), once. A token is one of CSS Syntax Level 3's tokens of a
     selector text: a run of whitespace, a comment, an identifier, a hash, a string, a
     delimiter, a bracket, a parenthesis, a comma. A text has at most 4,096 tokens, and its
     names at most 6,144 bytes, so a parse costs at most 4,353 + 384 = 4,737 steps. A parse
     that ends at a fault or a bound has paid for what it took. A text over the bound is
     refused before `selector_text`: the slice charges nothing for it.
   - The walk of `querySelector` and `querySelectorAll`: one step per node it visits, before the
     node is read (a walk that charges as it goes and builds no list of the tree first). The
     rightmost compound of each complex selector, tested at each element, is a compound test
     of the matcher's.
   - The matcher, for each element it tests:
     - One step per compound it tests against the element, and one per 16 bytes of the
       selector's type name (the form the element's namespace calls for, decision 4) when the
       compound has one, charged before the compare (the compare reads no more than that name).
     - The memo. A complex selector of one compound needs none. For any other, no memo is made
       for an element until its rightmost compound has matched it. Then the matcher reads the
       element's chain, the element and its element ancestors up to the root, one step per
       ancestor, before each read (`next_ancestor`), and makes a memo of one byte for each
       (compound, chain position) answer: `compounds x chain length` bytes, charged `bytes /
       16` steps before it is allocated (`selector_reads::memo`). The memo is made for one
       element and dropped with it, so nothing is cleared; a design that keeps one memo for
       the call and clears it for each element pays the clear at the same price, `bytes / 16`
       before it. The chain's ids are bounded by 512 (decision 6). The matcher is top down: a
       (compound, position) answer is computed when a match asks for it, so the cells tested
       are those the match reaches, at most all of them (decision 8).
     - An id, class or attribute test reads through `selector_reads`, over the items of
       `reads.rs` that both slices use (decision 17), at the price of the reading unit's scan.
       `selector_reads` charges one step per 16 bytes of the selector's attribute name for
       each attribute of the element's list (`attribute_count x name bytes / 16`, none for a
       name under 16 bytes), before it calls the scan. The scan then charges as the reading
       unit has it: one step per 4 attributes of the list before the pass, one per 16 bytes of
       the element's value it reads or compares, before it is read (the compare reads no more
       than that value; the base charges per 64 until the reading story's price commit), and
       one per class token of a `class` value. In quirks mode the fold of a value is paid by
       the bytes' price. No existing charge of `reads.rs` changes.
   - How a charge that fails ends the member: `reads.rs`'s private `charge` answers a throw the
     engine does not deliver (the run is ending) and records no limit (reading decision 6);
     `charge` becomes `pub(super)` and `selector_reads` calls it, each function there that
     charges returns that `DomResult`, and `selectors.rs` passes it up with `?`.
   So the longest stretch of surf's own work between two charges is one compound test of one
   element, and every end a script can reach through a selector is the engine's: `limit:
   steps-per-script` or `limit: memory`, or a throw.
8. The worst cases in numbers, which the story's tests hold. Walk: a tree of the node limit
   (1,048,576 nodes) costs at most 2 steps per node to search for a single selector that matches
   nothing (the visit, and the rightmost compound at an element), 2,097,152 at the limit: 4
   calls fit in an entry and the 5th ends it (10,000,000 / 2,097,152 is 4.77). A loop of such
   calls ends `limit: steps-per-script` in at most 5 turns over a tree at the limit. A list of
   `k` complex selectors adds `k` compound steps an element. Matcher: an element has at most 511
   element ancestors, so its chain holds at most 512 elements, and a matcher that tests each
   (compound, chain position) pair once per element, by its memo, costs for one element and one
   text at the bounds:
   - 512 x 512 = 262,144 compound steps;
   - 384 x 512 = 196,608 more for the type names (6,144 bytes of names / 16, at each of 512
     positions);
   - 511 for the ancestors read;
   - the memo, 512 x 512 = 262,144 bytes, charged 262,144 / 16 = 16,384 steps before it is
     allocated;
   - 1 for the visit;
   in all 1 + 262,144 + 196,608 + 511 + 16,384 = 475,648 steps for elements without attributes
   (an attribute test adds the scan of decision 7, up to 256 steps a test on a list of 1,024
   attributes; the engine's limit ends such a text, charged as it goes). 21 such elements fit
   in an entry (21 x 475,648 = 9,988,608) and the 22nd ends it. The memo is why the matcher
   never backtracks; a matcher that did is charged by the same rule and ends the same way,
   only sooner. The memo is made only for an element whose rightmost compound matched, and
   never for a selector of one compound, so a script that wants the 16,384 steps and the 256
   KiB of the memo for each element must have elements that match the rightmost compound
   (acceptance 17h). `closest` tests at most 512 candidates (`this` and 511 ancestors), each
   with its own chain and memo: its memos alone are at most 512 compounds x (512 x 513 / 2)
   positions = 67,239,936 bytes, 4,202,496 steps, so its worst case is charged by the same
   rule and ends the entry.
   Result: a `NodeList` costs 8 steps a node in the glue (8,388,608 for 1,048,576 nodes) and
   ends `limit: memory` in the glue while it wraps, after the slice charged its walk, at about
   123,000 nodes (Context); the slice's own `Vec<NodeId>` is at most 8 MiB (1,048,576 ids),
   each id paid by its node's visit step. In the test profile a script runs 1,372,567 to
   1,405,663 steps per second (`docs/design/scripts.md:273-274 @ a14e3b0`, the same lines at
   `83e6b8a`; `:369 @ f463e24`, a commit not on `main`, holds the same text), so 10,000,000 steps
   take about 7.4 s there (7.358 s measured, `:271-272`) and the wall clock of 5 s ends a
   script first (`:302`). A test gives limits of its own and asserts no time (acceptance 12).
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
11. The deviations are named beside the members, as the epic says (`epic:753-758 @ 20b206b`).
    Each of the nine rows gains one note of two clauses, `deviation: a valid selector outside
    the subset, and a selector over the bounds, throws a SyntaxError whose message says surf
    does not support it, where a browser runs it; deviation: a pseudo-class or pseudo-element
    is recognised by its shape, so an unknown name or a bad argument gets that message, not
    the one of an invalid selector`, and `crates/script/src/bindings.rs` is regenerated (the
    note is part of each member's doc comment, as at `bindings.rs:2512`). The generator does not
    change: `git diff --stat` names nothing of `crates/webidl/src/`. No line of
    `crates/webidl/selection.txt` says `NotSupportedError` for a selector at `83e6b8a`
    (`git grep -n -i 'NotSupportedError' 83e6b8a -- crates/webidl/selection.txt` prints
    nothing); the nine rows carry no note, so the note above is new and replaces no text.
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
    as throwing (`git grep -n -E 'querySelector|closest|webkitMatches' 83e6b8a --
    crates/script/tests` prints that one line).
14. The plan's text this story makes false is corrected in the same wave, as part of the plan
    and not of this story's code, by four edits of
    `.engineering/planning/epic/scripts-in-a-page.md` (lines of the plan at `20b206b`) and one
    of `story:scripts-dom-attributes`: the epic's deviation line (`:134-135`), its candidate 11
    (`:475-477`: "7", `NotSupportedError`), its slice "Selectors, 7" (`:738-742`: "7", "that
    `scripts-decisions` names", `NotSupportedError`) and its list of deviations (`:753-758`,
    which gains the selector deviation), and acceptance 8 of the attributes story, which names
    `el.querySelector` as a member that still throws. One more bullet of the epic's last section
    "Corrections of 2026-11-25" (`:977`) records them. The exact finds and puts are the
    planning's file of edits, delivered with this story.
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
    a new file, `crates/script/src/dom/selector_reads.rs`, declared by one line in
    `crates/script/src/dom/mod.rs`, `mod selector_reads;` (the attributes story's
    `attribute_reads.rs` is the same form). It charges before the work and holds every call
    of `tree()` this slice makes. `selectors.rs` holds the lexer, the parser, the types of the
    parsed list (`Compound`, a complex selector, the list), the matcher over those types and
    the nine methods, and holds no call of `tree()`, no `tree_mut()`, no `charge(` and no
    `charge_steps(`; `selector_reads.rs` holds no `tree_mut()` and no `charge_steps(` (it
    charges through `charge` of `reads.rs`). A test reads both sources with `include_str!` and
    fails when one of those strings is added to the file that must not hold it (the form of
    `dom_reading_cost.rs:593-598`).
    `reads.rs` changes by visibility and by one widening, and in no other way: `pub(super)` on
    what both slices use, `charge`, `ATTRIBUTES_A_STEP` and `attribute_count` (the attributes
    story makes the same three `pub(super)`) and the attribute scan with its helpers, `Reads`,
    `scan_for`, `positions` and `value_at`; and the scan's name widened to `&str`: `Reads`
    gains a lifetime and its `Name` and `Value` take a `&'a str` where they take a
    `&'static str` at `83e6b8a`. No existing function changes its body, its charge or its
    answer. The functions of `selector_reads.rs`, each `pub(super)` and each with a doc comment
    that says what it charges (the names are the plan's and may change with a reason in the
    report); `reads::quirks` (no charge) is the one other function the members use:
    - `selector_text(cx, bytes)`: `1 + bytes / 16`, before the lexer reads the text.
    - `selector_token(cx)`: 1 step, before the lexer takes a token.
    - `selector_name(cx, bytes)`: `bytes / 16`, before a type name or an attribute name is
      folded to ASCII lower case (decision 4); once for each name, in the parse.
    - `descendant_elements(cx, scope, visit)`: the elements under `scope`, `scope` left out, in
      tree order, with its own stack as `find` has it; one step per node, before the node is
      read; `visit(cx, element)` is the matcher of `selectors.rs` and gets `cx`, not a `&Tree`;
      the walk goes on until `visit` says stop.
    - `next_ancestor(cx, node)`: the parent of `node` when it is an element, else none; one step,
      before the read.
    - `memo(cx, cells)`: a `Vec<u8>` of `cells` zeroed bytes; `cells / 16` steps, before it is
      allocated (decision 7).
    - `test_compound(cx, node, compound, quirks)`: whether the element `node` matches the
      compound; one step and one per 16 bytes of its type name, before the compare, then the
      id, class and attribute tests below.
    - `has_class`, `has_id`, `attribute_equals` and `has_attribute`, called by `test_compound`:
      each charges `attribute_count x name bytes / 16` for the selector's attribute name (the
      class and id names are the fixed `class` and `id`: none), then calls `scan_for` for its
      charge of one step per 4 attributes and of the value's bytes before the compare, and one
      step per class token before the first token is looked up; they use `positions` and
      `value_at` (`reads.rs:267-299`) to find and read the value.
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
  decision 15), the parser of the subset, the matcher over the parsed list with its memo, the
  nine methods of `SurfSelectors` (decisions 1 to 10), the table of the 46 HTML attribute names
  of decision 16, the grammar in its head comment. It holds no call of the tree and no charge.
  `SurfSelectors` stays a unit struct.
- `crates/script/src/dom/selector_reads.rs` (new): the functions of decision 17, which charge
  before the work and hold the tree calls. `crates/script/src/dom/mod.rs`: one line,
  `mod selector_reads;`.
- `crates/script/src/dom/reads.rs`: visibility only (`pub(super)` on `charge`,
  `ATTRIBUTES_A_STEP`, `attribute_count`, `Reads`, `scan_for`, `positions`, `value_at`) and the
  scan's name widened to `&str`; no function changes its body, its charge or its answer.
- `crates/script/tests/dom_selectors.rs`: the module `dom_selectors` (declared at
  `tests/main.rs:21`): `Case { row, expression, expected }`, `const CASES` and the helper `run_case`
  as `dom_reading.rs:252-268` and `:841-863` have them (private there, so copied), the test
  `mutation` slice (decision 12), the module `cost` (decision 19), the cases of acceptances 13
  and 14, the source test of decision 17 and the table's test with its own copy of the list.
- `crates/script/tests/dom_reading.rs`: the line of decision 13 (`:2426`).
- `crates/webidl/selection.txt`: the note of decision 11 on the nine rows;
  `crates/script/src/bindings.rs`: regenerated with `slot cargo run -p surf-webidl --bin
  bindings`; its header's digest of `selection.txt` and its own last line change, and nothing
  else but the nine doc comments.
- `docs/design/scripts.md`: one row of the table of Q4 (`:300-320`) for the selector's bounds
  (text, simple selectors, who charges), with the value marked "chosen", naming
  `story:scripts-dom-selectors`.
- `AGENTS.md`: the entry of `crates/script` (`AGENTS.md:365-369`: it names the slice `reading`
  and its story, the dependencies including `surf-url`, and `29 tests` with per-file counts
  of three files, which the reading story's last commit replaces by the log's): this story names
  the slice `selectors` and sets the count of tests from the log of its own tip.
- In the last commit, the public surfaces of acceptance 10: `README.md`, `docs/badges/make.sh`,
  `docs/badges/tests.svg`, `website/docs/how-it-is-built.md`, `website/docs/status.md`,
  `website/docs/what-works.md`.

## Acceptance

1. The rows and the table. `dom_selectors::cases_are_the_rows` reads `crates/webidl/selection.txt`
   as `selection()` does (`crates/script/tests/dom_reading.rs:880-894`, the same as
   `tests/bindings.rs:258-275`) and asserts that the set of row names of `const CASES` equals the
   9 rows with the slice `selectors` (`Element.closest`, `Element.matches`,
   `Element.webkitMatchesSelector`, `Element.querySelector`, `Element.querySelectorAll`,
   `Document.querySelector`, `Document.querySelectorAll`, `DocumentFragment.querySelector`,
   `DocumentFragment.querySelectorAll`), and that each entry has a non-empty expression and
   expected value. Each entry runs through `run_case`: a runner of its own, `expression ===
   expected` thrown on a mismatch, the end `ScriptEnd::Ran`, and the recorded limit `None`
   (`runner.document().and_then(|dom| dom.limit())`). The entries together hold a named case for
   each production of the subset (type, universal, `#id`, `.class`, `[a]`, `[a=v]`, compound,
   descendant, child, list), each comparing the nodes found with what the standards say over a
   parsed tree, the section cited in the entry's comment.
2. The meaning of decision 3: `qS` and `qSA` answer descendants only and never `this`, in tree
   order; a combinator sees an ancestor outside `this`; `closest` returns `this` when it matches
   and `null` past the root; `matches` and `webkitMatchesSelector` agree; a subtree removed with
   `removeChild` and a fragment made with `createDocumentFragment` and filled with
   `appendChild` (the test slice of decision 12) are searched as their own trees, so a selector
   that needs an ancestor they no longer have finds nothing; and for
   `<template><p id="t"></p></template>`, `document.querySelector("#t")` is `null` and
   `document.querySelectorAll("*")` holds no element of the template's contents.
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
   The case of attribute values is acceptance 14.
4. The one exception and its two messages (decisions 5 and 6). Each refusal is a `DOMException`
   with name `SyntaxError` and code 12, and no case for a selector expects `NotSupportedError`
   (`git grep -n 'NotSupportedError' -- crates/script/tests/dom_selectors.rs` prints only the
   case of acceptance 9). Each member of the nine has one case of each kind (every one parses
   its argument first).
   a. Not supported: the message starts `selector is not supported by surf: ` and ends with the
      word of the construct, for `a:hover`, `a::before`, `:scope > a`, `a + b`, `a ~ b`,
      `[a^=b]`, `[a=b i]`, `[a=b s]`, `[a=b I]`, `*|a`, `|a`, `[*|a]`, `a:not(b)` and `a:not(b`
      (the `(` closed by the end of the text).
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
   `SyntaxError` of the not-supported kind ending `text over 4096 bytes`, and the call's steps
   rise by no more than the glue's copy and a constant (the slice charges nothing before
   refusing); 512 simple selectors run and 513 are a `SyntaxError` of the not-supported kind
   ending `more than 512 simple selectors`; a text of 1,048,576 bytes, made by one script
   and passed by a second, is refused with the steps of the second under 70,000 (the glue's
   copy is 65,536 at one step per 16 bytes; the glue's price is the base's, decision 7). The
   report records the steps measured.
6. Steps and worst cases (decisions 7 and 8). The implementor measures first and records, in the
   cases' comments, the steps of each case; the assertions below are the floors, the ceilings
   and the ends.
   a. `querySelector("zzz")` over a document of 1,000 elements raises `runner.steps()` by at
      least 2,000 (a visit and a compound test an element).
   b. Over a document of 500 nested `div` and a `span` in the innermost, `span` with the selector
      `x div div div div div div div div span` (no `x` exists) ends `Ran` with the steps of the
      call under 8,000 and answers `null` (the walk 1,010: 506 visits and 504 compound tests;
      the chain read 502; the memo 10 compounds x 503 positions = 5,030 bytes, 314 steps; at
      most 9 x 503 = 4,527 more compound tests: 6,353 in all), where a matcher that backtracks
      does not end. The selector `div div div div div div div div span` answers the span,
      also with the steps under 8,000 (about 5,818).
   c. A document of 100,000 elements and a script `var n = 0; for (;;) { n++;
      document.querySelector("zzz"); }` under a step limit of the test's own (1,000,000) ends
      `limit: steps-per-script`, and a later `runner.evaluate("n")` is at most 5 (a call costs at
      least 200,000 steps, so no more than 5 calls end in it). The case asserts the end and the
      count, no time.
   d. A selector of 512 simple selectors (511 times `div `, then `span`) over 500 nested `div`
      with a `span` in the innermost, under a step limit of the test's own (1,000,000), ends
      `Ran` (no match) or `limit: steps-per-script`, never a stack overflow, and costs at least
      16,000 steps (the memo alone, 512 x 503 bytes, is 16,096); the case states which end after
      the first measure. It asserts the end and the floor, no time.
   e. `for (;;) document.querySelectorAll("*")` over 1,000 elements under a memory limit of 1 MiB
      ends `ScriptEnd::Limit` with `memory` or `steps-per-script`, and a later
      `runner.evaluate("1 + 1")` ends `Ran`.
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
   nine rows have the note of decision 11 and a case reads it from `selection.txt`; `git diff
   --stat` names nothing under `crates/webidl/src/`; `git diff -U0 -- crates/script/src/
   bindings.rs` holds hunks only of the header's digest lines, the nine members' doc comments
   and the last line; the `TestSelectors` types of `tests/bindings.rs`, `adversary_3.rs`,
   `adversary_4.rs` and `adversary_5.rs` compile unchanged; a member of another slice still
   throws `NotSupportedError` naming the member (`Element.attachShadow`, the message of
   decision 13); `git grep -n -F 'el.querySelector' -- crates/script/tests` prints no case that
   expects `NotSupportedError`; `git diff -U0 -- crates/script/src/dom/reads.rs` shows only
   lines that add `pub(super)` or give `Reads` its lifetime and `&'a str` (decision 17);
   `slot task check` exits 0 and `git diff --stat` names only the files of the Change.
10. Public surfaces: the story changes, in its last commit and after merging `main`, every public
    surface its change makes false, each number taken from the `slot task check` log of its tip.
    a. The old strings are read first, after the last merge of `main` (the reading story's merge
       included), by a command run before any change: `git grep -n -e 'passed' -e 'test files'
       -e 'tests:' -- README.md website/docs docs/badges`; the report records its output.
    b. After the last commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges`
       prints nothing for each old string of 10a, and each new number equals the one in the log.
    c. The values of `a14e3b0`, and of the reading unit at `83e6b8a`, which are the same, for
       reading only (the base is later: the reading story's last commit changes them): the test
       count `2394 passed` (`README.md:11`, `docs/badges/make.sh:38`, `docs/badges/tests.svg:1`,
       `:2`, `:10`); `2,397` and `1,845, and 552` (`README.md:285`, `website/docs/what-works.md:
       17`); `2,394 passed and 3 ignored: 1,842 passed` (`website/docs/status.md:31`); the line
       `1845 tests: 1842 passed, 0 failed, 3 ignored` (`README.md:626`,
       `website/docs/how-it-is-built.md:70`); `230 test files` (`README.md:612`,
       `website/docs/how-it-is-built.md:56`).
    d. The gate block (`README.md:596-645`, `website/docs/how-it-is-built.md:40-89`) holds the
       lines of the log, and the paragraph after it that names the gate run's commit
       (`README.md:658`, `website/docs/how-it-is-built.md:102`) names this story's run beside the
       earlier ones.
11. Housekeeping: `git grep -n -F 'scripts-dom-selectors' -- docs/design/scripts.md` prints
    line 5 and the new row of Q4; the entry of `crates/script` in `AGENTS.md` names the slice
    `selectors` and its test count equals the tests of that crate in the log of the tip; the
    epic's correction (decision 14): `awk '/^## Corrections of 2026-11-25/{f=1} f &&
    /scripts-dom-selectors/' .engineering/planning/epic/scripts-in-a-page.md` prints a line.
12. The folder's bounds hold. The test step ends within 120 s and no test runs over 60 s; a test
    over 5 s is named with its time in the report, which also gives the time of `dom_selectors`
    as a whole; no test asserts a time (the only bound on a time a test may hold is a wait for
    an event with a limit at least 10 times the time the test takes alone, `AGENTS.md:1966 @
    a14e3b0`, and a cost is proved by a count of steps); each case makes its own runner and
    parses a few lines, apart from 6c to 6f and the cost module, which build their trees in the
    script or in Rust; no model call and no network request in a test; no Python in anything
    that stays in the repository; no generator with a committed output is added (the existing
    one gains no source: its time is not measured again); a build or test job in the pool ends
    within 10 minutes or names its time with `slot --timeout`, at most 30 minutes, and is not
    started again unchanged; every compiling or
    testing command carries `slot`, and a script that measures or waits starts without the
    prefix; an expensive product (a large tree) is built once per case set, not per case, where
    a case set shares it.
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
       ending `more than 512 simple selectors`.
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
    status --short` shows nothing under `.out/` and `git diff --stat` names no file that holds
    text of the four standards; the report gives each address read, with the date of the text,
    the commit where there is one, the count of the list and each difference from decision 16.
15. Reads and charges (decisions 7 and 17), in `dom_selectors`.
    a. `the_selectors_read_the_tree_only_through_the_reads` reads the source of
       `crates/script/src/dom/selectors.rs` and fails when it holds `tree()`, `tree_mut()`,
       `charge(` or `charge_steps(`, as `dom_reading_cost.rs:593-598` does for `reading.rs`;
       it reads `selector_reads.rs` too and fails when that holds `tree_mut()` or
       `charge_steps(`. The implementor shows it red once, with a string added, and reports the
       output.
    b. A text of 4,096 bytes charges at least 256 steps more than a text of 16 bytes of the same
       shape, and a text of 1,000 tokens at least 1,000 more than one of 10 tokens, the texts
       valid and within the 512 simple selectors.
    c. `[a]` over 100 elements of 1,024 attributes each charges at least 100 x 256 steps for the
       attribute scans beyond the same selector over bare elements; `[` and a name of 4,000
       bytes and `]` over 10 elements of 100 attributes each charges at least 10 x 100 x 250
       steps (the name's bytes / 16 for each attribute passed).
    d. `.a` over 100 elements whose `class` is 64 KiB of one-letter tokens charges at least one
       step per 16 bytes and one per token of each value (at least 100 x (4,096 + 32,768)); in
       quirks mode the same, the fold paid by the bytes' charge.
    e. `[a="v"]` over 100 elements whose `a` is 64 KiB charges at least 100 x 4,096 steps (one
       per 16 bytes of the value, before the compare), and the value of an attribute other than
       `a` is not charged.
    f. The fold of a name (decision 4), charged once at the parse: a selector that is one type
       name of 4,000 bytes charges at least 250 steps more than the same length made of a
       comment around `a` (one token each); the fold is charged once, so the same text on a
       second call charges the same as on the first.
    g. The memo is made late: over 1,000 `div` siblings in `body`, `x y` (no `y`) and `div`
       each charge under 3 steps a node and 400 more (no chain, no memo), and `x div` charges
       at least 3,000 more than `div` does (each `div` matches the rightmost compound, so its
       chain is read and its compounds tested).
    h. Each charge of decision 17 is held by a floor that fails without it: for `selector_text`,
       `selector_token`, `selector_name`, the walk's step, `next_ancestor`, `memo`, the
       compound step of `test_compound` and the attribute and class scans, the case of 6, 15 or
       17 that holds it is named in a comment with its floor and the total it is less than
       when that charge is removed (arithmetic on the printed steps).
16. Nothing is kept (decision 18). `git grep -n -F 'pub struct SurfSelectors;' --
    crates/script/src/dom/selectors.rs` prints a line; the generated enum `SameObject` has the
    four variants it has at `83e6b8a` (`bindings.rs:2051`) and none is one of the nine;
    `querySelectorAll("p") !== querySelectorAll("p")` holds after no write between the calls.
17. The cost module (decision 19), each case at its worst shape with a floor of steps and no
    assertion on a time, printing its steps; a case over 5 s is named with its time in the
    report:
    a. `Element.querySelector("[zzz]")` over 5,000 elements of 100 attributes each: floor
       5,000 x (2 + 25) steps; and `Element.querySelector` with the selector of 6b over a chain
       of 500 `div`: floor 4,000.
    b. `Element.querySelectorAll("*")` over 50,000 elements: floor 50,000 x (2 + 8).
    c. `Element.closest("x div div div div div div div div span")` from a `span` at the bottom of
       500 nested `div`: floor 4,000 (the ancestors read and tested), ends `Ran` or
       `limit: steps-per-script`.
    d. `Element.matches` of an element that has the attributes `a`, `b`, `c` and 1,021 more,
       against `[a][b][c]`: floor 3 x 256 steps.
    e. A list of 512 selectors `a,a,...,a` over 1,000 elements named `b`: floor 1,000 x 512
       steps, ends `Ran`; over 10,000 elements under a step limit of the test's own (1,000,000)
       it ends `limit: steps-per-script` (the case asserts the end, no time).
    f. A selector text at the 4,096-byte bound made of the most tokens the grammar admits within
       the 512 simple selectors (the case counts its tokens): floor of 1 + 256 + its tokens, and
       it ends `Ran`.
    g. One case per remaining member (`Document.querySelector`, `Document.querySelectorAll`,
       `DocumentFragment.querySelector` and `querySelectorAll` over a filled fragment,
       `Element.webkitMatchesSelector`) with the shape of its sibling above, so each of the
       nine rows has a cost case.
    h. The memo (decisions 7 and 8): a chain of 300 nested `div` in `body`, and
       `Element.querySelector` of the selector `x > x > ... > x > div` (199 times `x >`, then
       `div`: 200 compounds, one simple selector each). Each `div` matches the rightmost
       compound, so each makes a chain and a memo of 200 x (k + 2) bytes for the `div` at
       depth k, 9,150,000 bytes in all, 571,875 steps; the child combinator stops each match
       at the parent, so the pair tests are about 2 for each `div`. Floor 550,000 steps; the
       steps without the memo's charge are about 47,000 (the walk, the chain reads 45,450 and
       the pair tests), so the floor fails when the charge is removed. It ends `Ran`.

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
table, the quirks mode, `reads.rs` and the shape of the cases; `story:webidl-bindings` and
`story:html-script-hook` are on `main`. Beside `story:scripts-dom-mutation` and
`story:scripts-dom-attributes`: the three write their own source file and test file, and none
of the other two lists `selection.txt` or `bindings.rs` in its scope. No edge to
`story:scripts-parse-run`: that story calls the slices through `dom::slices(info)` and adds
`DomState::swap_tree` to `prelude.rs` and `bindings.rs`, which this slice does not use.

The files shared with a story beside it or one the drafts below name:

- `crates/script/src/bindings.rs` is generated and never merged by hand. This story and
  `story:scripts-parse-run` both change it (this one the nine doc comments and the digest
  lines, that one a method of `DomState`); whichever merges second runs the generator again
  (`slot cargo run -p surf-webidl --bin bindings`), and `slot task webidl-check` and
  `slot task webidl-slow` hold the result. The same holds for `story:scripts-utf16-length`
  (`prelude.rs`, `bindings.rs`) and for the glue's prices if they merge after this story.
- `crates/script/src/dom/reads.rs`: the reading unit's file. Each slice has its own file for its
  reads: this story's are in the new `selector_reads.rs` and the attributes story's in
  `attribute_reads.rs`. This story changes `reads.rs` by visibility and one widening: `pub(super)`
  on `charge`, `ATTRIBUTES_A_STEP`, `attribute_count`, `Reads`, `scan_for`, `positions` and
  `value_at`, and `Reads`'s lifetime with `&'a str`. The attributes story makes `charge`,
  `ATTRIBUTES_A_STEP` and `attribute_count` `pub(super)` too (three of the same tokens), so
  the two stories' edits of those three lines are the same edit, and the story that merges
  second resolves it by keeping the one. There is no edge between the two stories.
  `story:scripts-dom-sibling-positions` and `story:scripts-dom-collection-names` (drafts) change
  other functions of the file (the sibling and collection reads): a merge conflict there is a
  conflict of position and is resolved by keeping both.
- `crates/script/src/dom/mod.rs`: one line, `mod selector_reads;`; the attributes story adds
  two (`mod attribute_reads;`, `mod attribute_writes;`) in other places of the list of `mod`
  lines, so the second merge keeps all three.
- `crates/script/tests/dom_reading.rs`: the case `other_slices_still_throw` holds three lines,
  one for each slice, at `:2424-2426`; this story changes the third; the stories of `mutation`
  and `attributes` change the first and the second, adjacent lines, so the later merge keeps
  all three changes. Acceptance 8 of `story:scripts-dom-attributes` also names `el.querySelector`
  as a member that still throws `NotSupportedError`: the plan's edit of decision 14 replaces it,
  so that story's test does not fail when this one merges.
- `AGENTS.md`, `docs/design/scripts.md` (also `story:scripts-parse-run`'s) and the public
  surfaces, which the stories beside it rewrite too: whichever merges second merges `main` first
  and takes its numbers from its own log, as the reading story's Placement says of its pair
  with `story:swell-0-9-0`. The public-surface lines the last commit rewrites are those of
  acceptance 10 (`README.md:11, 285, 596-645, 612, 626, 658`, `docs/badges/make.sh:38`,
  `docs/badges/tests.svg:1, 2, 10`, `website/docs/how-it-is-built.md:40-89, 56, 70, 102`,
  `website/docs/status.md:31`, `website/docs/what-works.md:17`).
- `crates/webidl/selection.txt`: this story changes the nine rows of its slice; no other story
  changes those lines.

This story changes `selection.txt` and `bindings.rs`, so it lists both; the reading unit's
merge makes them its base. `story:scripts-html-conformance` waits for this story.

## Why `epic:scripts-in-a-page`

The selector members the epic's acceptance 3 names, and the `querySelector("table")` of the
`needs-script` cases of web-platform-tests (`scripted_foster01.dat`, `epic:396-398`).
