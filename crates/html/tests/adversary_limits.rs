//! Adversarial cases against `story:html-parser-limits` (wave 10): inputs and properties that
//! hold the parser to the unit's claim that "time and memory of a parse are bounded by the
//! limits for every input", to the crate documentation's section "Limits" and to
//! `docs/design/html-parser.md` item 7. Each test names the sentence it holds the code to.
//!
//! The memory tests read a counting global allocator of this test binary (`tests/common`, which
//! counts for each thread). Under cargo-nextest each test runs in a process of its own; under
//! `cargo test` every test takes [`serial`] as well, so that no other test of this file runs
//! while one measures.
//!
//! No test asserts a time (the fix unit of the review): the time test of the review now
//! asserts the steps and the bytes allocated, and the runner's 60 s end is the only time bound.

mod common;

use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard};

use common::{allocated_while, live, peak_while};
use surf_html::limits::{ATTRIBUTE_BYTES, Limit, Limits, NODE_BYTES};
use surf_html::tokenizer::Token;
use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document, parse_document_bytes};

/// The size of the inputs that hold a bound per input byte: 64 KiB, as the worst inputs of
/// `tests/limits.rs` (`story:limit-tests-at-small-limits`).
const SIZE: usize = 1 << 16;

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

fn with_limits(limits: Limits) -> ParseOptions {
    ParseOptions {
        limits,
        ..ParseOptions::default()
    }
}

/// ` a0 a1 …`: `n` attributes with different names and no value (as `tests/limits.rs` makes them).
fn attributes(n: usize) -> String {
    (0..n).map(|k| format!(" a{k}")).collect()
}

/// `n` attributes with the shortest different names (`a` to `z`, then `aa`, `ab`, …), no value.
fn short_attributes(n: usize) -> String {
    let mut names = Vec::new();
    let letters: Vec<char> = ('a'..='z').collect();
    let mut len = 1;
    while names.len() < n {
        let mut index = vec![0usize; len];
        loop {
            names.push(index.iter().map(|&i| letters[i]).collect::<String>());
            if names.len() == n {
                break;
            }
            // The next name of this length, or the end of them.
            let mut at = len;
            loop {
                if at == 0 {
                    break;
                }
                at -= 1;
                index[at] += 1;
                if index[at] < letters.len() {
                    break;
                }
                index[at] = 0;
            }
            if index.iter().all(|&i| i == 0) {
                break;
            }
        }
        len += 1;
    }
    names.iter().map(|name| format!(" {name}")).collect()
}

/// Every node reachable from the root, template contents included, in the order of a walk that
/// keeps its own stack.
fn reachable(tree: &Tree) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        out.push(node);
        pending.extend(tree.children(node).iter().rev());
        if let NodeData::Element {
            template_contents: Some(contents),
            ..
        } = tree.data(node)
        {
            pending.push(*contents);
        }
    }
    out
}

fn bytes_of(data: &NodeData) -> usize {
    match data {
        NodeData::Document | NodeData::DocumentFragment => 0,
        NodeData::Doctype {
            name,
            public_id,
            system_id,
        } => name.len() + public_id.len() + system_id.len(),
        NodeData::Element {
            local_name,
            attributes,
            ..
        } => {
            local_name.len()
                + attributes
                    .iter()
                    .map(|a| {
                        ATTRIBUTE_BYTES
                            + a.prefix.as_ref().map_or(0, String::len)
                            + a.local_name.len()
                            + a.value.len()
                    })
                    .sum::<usize>()
        }
        NodeData::Text(text) => text.len(),
        NodeData::Comment(data) => data.len(),
        NodeData::ProcessingInstruction { target, data } => target.len() + data.len(),
    }
}

