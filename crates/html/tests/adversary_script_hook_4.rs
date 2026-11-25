//! Adversarial cases of the fourth review of `story:html-script-hook`, against fix round 3:
//! no node record keeps a depth; the parser keeps one per entry of the stack of open elements
//! and reads the entries again after a hook run that moved or removed an element and after an
//! adoption agency iteration; the stack has no bound of its own. Each test names what it holds
//! the unit to: the rulings of fix round 3, the crate documentation's "Limits" and the
//! documentation of `src/limits.rs`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::mpsc;
use std::time::Duration;

use surf_html::limits::{Limit, Limits};
use surf_html::{
    Namespace, NodeData, NodeId, NotRun, ParseOptions, ScriptHook, Tree, parse_document_with_hook,
};

fn deep(max_depth: usize) -> ParseOptions {
    ParseOptions {
        scripting_disabled: false,
        limits: Limits {
            max_depth,
            ..Limits::default()
        },
    }
}

fn is_element(tree: &Tree, node: NodeId) -> bool {
    matches!(tree.data(node), NodeData::Element { .. })
}

fn contents_of(tree: &Tree, template: NodeId) -> Option<NodeId> {
    match tree.data(template) {
        NodeData::Element {
            template_contents: Some(contents),
            ..
        } => Some(*contents),
        _ => None,
    }
}

fn is_script_element(tree: &Tree, node: NodeId) -> bool {
    matches!(tree.data(node), NodeData::Element {
        namespace: Namespace::Html | Namespace::Svg,
        local_name,
        ..
    } if local_name == "script")
}

/// Every node reached from `starts` through children, template contents and parents, each
/// once.
fn reach(tree: &Tree, starts: &[NodeId]) -> BTreeSet<NodeId> {
    let mut seen = BTreeSet::new();
    let mut pending = starts.to_vec();
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        pending.extend(tree.children(node).iter().copied());
        pending.extend(contents_of(tree, node));
        pending.extend(tree.parent(node));
    }
    seen
}

/// Every node reached from the root through children and template contents, each once.
fn connected(tree: &Tree) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        out.push(node);
        pending.extend(contents_of(tree, node));
        pending.extend(tree.children(node).iter().rev());
    }
    out
}

/// The deepest node of the trees that hold the nodes reached from `known` and the root, each
/// tree counted from its own top (the document, or a node with no parent that is no
/// template's contents), a template counted for the nodes of its contents.
fn deepest_held(tree: &Tree, known: &[NodeId]) -> usize {
    let mut starts = known.to_vec();
    starts.push(tree.root());
    let reached = reach(tree, &starts);
    let hosted: HashSet<NodeId> = reached
        .iter()
        .filter_map(|&n| contents_of(tree, n))
        .collect();
    let mut most = 0;
    for &top in &reached {
        if tree.parent(top).is_some() || hosted.contains(&top) {
            continue;
        }
        let mut pending = vec![(top, 0usize)];
        let mut seen = HashSet::new();
        while let Some((node, above)) = pending.pop() {
            if !seen.insert(node) {
                continue;
            }
            let depth = above + usize::from(is_element(tree, node));
            most = most.max(depth);
            pending.extend(tree.children(node).iter().map(|&c| (c, depth)));
            if let Some(contents) = contents_of(tree, node) {
                pending.push((contents, depth));
            }
        }
    }
    most
}

/// A chain of `length` new HTML `div` elements, each the only child of the one before: the
/// first and the last.
fn chain(tree: &mut Tree, length: usize) -> (NodeId, NodeId) {
    let first = tree.create_element_ns(Namespace::Html, "div").unwrap();
    let mut last = first;
    for _ in 1..length {
        let next = tree.create_element_ns(Namespace::Html, "div").unwrap();
        tree.append_child(last, next).unwrap();
        last = next;
    }
    (first, last)
}

/// A hook that moves the script's parent to the end of the `body` and takes the element that
/// held it out of the tree: every element of the markup stays open, and all but the last one
/// have no parent. Two writes per run.
struct Unhang {
    runs: usize,
}

