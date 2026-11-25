//! Adversarial cases of the write counter ([`Tree::writes`]) of `story:scripts-dom-reading`,
//! second review: the public writes of `Tree` the first random test did not draw (the data
//! writes past `set_data` and `append_data`, the namespaced and toggled attributes,
//! `import_node`, comments and processing instructions, a template, a fragment with children
//! inserted and put in place of a child), and a hooked parse whose script writes the tree
//! between the parser's own writes.

use surf_html::{
    Namespace, NodeData, NodeId, NotRun, ParseOptions, ScriptHook, Tree, parse_document,
    parse_document_with_hook,
};

/// A fixed-seed generator of the test's own (no dependency).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next()).unwrap_or(0) % n.max(1)
    }
}

/// The parent and the children of each node the test knows, in its order.
type Lists = Vec<(Option<NodeId>, Vec<NodeId>)>;

/// The [`Lists`] of the nodes `known`.
fn lists(tree: &Tree, known: &[NodeId]) -> Lists {
    known
        .iter()
        .map(|&node| (tree.parent(node), tree.children(node).to_vec()))
        .collect()
}

fn learn(tree: &Tree, known: &mut Vec<NodeId>, node: NodeId) {
    for n in tree.descendants(node).into_iter().chain([node]) {
        if !known.contains(&n) {
            known.push(n);
        }
    }
}

fn is_character_data(tree: &Tree, node: NodeId) -> bool {
    matches!(
        tree.data(node),
        NodeData::Text(_) | NodeData::Comment(_) | NodeData::ProcessingInstruction { .. }
    )
}

/// One random public write of `Tree` on the nodes `a`, `b`, `c`: its name, whether it was
/// taken, and the node it made, if any.
fn write(
    tree: &mut Tree,
    rng: &mut Lcg,
    a: NodeId,
    b: NodeId,
    c: NodeId,
) -> (&'static str, bool, Option<NodeId>) {
    match rng.below(16) {
        0 => {
            let offset = rng.below(4);
            let count = rng.below(4);
            let ok =
                is_character_data(tree, a) && tree.replace_data(a, offset, count, "rz").is_ok();
            ("replace_data", ok, None)
        }
        1 => {
            let offset = rng.below(3);
            (
                "insert_data",
                tree.insert_data(a, offset, "\u{1F600}").is_ok(),
                None,
            )
        }
        2 => {
            let offset = rng.below(3);
            let count = rng.below(3);
            (
                "delete_data",
                tree.delete_data(a, offset, count).is_ok(),
                None,
            )
        }
        3 => (
            "set_attribute_ns",
            tree.set_attribute_ns(a, Some(Namespace::XLink), "xlink:href", "v")
                .is_ok(),
            None,
        ),
        4 => (
            "remove_attribute_ns",
            tree.remove_attribute_ns(a, Some(Namespace::XLink), "href")
                .is_ok(),
            None,
        ),
        5 => {
            let force = [None, Some(true), Some(false)][rng.below(3)];
            (
                "toggle_attribute",
                tree.toggle_attribute(a, "t", force).is_ok(),
                None,
            )
        }
        6 => {
            let made = tree.import_node(a, rng.below(2) == 0).ok();
            ("import_node", made.is_some(), made)
        }
        7 => {
            let made = tree.create_comment("c").ok();
            ("create_comment", made.is_some(), made)
        }
        8 => {
            let made = tree.create_processing_instruction("t", "d").ok();
            ("create_processing_instruction", made.is_some(), made)
        }
        9 => {
            let made = tree.create_element_ns(Namespace::Html, "template").ok();
            ("create_element_ns template", made.is_some(), made)
        }
        10 => {
            // A fragment with two children, appended: its children move in one write.
            let Ok(fragment) = tree.create_document_fragment() else {
                return ("fragment appended", false, None);
            };
            let first = tree.append_child(fragment, b).is_ok();
            let second = tree.append_child(fragment, c).is_ok();
            let ok = tree.append_child(a, fragment).is_ok();
            ("fragment appended", ok && (first || second), Some(fragment))
        }
        11 => {
            let Ok(fragment) = tree.create_document_fragment() else {
                return ("fragment in place of a child", false, None);
            };
            let _ = tree.append_child(fragment, b);
            let ok = tree.replace_child(a, fragment, c).is_ok();
            ("fragment in place of a child", ok, Some(fragment))
        }
        12 => (
            "replace_child with itself",
            tree.replace_child(a, b, b).is_ok(),
            None,
        ),
        13 => (
            "insert_before itself",
            tree.insert_before(a, b, Some(b)).is_ok(),
            None,
        ),
        14 => (
            "move_before itself",
            tree.move_before(a, b, Some(b)).is_ok(),
            None,
        ),
        _ => {
            let made = tree.clone_node(a, true).ok();
            ("clone_node deep", made.is_some(), made)
        }
    }
}