/// The tree after any parse, stopped or not, is well formed (the crate documentation, "Limits":
/// "the tree holds every node inserted before that point"; `tree.rs`: "Every node created while
/// parsing stays in the arena"): every reachable node once, each child's parent its parent, the
/// counts inside the limits, and `stored_bytes()` the bytes of the strings of the arena and
/// `ATTRIBUTE_BYTES` for each attribute ([`bytes_of`]), exact when every node of the arena is
/// reachable.
fn check_well_formed(tree: &Tree, limits: &Limits, what: &str) {
    let nodes = reachable(tree);
    let mut seen = HashSet::new();
    for &node in &nodes {
        assert!(seen.insert(node), "{what}: {node:?} reached twice");
        for &child in tree.children(node) {
            assert_eq!(
                tree.parent(child),
                Some(node),
                "{what}: parent of {child:?}"
            );
        }
        if let NodeData::Element {
            template_contents: Some(contents),
            ..
        } = tree.data(node)
        {
            assert_eq!(tree.parent(*contents), None, "{what}: template contents");
        }
    }
    assert_eq!(tree.parent(tree.root()), None, "{what}: root");
    assert!(
        nodes.len() <= tree.node_count(),
        "{what}: reachable past node_count"
    );
    assert!(
        tree.node_count() <= limits.max_nodes.max(1),
        "{what}: node_count {} past max_nodes {}",
        tree.node_count(),
        limits.max_nodes
    );
    assert!(
        tree.stored_bytes() <= limits.max_stored_bytes,
        "{what}: stored_bytes past the limit"
    );
    let strings: usize = nodes.iter().map(|&n| bytes_of(tree.data(n))).sum();
    assert!(
        strings <= tree.stored_bytes(),
        "{what}: strings past stored_bytes"
    );
    if nodes.len() == tree.node_count() {
        assert_eq!(
            strings,
            tree.stored_bytes(),
            "{what}: stored_bytes not exact"
        );
    }
}

// Time: a path the step counter does not count.

/// Holds the code to the unit's claim "time and memory of a parse are bounded by the limits for
/// every input" and to lib.rs "Steps … this one bounds time": a formatting end tag whose element
/// is in the list and on the stack but not in scope (behind a `table`, which pushes no marker)
/// returns at step 4.5 of the adoption agency algorithm after 4 counted steps, but
/// `last_active_named` cloned the element's whole start tag token first (`formatting.rs:38`,
/// `token.clone()`: 1,024 attribute strings). So `</b>` costs O(attributes of the `b`) and
/// 15 MiB of them cost 3.9 million clones of a 1,024-attribute token, inside every limit. The
/// same input with `</i>` (no entry, no clone) is the control.
///
/// Since the fix unit, without a clock: `n` more `</b>` add their 4 steps each and allocate no
/// more than `n` more `</i>` (2 steps each), but for one byte a tag; a clone of the token
/// allocates about 61 KB for each.
#[test]
fn out_of_scope_formatting_end_tags_cost_no_more_than_their_steps() {
    let _serial = serial();
    let prefix = format!("<b{}><table>", attributes(1_024));
    let options = ParseOptions::default();
    let parse = |end_tag: &str, n: usize| {
        let input = format!("{prefix}{}", end_tag.repeat(n));
        let (tree, allocated) = allocated_while(|| parse_document(&input, &options));
        assert_eq!(tree.stopped_by(), None, "{n} `{end_tag}`");
        (tree.steps(), allocated)
    };
    let n = 1_000;
    let (b_steps, b_allocated) = parse("</b>", n);
    let (b2_steps, b2_allocated) = parse("</b>", 2 * n);
    let (i_steps, i_allocated) = parse("</i>", n);
    let (i2_steps, i2_allocated) = parse("</i>", 2 * n);
    let b_more = b2_allocated.saturating_sub(b_allocated);
    let i_more = i2_allocated.saturating_sub(i_allocated);
    println!(
        "measured: {n} more `</b>` {} steps, {b_more} bytes allocated; {n} more `</i>` {} steps, \
         {i_more} bytes allocated",
        b2_steps - b_steps,
        i2_steps - i_steps
    );
    // `</b>`: the list's entry (step 4.3), the stack from the top down to the `b` (4.4, 2), and
    // the `table`, which ends the scope (4.5).
    assert_eq!(b2_steps - b_steps, 4 * n);
    // `</i>`: the list's entry, then "any other end tag" stops at the `table`, special.
    assert_eq!(i2_steps - i_steps, 2 * n);
    assert!(
        b_more <= i_more + n,
        "{n} more `</b>` allocated {b_more} bytes, {n} more `</i>` {i_more}"
    );
}

// Memory.