impl ScriptHook for Unhang {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
        self.runs += 1;
        let element = tree.parent(script).unwrap();
        let held_by = tree.parent(element).unwrap();
        let html = tree.children(tree.root())[0];
        let body = tree.children(html)[1];
        tree.append_child(body, element).unwrap();
        if held_by != body {
            tree.remove_node(held_by).unwrap();
        }
    }

    fn skip(&mut self, _: &Tree, _: NodeId, _: NotRun) {}
}

// The cases below held on the tip they were written against (`4ce6829`).

/// The depth limit bounds the tree, not the stack (ruling 1 of fix round 2, kept in round 3):
/// with the `Unhang` hook, 2,000 `span` stay open at a limit of 8, and the parse ends with no
/// limit named, every script run, and no tree deeper than 4 (`html`, `body`, the last `span`,
/// its `script`).
#[test]
fn a_stack_far_longer_than_the_depth_limit_keeps_the_tree_inside_it() {
    const ROUNDS: usize = 2_000;
    let input = format!("<body>{}", "<span><script>x</script>".repeat(ROUNDS));
    let mut hook = Unhang { runs: 0 };
    let tree = parse_document_with_hook(&input, &deep(8), &mut hook);
    assert_eq!(tree.stopped_by(), None);
    assert_eq!(hook.runs, ROUNDS);
    let spans: Vec<NodeId> = connected(&tree)
        .into_iter()
        .filter(|&n| is_element(&tree, n))
        .collect();
    assert!(deepest_held(&tree, &spans) <= 4);
}