/// Random sequences of the writes above from a fixed seed: after each, the child lists and
/// parents of every node the test knows are compared with those before it; the counter moved
/// if and only if one of them changed.
#[test]
fn the_counter_over_the_writes_the_first_test_did_not_draw() {
    let page = "<!DOCTYPE html><html><head><title>t</title><template><b>in</b></template></head><body>\
                <div id=a>one<p>two</p>three<!--c--><b>four</b></div><ul><li>x</li><li>y</li></ul>\
                <svg><circle></circle><a xlink:href=\"u\">l</a></svg>five\u{1F600}</body></html>";
    let mut mismatches = Vec::new();
    let mut counts = std::collections::BTreeMap::<&str, (usize, usize)>::new();
    for seed in 0..60_u64 {
        let mut tree = parse_document(page, &ParseOptions::default());
        let mut known = Vec::new();
        learn(&tree, &mut known, tree.root());
        let mut rng = Lcg(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        for _ in 0..60 {
            let before = lists(&tree, &known);
            let writes = tree.writes();
            let a = known[rng.below(known.len())];
            let b = known[rng.below(known.len())];
            let c = known[rng.below(known.len())];
            let (name, ok, made) = write(&mut tree, &mut rng, a, b, c);
            let after = lists(&tree, &known);
            let made_lists = made.is_some_and(|made| {
                tree.descendants(made)
                    .into_iter()
                    .chain([made])
                    .any(|n| !tree.children(n).is_empty())
            });
            let changed = before != after || made_lists;
            let moved = tree.writes() != writes;
            let entry = counts.entry(name).or_default();
            entry.0 += 1;
            if ok {
                entry.1 += 1;
            }
            // A made node's own lists count as changed only when they were written; a fragment
            // that lost its children in the same composite write is no change of a known list.
            if before != after && !moved {
                mismatches.push(format!(
                    "seed {seed} {name} ok {ok}: a known list changed, the counter stayed"
                ));
            }
            if moved && !changed {
                mismatches.push(format!(
                    "seed {seed} {name} ok {ok}: the counter moved, no list changed"
                ));
            }
            if let Some(made) = made {
                learn(&tree, &mut known, made);
            }
        }
    }
    eprintln!("writes tried (all, taken): {counts:?}");
    for line in mismatches.iter().take(20) {
        eprintln!("{line}");
    }
    assert!(
        mismatches
            .iter()
            .all(|line| !line.contains("the counter stayed")),
        "{} mismatches, first: {:?}",
        mismatches.len(),
        mismatches.first()
    );
}

/// A hook whose script writes the tree (moves, removals, an empty fragment) between the
/// parser's writes; at each call it compares every child list with the one at the end of the
/// last call, and the counter with its value then: a list changed by the parser in between
/// with the counter left would be a stale kept collection.
#[derive(Default)]
struct Writer {
    known: Vec<NodeId>,
    last: Option<(u64, Lists)>,
    calls: usize,
    stale: Vec<String>,
    rng: u64,
}

impl Writer {
    fn audit(&mut self, tree: &Tree, when: &str) {
        learn(tree, &mut self.known, tree.root());
        if let Some((writes, before)) = &self.last {
            let now = lists(tree, &self.known[..before.len()]);
            if &now != before && tree.writes() == *writes {
                self.stale.push(format!(
                    "{when}: a list changed since the last call, counter left"
                ));
            }
        }
    }

    fn remember(&mut self, tree: &Tree) {
        learn(tree, &mut self.known, tree.root());
        self.last = Some((tree.writes(), lists(tree, &self.known)));
    }
}

impl ScriptHook for Writer {
    fn run(&mut self, tree: &mut Tree, _script: NodeId) {
        self.calls += 1;
        self.audit(tree, &format!("call {}", self.calls));
        let mut rng = Lcg(self.rng.wrapping_add(self.calls as u64) | 1);
        for _ in 0..6 {
            let before = lists(tree, &self.known);
            let writes = tree.writes();
            let a = self.known[rng.below(self.known.len())];
            let b = self.known[rng.below(self.known.len())];
            let c = self.known[rng.below(self.known.len())];
            let (name, _, made) = write(tree, &mut rng, a, b, c);
            if lists(tree, &self.known) != before && tree.writes() == writes {
                self.stale
                    .push(format!("call {} {name}: counter left", self.calls));
            }
            if let Some(made) = made {
                learn(tree, &mut self.known, made);
            }
        }
        self.remember(tree);
    }

    fn skip(&mut self, _tree: &Tree, _script: NodeId, _reason: NotRun) {}
}

#[test]
fn the_counter_over_a_hooked_parse_with_writes() {
    let options = ParseOptions {
        scripting_disabled: false,
        ..ParseOptions::default()
    };
    let page = "<table><tr><td>a<script>1</script><b>x<p>y</b>z</p></td></tr>q<script>2</script></table>\
                <i><b>q<div>r<script>3</script></i>s</div><select><option>o<script>4</script></select>\
                <template><script>5</script>t</template><p>tail<script>6</script>end";
    let mut stale = Vec::new();
    let mut calls = 0;
    for seed in 0..40_u64 {
        let mut hook = Writer {
            rng: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15),
            ..Writer::default()
        };
        let tree = parse_document_with_hook(page, &options, &mut hook);
        hook.audit(&tree, "the end of the parse");
        calls += hook.calls;
        stale.extend(
            hook.stale
                .into_iter()
                .map(|line| format!("seed {seed} {line}")),
        );
    }
    eprintln!("{calls} hook calls, {} stale", stale.len());
    assert!(calls > 0);
    assert!(stale.is_empty(), "{stale:#?}");
}