/// The most heap a parse may hold for each input byte in these tests: the input as code points
/// (4 bytes, held twice while `InputStream::new` copies them), the tree's strings (1 byte, up to
/// 2 with a `String`'s spare capacity), and the tokenizer's tag name and temporary buffer (1 and
/// 4 bytes, doubled the same way): about 20, rounded up. The crate documentation, "Limits":
/// "What a parse holds besides the tree: the input as code points, 4 bytes for each character
/// …, the stack of open elements … and the list of active formatting elements".
const HEAP_PER_INPUT_BYTE: usize = 32;

/// Holds the code to the unit's claim "time and memory of a parse are bounded by the limits for
/// every input" and to the crate documentation's list of what a parse holds besides the tree: in
/// RCDATA, `</` and a run of letters that is not an appropriate end tag is emitted at its end as
/// one character token per letter at once (`machine.rs:1846`), so the tokenizer's queue holds
/// one `Token` (80 bytes) per input byte before the tree builder takes the first. A 16 MiB input
/// holds about 1.3 to 2.6 GB (inferred from the size of `Token` and the queue's doubling). The
/// control, the same length of plain text, stays inside the same bound. Both run at [`SIZE`].
#[test]
fn a_long_end_tag_name_in_rcdata_holds_no_token_per_input_byte() {
    let _serial = serial();
    let options = ParseOptions::default();
    let plain = "x".repeat(SIZE);
    let (tree, plain_peak) = peak_while(|| parse_document(&plain, &options));
    drop(tree);
    let input = format!("<title></{}", "a".repeat(SIZE));
    let (tree, peak) = peak_while(|| parse_document(&input, &options));
    println!(
        "measured: peak heap {peak} bytes for {} input bytes ({} per byte); plain text {plain_peak} \
         ({} per byte); size_of::<Token>() {}",
        input.len(),
        peak / input.len(),
        plain_peak / plain.len(),
        std::mem::size_of::<Token>()
    );
    assert_eq!(tree.stopped_by(), None);
    assert!(
        plain_peak <= HEAP_PER_INPUT_BYTE * plain.len(),
        "control: {plain_peak}"
    );
    assert!(
        peak <= HEAP_PER_INPUT_BYTE * input.len(),
        "peak heap {peak} bytes for {} input bytes",
        input.len()
    );
}

/// Holds the code to lib.rs "The other five limits bound memory" and to the stored-byte limit
/// ("the UTF-8 bytes of every string the tree holds"): the tree's heap should be bounded by its
/// stored bytes and its nodes. An attribute with a short name costs its 80-byte `Attribute` and
/// counts 1 to 3 bytes, so 400 `span` of 1,024 short attributes hold about 35 times their stored
/// bytes. Bound asserted here (mine): 8 bytes of heap per stored byte and 256 per node.
///
/// Since the fix unit the bound is the one `Limits::max_stored_bytes` promises: 2 bytes of heap
/// per stored byte (each attribute counts its 80-byte record) and 3 × `NODE_BYTES` (408) per
/// node. The bound is per stored byte and per node with no constant term, so 40 `span` hold it
/// as 400 did.
#[test]
fn the_heap_of_a_tree_is_bounded_by_its_stored_bytes_and_nodes() {
    let _serial = serial();
    let unit = format!("<span{}></span>", short_attributes(1_024));
    let before = live();
    let tree = {
        let input = unit.repeat(40);
        parse_document(&input, &ParseOptions::default())
    };
    let held = (live() - before).max(0) as usize;
    println!(
        "measured: tree heap {held} bytes, stored_bytes {}, node_count {}, {} heap bytes per \
         stored byte",
        tree.stored_bytes(),
        tree.node_count(),
        held / tree.stored_bytes()
    );
    assert_eq!(tree.stopped_by(), None);
    assert!(
        held <= 2 * tree.stored_bytes() + 3 * NODE_BYTES * tree.node_count(),
        "tree heap {held} bytes, stored_bytes {}, node_count {}",
        tree.stored_bytes(),
        tree.node_count()
    );
}

// Counts at their boundaries.

/// Holds the code to lib.rs "A node or an append that would take `Tree::stored_bytes` past the
/// limit is not made": an append that reaches the limit exactly is made. `x` after the implied
/// `html`, `head` and `body` is 13 stored bytes.
#[test]
fn stored_bytes_exactly_at_the_limit_are_admitted() {
    let _serial = serial();
    let at = parse_document(
        "x",
        &with_limits(Limits {
            max_stored_bytes: 13,
            ..Limits::default()
        }),
    );
    assert_eq!(at.stopped_by(), None);
    assert_eq!(at.stored_bytes(), 13);
    let below = parse_document(
        "x",
        &with_limits(Limits {
            max_stored_bytes: 12,
            ..Limits::default()
        }),
    );
    assert_eq!(below.stopped_by(), Some(Limit::StoredBytes));
    assert_eq!(below.stored_bytes(), 12);
}