/// A hook write that takes `a`, the script's parent, 5 deep, to a shallower place, and the
/// depth `a` has there in its own tree.
type Lift = (&'static str, fn(&mut Tree, NodeId), usize);

/// Every write of the hook's interface that takes an open element to a shallower place marks
/// it, and the parser reads the depths of its entries again (ruling 1 of fix round 3: the
/// depth checked is the real one): `a` (5 deep) holds the script; after the write, markup puts
/// `i` elements under `a` up to the limit of 12 exactly, and the parse ends with no limit
/// named and no tree deeper than 12. With `a`'s old depth kept, it would stop 3 or more `i`
/// early.
#[test]
fn every_write_that_lifts_an_open_element_is_read_again() {
    const N: usize = 12;
    fn html(tree: &Tree) -> NodeId {
        tree.children(tree.root())[0]
    }
    let lifts: [Lift; 9] = [
        (
            "append_child",
            |t, a| t.append_child(html(t), a).unwrap(),
            2,
        ),
        (
            "insert_before",
            |t, a| {
                let head = t.children(html(t))[0];
                t.insert_before(html(t), a, Some(head)).unwrap();
            },
            2,
        ),
        (
            "move_before",
            |t, a| t.move_before(html(t), a, None).unwrap(),
            2,
        ),
        (
            "replace_child",
            |t, a| {
                let head = t.children(html(t))[0];
                t.replace_child(html(t), a, head).unwrap();
            },
            2,
        ),
        ("remove_node", |t, a| t.remove_node(a).unwrap(), 1),
        ("adopt_node", |t, a| t.adopt_node(a).unwrap(), 1),
        (
            "a fragment",
            |t, a| {
                let fragment = t.create_document_fragment().unwrap();
                t.append_child(fragment, a).unwrap();
            },
            1,
        ),
        (
            "template contents",
            |t, a| {
                let template = t.create_element_ns(Namespace::Html, "template").unwrap();
                t.append_child(html(t), template).unwrap();
                let contents = contents_of(t, template).unwrap();
                t.append_child(contents, a).unwrap();
            },
            3,
        ),
        (
            "set_text_content",
            |t, a| {
                let outer = t.parent(t.parent(a).unwrap()).unwrap();
                t.set_text_content(outer, "t").unwrap();
            },
            2,
        ),
    ];
    let mut wrong = Vec::new();
    for (what, lift, depth) in lifts {
        let input = format!(
            "<body><div><div><div id=a><script>x</script>{}",
            "<i>".repeat(N - depth)
        );
        let kept = std::rc::Rc::new(std::cell::Cell::new(None));
        let keep = std::rc::Rc::clone(&kept);
        let mut hook = Lifter(Some(Box::new(move |tree: &mut Tree, script: NodeId| {
            let a = tree.parent(script).unwrap();
            keep.set(Some(a));
            lift(tree, a);
        })));
        let tree = parse_document_with_hook(&input, &deep(N), &mut hook);
        let a = kept.get().unwrap();
        let deepest = deepest_held(&tree, &[a]);
        if tree.stopped_by().is_some() || deepest != N {
            wrong.push(format!(
                "{what}: stopped by {:?}, {deepest} deep",
                tree.stopped_by()
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A hook that makes one write, in its first run.
struct Lifter(Option<OneWrite>);

/// The write of a [`Lifter`].
type OneWrite = Box<dyn FnOnce(&mut Tree, NodeId)>;

impl ScriptHook for Lifter {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
        if let Some(write) = self.0.take() {
            write(tree, script);
        }
    }

    fn skip(&mut self, _: &Tree, _: NodeId, _: NotRun) {}
}

/// A small xorshift generator: the same seed gives the same case.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

/// The markup of the random cases.
const PIECES: &[&str] = &[
    "<b>",
    "</b>",
    "<i>",
    "</i>",
    "<a>",
    "</a>",
    "<nobr>",
    "<p>",
    "</p>",
    "<div>",
    "</div>",
    "<span>",
    "</span>",
    "<table>",
    "</table>",
    "<tr>",
    "<td>",
    "</td>",
    "<caption>",
    "<template>",
    "</template>",
    "<select>",
    "</select>",
    "<option>",
    "<button>",
    "<selectedcontent>",
    "<svg>",
    "</svg>",
    "<math>",
    "<mi>",
    "<foreignObject>",
    "<font color=red>",
    "<h1>",
    "<em>",
    "x",
    "</body>",
    "<object>",
    "<frameset>",
    "<head>",
    "</head>",
    "<b id=1>",
    "<u>",
    "<script>s</script>",
    "<script>s</script>",
    "<script>s</script>",
    "<script>s</script>",
    "<svg><script>v</script>",
    "<template><script>t</script></template>",
    "<table><script>u</script>",
];

/// A write whose answer the random hook does not read.
fn tried<T>(_: Result<T, surf_html::WriteError>) {}

/// A hook that writes at random, most of all on the open elements above the script: it moves
/// them up and down, takes them out of the tree, puts them under each other and into template
/// contents. At each call it records the deepest node of the trees it knows.
struct Random {
    rng: Rng,
    calls: Vec<(NodeId, Option<NotRun>)>,
    seen: BTreeSet<NodeId>,
    deepest: usize,
}

impl Random {
    fn look(&mut self, tree: &Tree) {
        let known: Vec<NodeId> = self.seen.iter().copied().collect();
        let now = deepest_held(tree, &known);
        self.deepest = self.deepest.max(now);
    }

    fn write(&mut self, tree: &mut Tree, script: NodeId) {
        let mut above = Vec::new();
        let mut at = script;
        while let Some(up) = tree.parent(at) {
            above.push(up);
            at = up;
        }
        let pool: Vec<NodeId> = self.seen.iter().copied().collect();
        let near = |rng: &mut Rng| {
            if above.is_empty() || rng.below(3) == 0 {
                pool[rng.below(pool.len())]
            } else {
                above[rng.below(above.len())]
            }
        };
        let a = near(&mut self.rng);
        let b = near(&mut self.rng);
        let html = tree.children(tree.root()).first().copied();
        match self.rng.below(9) {
            0 | 1 => {
                if let Some(html) = html {
                    tried(tree.append_child(html, a));
                }
            }
            2 | 3 => tried(tree.remove_node(a)),
            4 | 5 => tried(tree.append_child(a, b)),
            6 => {
                if let Ok(new) = tree.create_element_ns(Namespace::Html, "template") {
                    self.seen.extend(reach(tree, &[new]));
                    tried(tree.append_child(b, new));
                    if let Some(contents) = contents_of(tree, new) {
                        tried(tree.append_child(contents, a));
                    }
                }
            }
            7 => {
                let (first, last) = chain(tree, 1 + self.rng.below(5));
                self.seen.extend(reach(tree, &[first]));
                tried(tree.append_child(last, a));
                tried(tree.append_child(b, first));
            }
            _ => tried(tree.set_text_content(a, "t")),
        }
    }
}

impl ScriptHook for Random {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
        self.calls.push((script, None));
        self.seen.insert(script);
        self.seen.extend(connected(tree));
        self.look(tree);
        for _ in 0..1 + self.rng.below(4) {
            self.write(tree, script);
        }
        self.look(tree);
    }

    fn skip(&mut self, tree: &Tree, script: NodeId, reason: NotRun) {
        self.calls.push((script, Some(reason)));
        self.seen.insert(script);
        self.look(tree);
    }
}

/// What one random parse left: its input, its stop, the deepest node of the trees the hook
/// knew at the end and at any call, what is wrong with its calls, and the hooks that ran.
struct Case {
    input: String,
    stopped: Option<Limit>,
    end: usize,
    deepest: usize,
    wrong: Vec<String>,
}

fn random_case(seed: u64, n: usize) -> Case {
    let mut rng = Rng::new(seed);
    let mut input = String::new();
    for _ in 0..4 + rng.below(40) {
        input.push_str(PIECES[rng.below(PIECES.len())]);
    }
    let mut hook = Random {
        rng,
        calls: Vec::new(),
        seen: BTreeSet::new(),
        deepest: 0,
    };
    let tree = parse_document_with_hook(&input, &deep(n), &mut hook);
    let known: Vec<NodeId> = hook.seen.iter().copied().collect();
    let end = deepest_held(&tree, &known);
    let mut wrong = Vec::new();
    let mut called = HashMap::new();
    for &(node, _) in &hook.calls {
        *called.entry(node).or_insert(0) += 1;
    }
    for (node, count) in &called {
        if *count > 1 {
            wrong.push(format!("{node:?} called {count} times"));
        }
    }
    for node in connected(&tree) {
        if is_script_element(&tree, node) && !called.contains_key(&node) {
            wrong.push(format!("{node:?} in the document had no call"));
        }
    }
    Case {
        input,
        stopped: tree.stopped_by(),
        end,
        deepest: hook.deepest.max(end),
        wrong,
    }
}

/// Random markup and a hook that moves the open elements above its script up, down, out of
/// the tree, under each other and into template contents, at limits 6, 8 and 10, 9,000
/// parses: no tree the hook knew is deeper than the limit, at the end or at any call, and
/// every script in the document had one call. Ruling 1 of fix round 3 lets an adoption agency
/// iteration stop with no element at the limit, when its move would pass it; the test prints
/// how many such stops it met. (Seed 3980 at 10 is one: the first outer iteration of its
/// `</b>` would put an SVG `script` 11 deep; its second iteration moves it up again, so at a
/// limit of 11 the tree ends 10 deep.)
#[test]
fn random_writes_on_open_elements_keep_the_limit_and_one_call() {
    let (send, receive) = mpsc::channel();
    const CASES: u64 = 9_000;
    std::thread::spawn(move || {
        for seed in 0..CASES {
            let n = [6, 8, 10][(seed % 3) as usize];
            let result = std::panic::catch_unwind(|| {
                let case = random_case(seed, n);
                let mut wrong = case.wrong.clone();
                if case.deepest > n {
                    wrong.push(format!("{} deep at a limit of {n}", case.deepest));
                }
                let quiet = case.stopped == Some(Limit::Depth) && case.end < n;
                (case.input, wrong, quiet)
            });
            let report = match result {
                Ok((_, wrong, quiet)) if wrong.is_empty() => (None, quiet),
                Ok((input, wrong, quiet)) => (
                    Some(format!("seed {seed}, limit {n}, `{input}`: {wrong:#?}")),
                    quiet,
                ),
                Err(_) => (Some(format!("seed {seed} panicked")), false),
            };
            if send.send(report).is_err() {
                return;
            }
        }
    });
    let mut wrong = Vec::new();
    let mut quiet = 0;
    for seed in 0..CASES {
        match receive.recv_timeout(Duration::from_secs(5)) {
            Ok((report, q)) => {
                wrong.extend(report);
                quiet += usize::from(q);
            }
            Err(_) => panic!("seed {seed} did not return within 5 s"),
        }
    }
    eprintln!("{quiet} Depth stops with no element at the limit");
    assert!(
        wrong.is_empty(),
        "{} of {CASES}: {:#?}",
        wrong.len(),
        &wrong[..wrong.len().min(5)]
    );
}
