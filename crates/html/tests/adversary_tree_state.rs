//! Adversarial cases of what `story:scripts-dom-reading` adds to the tree: the write counter
//! ([`Tree::writes`]), raised by a write that changes a child list and by nothing else, over
//! random sequences of the public writes; and the document's mode for the doctypes of the
//! parser's quirks tables.

use surf_html::{
    Context, Namespace, NodeData, NodeId, ParseOptions, QuirksMode, Tree, parse_document,
    parse_fragment,
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

/// The parent and the children of each node the test knows.
fn lists(tree: &Tree, known: &[NodeId]) -> Vec<(Option<NodeId>, Vec<NodeId>)> {
    known
        .iter()
        .map(|&node| (tree.parent(node), tree.children(node).to_vec()))
        .collect()
}

fn learn(tree: &Tree, known: &mut Vec<NodeId>, node: NodeId) {
    for n in tree.descendants(node) {
        if !known.contains(&n) {
            known.push(n);
        }
    }
    if !known.contains(&node) {
        known.push(node);
    }
}

fn is_text(tree: &Tree, node: NodeId) -> bool {
    matches!(tree.data(node), NodeData::Text(_))
}

/// Random sequences of the public writes of `Tree` from a fixed seed: after each write, the
/// child lists and parents of every node the test knows are compared with those before it; the
/// counter moved if and only if one of them changed (decision 10). A refused write changes no
/// list. Every mismatch is listed: a list changed with the counter left is a stale kept
/// collection; a counter raised with no list changed is a refill for nothing.
#[test]
fn the_counter_moves_if_and_only_if_a_child_list_changed() {
    let page = "<!DOCTYPE html><html><head><title>t</title></head><body>\
                <div id=a>one<p>two</p>three<!--c--><b>four</b></div><ul><li>x</li><li>y</li></ul>\
                <svg><circle></circle></svg>five</body></html>";
    let mut mismatches = Vec::new();
    let mut counts = std::collections::BTreeMap::<&str, (usize, usize)>::new();
    for seed in 0..40_u64 {
        let mut tree = parse_document(page, &ParseOptions::default());
        let mut known = Vec::new();
        learn(&tree, &mut known, tree.root());
        let mut rng = Lcg(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        for _ in 0..60 {
            let before = lists(&tree, &known);
            let writes = tree.writes();
            let pick = |rng: &mut Lcg, known: &[NodeId]| known[rng.below(known.len())];
            let a = pick(&mut rng, &known);
            let b = pick(&mut rng, &known);
            let c = pick(&mut rng, &known);
            let kind = rng.below(19);
            let (name, ok, made): (&str, bool, Option<NodeId>) = match kind {
                0 => ("append_child", tree.append_child(a, b).is_ok(), None),
                1 => (
                    "insert_before",
                    tree.insert_before(a, b, Some(c)).is_ok(),
                    None,
                ),
                2 => (
                    "insert_before none",
                    tree.insert_before(a, b, None).is_ok(),
                    None,
                ),
                3 => ("replace_child", tree.replace_child(a, b, c).is_ok(), None),
                4 => ("move_before", tree.move_before(a, b, Some(c)).is_ok(), None),
                5 => (
                    "move_before none",
                    tree.move_before(a, b, None).is_ok(),
                    None,
                ),
                6 => ("remove_node", tree.remove_node(a).is_ok(), None),
                7 => ("adopt_node", tree.adopt_node(a).is_ok(), None),
                8 => {
                    let made = tree.clone_node(a, rng.below(2) == 0).ok();
                    ("clone_node", made.is_some(), made)
                }
                9 => ("normalize", tree.normalize(a).is_ok(), None),
                10 => {
                    let made = if is_text(&tree, a) {
                        tree.split_text(a, rng.below(3)).ok()
                    } else {
                        None
                    };
                    ("split_text", made.is_some(), made)
                }
                11 => {
                    let value = ["", "v", "w x"][rng.below(3)];
                    (
                        "set_text_content",
                        tree.set_text_content(a, value).is_ok(),
                        None,
                    )
                }
                12 => ("set_data", tree.set_data(a, "d").is_ok(), None),
                13 => ("append_data", tree.append_data(a, "").is_ok(), None),
                14 => (
                    "set_attribute",
                    tree.set_attribute(a, "k", "v").is_ok(),
                    None,
                ),
                15 => {
                    let made = tree.create_document_fragment().ok();
                    ("create_document_fragment", made.is_some(), made)
                }
                16 => {
                    let made = tree.create_text_node("t").ok();
                    ("create_text_node", made.is_some(), made)
                }
                17 => {
                    let made = tree.create_element_ns(Namespace::Html, "em").ok();
                    ("create_element_ns", made.is_some(), made)
                }
                _ => (
                    "remove_attribute",
                    tree.remove_attribute(a, "k").is_ok(),
                    None,
                ),
            };
            let after_known = lists(&tree, &known);
            // A node the write made is new to the test: its own child lists (a deep clone's,
            // a split text's place among its siblings is already in a known list) count too.
            let made_lists = made.is_some_and(|made| {
                tree.descendants(made)
                    .into_iter()
                    .chain([made])
                    .any(|n| !tree.children(n).is_empty())
            });
            let changed = before != after_known || made_lists;
            let moved = tree.writes() != writes;
            let entry = counts.entry(name).or_default();
            entry.0 += 1;
            if ok {
                entry.1 += 1;
            }
            if changed != moved || (!ok && changed) {
                mismatches.push(format!(
                    "seed {seed} {name} ok {ok}: lists changed {changed}, counter moved {moved} \
                     (a {:?} b {:?} c {:?})",
                    tree.data(a),
                    tree.data(b),
                    tree.data(c)
                ));
            }
            if let Some(made) = made {
                learn(&tree, &mut known, made);
            }
        }
    }
    eprintln!("writes tried (all, successful): {counts:?}");
    eprintln!("{} mismatches", mismatches.len());
    for line in mismatches.iter().take(20) {
        eprintln!("{line}");
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches, first: {:?}",
        mismatches.len(),
        mismatches.first()
    );
}

/// An empty document fragment appended or inserted changes no child list; the counter must
/// stay (decision 10: "raised ... by nothing else").
#[test]
fn an_empty_fragment_inserted_leaves_the_counter() {
    let mut tree = parse_document("<p></p>", &ParseOptions::default());
    let p = tree
        .descendants(tree.root())
        .into_iter()
        .find(
            |&n| matches!(tree.data(n), NodeData::Element { local_name, .. } if local_name == "p"),
        )
        .expect("p");
    let fragment = tree.create_document_fragment().expect("a fragment");
    let writes = tree.writes();
    let children = tree.children(p).to_vec();
    tree.append_child(p, fragment).expect("appended");
    assert_eq!(tree.children(p), children.as_slice());
    assert_eq!(
        tree.writes(),
        writes,
        "the counter moved with no child list changed"
    );
}

fn mode(input: &str) -> QuirksMode {
    parse_document(input, &ParseOptions::default()).quirks_mode()
}

/// The mode for the doctypes of the parser's quirks tables (HTML 13.2.6.4.1), a doctype after a
/// comment, a second doctype, a fragment, a tree's clone (brief 14).
#[test]
fn the_mode_of_each_quirks_table() {
    let cases = [
        ("", QuirksMode::Quirks),
        ("<!DOCTYPE html>", QuirksMode::NoQuirks),
        (
            "<!DOCTYPE html SYSTEM \"about:legacy-compat\">",
            QuirksMode::NoQuirks,
        ),
        ("<!DOCTYPE>", QuirksMode::Quirks),
        ("<!DOCTYPE htm>", QuirksMode::Quirks),
        ("<!--c--><!DOCTYPE html>", QuirksMode::NoQuirks),
        (
            "<!--c--><!DOCTYPE html PUBLIC \"HTML\">",
            QuirksMode::Quirks,
        ),
        (
            "<!DOCTYPE html><!DOCTYPE html PUBLIC \"-//W3O//DTD W3 HTML Strict 3.0//EN//\">",
            QuirksMode::NoQuirks,
        ),
        ("<p><!DOCTYPE html>", QuirksMode::Quirks),
        (
            "<!DOCTYPE html PUBLIC \"-//W3O//DTD W3 HTML Strict 3.0//EN//\">",
            QuirksMode::Quirks,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-/W3C/DTD HTML 4.0 Transitional/EN\">",
            QuirksMode::Quirks,
        ),
        ("<!DOCTYPE html PUBLIC \"html\">", QuirksMode::Quirks),
        (
            "<!DOCTYPE html SYSTEM \"http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd\">",
            QuirksMode::Quirks,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.0 Transitional//EN\">",
            QuirksMode::Quirks,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//w3c//dtd html 3.2//en\">",
            QuirksMode::Quirks,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01 Frameset//EN\">",
            QuirksMode::Quirks,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01 Transitional//EN\">",
            QuirksMode::Quirks,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01 Frameset//EN\" \"http://www.w3.org/TR/html4/frameset.dtd\">",
            QuirksMode::LimitedQuirks,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01 Transitional//EN\" \"http://www.w3.org/TR/html4/loose.dtd\">",
            QuirksMode::LimitedQuirks,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Frameset//EN\">",
            QuirksMode::LimitedQuirks,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\">",
            QuirksMode::LimitedQuirks,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01//EN\">",
            QuirksMode::NoQuirks,
        ),
    ];
    let mut wrong = Vec::new();
    for (input, expected) in cases {
        let got = mode(input);
        if got != expected {
            wrong.push(format!("{input}: {got:?}, expected {expected:?}"));
        }
    }
    let tree = parse_document("<p>", &ParseOptions::default());
    if tree.clone().quirks_mode() != QuirksMode::Quirks {
        wrong.push("a clone of a quirks tree".to_string());
    }
    let context = Context {
        namespace: Namespace::Html,
        local_name: "body".to_string(),
    };
    for input in ["<!DOCTYPE html PUBLIC \"HTML\"><p>", "x"] {
        let fragment = parse_fragment(input, &context, &ParseOptions::default());
        if fragment.quirks_mode() != QuirksMode::NoQuirks {
            wrong.push(format!("fragment {input}: {:?}", fragment.quirks_mode()));
        }
    }
    let quirks = parse_document("<p>x</p>", &ParseOptions::default());
    let standard = parse_document("<!DOCTYPE html><p>x</p>", &ParseOptions::default());
    if quirks == standard {
        wrong.push("trees with other doctypes compare equal".to_string());
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