/// Holds the code to lib.rs "Steps: every entry visited in a scan of the stack of open elements
/// … The count is checked after each token: the token that takes it past the limit is processed
/// whole, then parsing stops": `<p>` in body visits `body` and `html` for "has a p element in
/// button scope", 2 steps, and `x` none. A limit of 2 is not passed; a limit of 1 is, after the
/// `p` was inserted.
#[test]
fn steps_exactly_at_the_limit_do_not_stop_the_parse() {
    let _serial = serial();
    let at = parse_document(
        "<p>x",
        &with_limits(Limits {
            max_steps: 2,
            ..Limits::default()
        }),
    );
    assert_eq!(at.stopped_by(), None);
    let below = parse_document(
        "<p>x",
        &with_limits(Limits {
            max_steps: 1,
            ..Limits::default()
        }),
    );
    assert_eq!(below.stopped_by(), Some(Limit::Steps));
    let texts = reachable(&below)
        .into_iter()
        .filter(|&n| matches!(below.data(n), NodeData::Text(_)))
        .count();
    assert_eq!(texts, 0);
    // A token of no visit with a limit of 0: nothing passes it.
    let zero = parse_document(
        "x",
        &with_limits(Limits {
            max_steps: 0,
            ..Limits::default()
        }),
    );
    assert_eq!(zero.stopped_by(), None);
}

/// Holds the code to lib.rs "Stored bytes … doctype fields" and `merge_attributes` "A merge that
/// would … pass the stored-byte limit, is not made": the doctype's name and identifiers, a comment,
/// and attributes merged into `html` and `body` count exactly, and a merge that would pass the
/// stored-byte limit is not made.
#[test]
fn doctype_fields_and_merged_attributes_count_as_stored_bytes() {
    let _serial = serial();
    let input = "<!DOCTYPE html PUBLIC \"pub\" \"sys\"><!--c--><p>x<html lang=en><body id=bb>";
    let tree = parse_document(input, &ParseOptions::default());
    assert_eq!(tree.stopped_by(), None);
    // doctype 4 + 3 + 3, comment 1, html 4 + 6, head 4, body 4 + 4, p 1, text 1; since the fix
    // unit each of the two attributes also counts its record, `ATTRIBUTE_BYTES`.
    assert_eq!(
        tree.stored_bytes(),
        10 + 1 + 10 + 4 + 8 + 1 + 1 + 2 * ATTRIBUTE_BYTES
    );
    check_well_formed(&tree, &Limits::default(), input);
    // One byte less: the `body` merge (4 bytes and a record) is not made.
    let limits = Limits {
        max_stored_bytes: 34 + 2 * ATTRIBUTE_BYTES,
        ..Limits::default()
    };
    let tree = parse_document(input, &with_limits(limits));
    assert_eq!(tree.stopped_by(), Some(Limit::StoredBytes));
    assert_eq!(tree.stored_bytes(), 31 + ATTRIBUTE_BYTES);
    check_well_formed(&tree, &limits, input);
}

/// Holds the code to lib.rs "Nodes … a node whose creation would take `Tree::node_count` past
/// the limit is not created (a `template` is two nodes)": a `template` with one node of room
/// left is not created, and with two it is.
#[test]
fn a_template_needs_room_for_two_nodes() {
    let _serial = serial();
    // Document, html, head, body: 4 nodes; the template is 2 more.
    let input = "<body><template>";
    let two = Limits {
        max_nodes: 6,
        ..Limits::default()
    };
    let tree = parse_document(input, &with_limits(two));
    assert_eq!(tree.stopped_by(), None);
    assert_eq!(tree.node_count(), 6);
    check_well_formed(&tree, &two, input);
    let one = Limits {
        max_nodes: 5,
        ..Limits::default()
    };
    let tree = parse_document(input, &with_limits(one));
    assert_eq!(tree.stopped_by(), Some(Limit::Nodes));
    assert_eq!(tree.node_count(), 4);
    check_well_formed(&tree, &one, input);
}

// The step limit as a budget.

/// Holds the code to `docs/design/html-parser.md` item 7, "The default is twice the visits a
/// 16 MiB page of the kind of five large documentation pages would make (at most 3.8 visits per
/// input byte)", and to `tests/limits.rs`' small form (`max_steps` 8 × the input's size, the
/// same visits per input byte): every character in body reconstructs the active formatting
/// elements, which asks whether the last entry is open from the current node down, so text 20
/// elements below an unclosed `<b>` costs 20 steps per byte, and a page of it is cut by the step
/// limit. Severity note: a sloppy page, not an attack.
///
/// Added by the fix unit: the characters cost no step at all (the 20 `div` start tags cost 250,
/// each scanning the stack for a `p` in button scope), so consecutive characters do not pay
/// again for each character. The page is [`SIZE`] bytes.
#[test]
fn text_twenty_elements_below_an_unclosed_b_is_not_cut_by_the_step_limit() {
    let _serial = serial();
    let mut input = String::from("<b>");
    input.push_str(&"<div>".repeat(20));
    input.push_str(&"x".repeat(SIZE - input.len()));
    let tree = parse_document(
        &input,
        &with_limits(Limits {
            max_steps: 8 * SIZE,
            ..Limits::default()
        }),
    );
    assert_eq!(tree.stopped_by(), None);
    assert!(tree.steps() < 1_000, "{} steps", tree.steps());
}

// Properties over seeded inputs.

struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const FRAGMENTS: &[&str] = &[
    "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01//EN\" \"http://x/y\">",
    "<!DOCTYPE html>",
    "<!-- comment -->",
    "<?pi data?>",
    "<html lang=en>",
    "<html lang=de dir=rtl>",
    "<body class=b>",
    "<body id=z>",
    "<head>",
    "<template>",
    "</template>",
    "<table>",
    "<td>",
    "<a href=x>",
    "</a>",
    "<b>",
    "</b>",
    "<i id=1>",
    "</i>",
    "<font color=red size=2>",
    "</font>",
    "<nobr>",
    "<div>",
    "</div>",
    "<p>",
    "</p>",
    "<object>",
    "</object>",
    "<li>",
    "<dd>",
    "<form>",
    "</form>",
    "<frameset>",
    "<frame>",
    "<title>",
    "</title>",
    "<textarea>",
    "<script>",
    "</script>",
    "<plaintext>",
    "<meta charset=utf-8>",
    "<image>",
    "<hr>",
    "x",
    "yz ",
    "\r\n",
    "&amp;",
    "&notin",
    "é",
    "€",
    "<",
    "</",
    "\"",
    "=",
    ">",
];

fn seeded_input(random: &mut Seeded) -> String {
    let len = random.below(600);
    let mut s = String::new();
    loop {
        let fragment = FRAGMENTS[random.below(FRAGMENTS.len())];
        if s.len() + fragment.len() > len {
            return s;
        }
        s.push_str(fragment);
    }
}

/// Small limits, each in turn, so that each stops some of the seeded inputs.
fn small_limits(k: usize) -> Limits {
    let d = Limits::default();
    match k % 7 {
        0 => d,
        1 => Limits {
            max_input_bytes: 200,
            ..d
        },
        2 => Limits { max_depth: 6, ..d },
        3 => Limits { max_nodes: 15, ..d },
        4 => Limits {
            max_attributes: 1,
            ..d
        },
        5 => Limits {
            max_stored_bytes: 60,
            ..d
        },
        _ => Limits { max_steps: 40, ..d },
    }
}

/// Holds the code to the crate documentation, "Limits": "the tree holds every node inserted
/// before that point", and `Tree::stored_bytes` "the UTF-8 bytes of every string the tree holds
/// … in every node of the arena": after 1,000 seeded parses, stopped or not, each tree is well
/// formed ([`check_well_formed`]) and its stored bytes exact when every node is reachable. All
/// 7 outcomes (no stop and each of the six limits) are seen, so a limit that can stop no input
/// turns it red.
#[test]
fn every_stopped_tree_is_well_formed_and_its_stored_bytes_exact() {
    let _serial = serial();
    let mut random = Seeded(0xAD0E_25A1_0000_0007);
    let mut stopped = std::collections::HashMap::new();
    for n in 0..1_000 {
        let input = seeded_input(&mut random);
        let limits = small_limits(n);
        let options = with_limits(limits);
        let tree = if n % 2 == 0 {
            parse_document(&input, &options)
        } else {
            parse_document_bytes(input.as_bytes(), &options).tree
        };
        check_well_formed(&tree, &limits, &format!("input {n} {limits:?}: {input:?}"));
        *stopped.entry(tree.stopped_by()).or_insert(0) += 1;
    }
    println!("measured: stopped by {stopped:?}");
    assert_eq!(stopped.len(), 7, "{stopped:?}");
}

/// Holds the code to lib.rs: `parse_document_bytes` runs "the tokenizer and tree construction
/// over the decoded text, as `parse_document` does", and "`InputBytes` is named unless another
/// limit stopped the parse of the prefix": UTF-8 text through `parse_document` and the same text
/// after a byte order mark through `parse_document_bytes` (the input limit three bytes higher)
/// give the same nodes and the same `stopped_by`, for a cut that ends on a character boundary.
#[test]
fn both_entry_points_give_the_same_tree_and_the_same_limit() {
    let _serial = serial();
    let mut random = Seeded(0xAD0E_25A1_0000_0011);
    for n in 0..2_000 {
        let input = seeded_input(&mut random);
        let limits = small_limits(n);
        if !input.is_char_boundary(input.len().min(limits.max_input_bytes)) {
            continue;
        }
        let text = parse_document(&input, &with_limits(limits));
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(input.as_bytes());
        let bytes_limits = Limits {
            max_input_bytes: limits.max_input_bytes.saturating_add(3),
            ..limits
        };
        let parsed = parse_document_bytes(&bytes, &with_limits(bytes_limits));
        assert_eq!(parsed.encoding, Some("UTF-8"), "input {n}");
        let data = |tree: &Tree| -> Vec<NodeData> {
            reachable(tree)
                .into_iter()
                .map(|node| tree.data(node).clone())
                .collect()
        };
        assert_eq!(
            text.stopped_by(),
            parsed.tree.stopped_by(),
            "input {n} {limits:?}: {input:?}"
        );
        assert_eq!(
            data(&text),
            data(&parsed.tree),
            "input {n} {limits:?}: {input:?}"
        );
        assert_eq!(text.node_count(), parsed.tree.node_count(), "input {n}");
        assert_eq!(text.stored_bytes(), parsed.tree.stored_bytes(), "input {n}");
    }
}

/// Holds the code to the crate documentation, "Limits": "Every limit below ends a parse with a
/// defined result; none of them is a hang, an unbounded allocation or a panic": each limit at
/// 0, 1, 2, 3 and `usize::MAX`, the others at their defaults, through both entry points, gives
/// a well-formed tree without a panic.
#[test]
fn limits_of_zero_one_and_max_end_without_a_panic() {
    let _serial = serial();
    let inputs = [
        "",
        "x",
        "<!DOCTYPE html><html a=b><head><title>t</title></head><body><p>x</p>\
         <template>y</template><b><i>z</b>w</i></body></html>",
        "<a><b><i><div>x</a>y",
        "<frameset><frame></frameset>",
        "<meta charset=windows-1252><p>é",
    ];
    let d = Limits::default();
    for value in [0, 1, 2, 3, usize::MAX] {
        let each = [
            Limits {
                max_input_bytes: value,
                ..d
            },
            Limits {
                max_depth: value,
                ..d
            },
            Limits {
                max_nodes: value,
                ..d
            },
            Limits {
                max_attributes: value,
                ..d
            },
            Limits {
                max_stored_bytes: value,
                ..d
            },
            Limits {
                max_steps: value,
                ..d
            },
        ];
        for limits in each {
            for input in inputs {
                let options = with_limits(limits);
                let what = format!("{limits:?}: {input:?}");
                let tree = std::panic::catch_unwind(|| parse_document(input, &options))
                    .unwrap_or_else(|_| panic!("parse_document panicked: {what}"));
                check_well_formed(&tree, &limits, &what);
                let tree =
                    std::panic::catch_unwind(|| parse_document_bytes(input.as_bytes(), &options))
                        .unwrap_or_else(|_| panic!("parse_document_bytes panicked: {what}"))
                        .tree;
                check_well_formed(&tree, &limits, &what);
            }
        }
    }
}
