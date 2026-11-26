//! The script hook and the tree's public write surface (story `html-script-hook`): with
//! scripting enabled the tree builder calls a [`ScriptHook`] at the end tag of each
//! parser-inserted script, to run it or to report it skipped with a word of the design note's
//! Q1 (`docs/design/scripts.md`), and parsing goes on over the tree the hook left. Every write of
//! the public surface is held to the limits of the parse and answers the one it reached instead
//! of writing.

use std::collections::HashSet;

use surf_html::limits::{ATTRIBUTE_BYTES, Limit, Limits};
use surf_html::{
    Attribute, Namespace, NodeData, NodeId, NotRun, ParseOptions, ScriptHook, Tree, WriteError,
    parse_document, parse_document_with_hook,
};

/// What a hook was called with: `run`, or `skip` with its reason, for the element of an `id`.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Call {
    Run(String),
    Skip(String, NotRun),
}

/// What a recording hook does in `run` after it recorded the call: it gets the calls so far.
type Action = Box<dyn FnMut(&mut Tree, NodeId, &[(NodeId, Call)])>;

/// A hook that records each call with its element, and runs an action in `run`.
#[derive(Default)]
struct Recorder {
    calls: Vec<(NodeId, Call)>,
    on_run: Option<Action>,
}

impl Recorder {
    fn with(action: impl FnMut(&mut Tree, NodeId, &[(NodeId, Call)]) + 'static) -> Recorder {
        Recorder {
            calls: Vec::new(),
            on_run: Some(Box::new(action)),
        }
    }

    fn calls(&self) -> Vec<Call> {
        self.calls.iter().map(|(_, call)| call.clone()).collect()
    }
}

impl ScriptHook for Recorder {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
        self.calls.push((script, Call::Run(id_of(tree, script))));
        if let Some(action) = self.on_run.as_mut() {
            action(tree, script, &self.calls);
        }
    }

    fn skip(&mut self, tree: &Tree, script: NodeId, reason: NotRun) {
        self.calls
            .push((script, Call::Skip(id_of(tree, script), reason)));
    }
}

/// A hook that does nothing.
struct Nothing;

impl ScriptHook for Nothing {
    fn run(&mut self, _: &mut Tree, _: NodeId) {}

    fn skip(&mut self, _: &Tree, _: NodeId, _: NotRun) {}
}

fn scripted() -> ParseOptions {
    ParseOptions {
        scripting_disabled: false,
        ..ParseOptions::default()
    }
}

fn scripted_with(limits: Limits) -> ParseOptions {
    ParseOptions {
        scripting_disabled: false,
        limits,
    }
}

fn attribute<'t>(tree: &'t Tree, node: NodeId, name: &str) -> Option<&'t str> {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes
            .iter()
            .find(|a| a.namespace.is_none() && a.local_name == name)
            .map(|a| a.value.as_str()),
        _ => None,
    }
}

fn id_of(tree: &Tree, node: NodeId) -> String {
    attribute(tree, node, "id").unwrap_or_default().to_string()
}

/// A node as the tests name it: an element's local name, text in quotes, or the kind.
fn name(tree: &Tree, node: NodeId) -> String {
    match tree.data(node) {
        NodeData::Document => "document".to_string(),
        NodeData::DocumentFragment => "fragment".to_string(),
        NodeData::Doctype { name, .. } => format!("<!DOCTYPE {name}>"),
        NodeData::Element { local_name, .. } => local_name.clone(),
        NodeData::Text(text) => format!("{text:?}"),
        NodeData::Comment(data) => format!("<!--{data}-->"),
        NodeData::ProcessingInstruction { target, .. } => format!("<?{target}>"),
    }
}

fn child_names(tree: &Tree, node: NodeId) -> Vec<String> {
    tree.children(node).iter().map(|&n| name(tree, n)).collect()
}

/// The element of an `id`, also in template contents and in a subtree out of the tree that
/// hangs under a node the walk reaches.
fn by_id(tree: &Tree, id: &str) -> NodeId {
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        if attribute(tree, node, "id") == Some(id) {
            return node;
        }
        pending.extend(tree.children(node));
        if let NodeData::Element {
            template_contents: Some(contents),
            ..
        } = tree.data(node)
        {
            pending.push(*contents);
        }
    }
    panic!("no element with the id {id}");
}

fn the_element(tree: &Tree, local_name: &str) -> NodeId {
    let found: Vec<NodeId> = tree
        .descendants(tree.root())
        .into_iter()
        .filter(
            |&n| matches!(tree.data(n), NodeData::Element { local_name: l, .. } if l == local_name),
        )
        .collect();
    assert_eq!(found.len(), 1, "one {local_name}");
    found[0]
}

/// The 16 JavaScript MIME type essences of MIME Sniffing 4.6.
const ESSENCES: [&str; 16] = [
    "application/ecmascript",
    "application/javascript",
    "application/x-ecmascript",
    "application/x-javascript",
    "text/ecmascript",
    "text/javascript",
    "text/javascript1.0",
    "text/javascript1.1",
    "text/javascript1.2",
    "text/javascript1.3",
    "text/javascript1.4",
    "text/javascript1.5",
    "text/jscript",
    "text/livescript",
    "text/x-ecmascript",
    "text/x-javascript",
];

/// One script element of a row of Q1: its `id`, its attributes, its text, and what the hook
/// sees for it (`None`: `run`).
struct Row {
    id: String,
    attributes: String,
    text: &'static str,
    not_run: Option<NotRun>,
}

fn row(id: &str, attributes: &str, text: &'static str, not_run: Option<NotRun>) -> Row {
    Row {
        id: id.to_string(),
        attributes: attributes.to_string(),
        text,
        not_run,
    }
}

fn scripts(rows: &[Row]) -> String {
    rows.iter()
        .map(|r| format!("<script id={} {}>{}</script>", r.id, r.attributes, r.text))
        .collect()
}

/// A document holding one element of each row of Q1 that the tree builder decides, and the
/// calls a recording hook sees for it, in document order.
fn every_row() -> (String, Vec<Call>) {
    let mut head = vec![
        row("plain", "", "a", None),
        row("type-empty", "type=\"\"", "a", None),
    ];
    for (i, essence) in ESSENCES.iter().enumerate() {
        // As written, in upper case, and with ASCII whitespace around it.
        let value = match i % 3 {
            0 => essence.to_string(),
            1 => essence.to_ascii_uppercase(),
            _ => format!(" \t{essence}\n\x0C "),
        };
        head.push(row(
            &format!("essence-{i}"),
            &format!("type=\"{value}\""),
            "a",
            None,
        ));
    }
    head.extend([
        row(
            "parameter",
            "type=\"text/javascript; charset=utf-8\"",
            "a",
            Some(NotRun::Type),
        ),
        row("module", "type=MODULE", "a", Some(NotRun::Module)),
        row("importmap", "type=importmap", "{}", Some(NotRun::Importmap)),
        row(
            "speculationrules",
            "type=speculationrules",
            "{}",
            Some(NotRun::Speculationrules),
        ),
        row("data-block", "type=text/template", "a", Some(NotRun::Type)),
        row("json", "type=application/json", "{}", Some(NotRun::Type)),
        row("language-empty", "language=\"\"", "a", None),
        row("language-javascript", "language=JavaScript1.2", "a", None),
        row(
            "language-vbscript",
            "language=vbscript",
            "a",
            Some(NotRun::Type),
        ),
        row(
            "type-over-language",
            "type=text/javascript language=vbscript",
            "a",
            None,
        ),
        row(
            "type-over-language-not",
            "type=text/plain language=javascript",
            "a",
            Some(NotRun::Type),
        ),
    ]);
    let body = vec![
        row("src-empty", "src=a.js", "", Some(NotRun::Src)),
        row("src-text", "src=a.js", "a", Some(NotRun::Src)),
        row("src-blank", "src=\"\"", "", Some(NotRun::Src)),
        row("empty", "", "", Some(NotRun::Empty)),
        row("nomodule", "nomodule", "a", Some(NotRun::Nomodule)),
        row(
            "nomodule-module",
            "nomodule type=module",
            "a",
            Some(NotRun::Module),
        ),
        row("async-defer", "async defer", "a", None),
        row(
            "event-document",
            "event=onload for=document",
            "a",
            Some(NotRun::Event),
        ),
        row(
            "event-onclick",
            "event=onclick for=window",
            "a",
            Some(NotRun::Event),
        ),
        row(
            "event-onload",
            "event=\" OnLoad() \" for=\" Window \"",
            "a",
            None,
        ),
        row("event-alone", "event=onclick", "a", None),
    ];
    let html = format!(
        "<!DOCTYPE html><html><head>{}</head><body>{}\
         <template><script id=disconnected>a</script></template>\
         <svg><script id=svg>a</script><script id=svg-self-closing /></svg></body></html>",
        scripts(&head),
        scripts(&body)
    );
    let mut calls: Vec<Call> = head
        .iter()
        .chain(&body)
        .map(|r| match r.not_run {
            None => Call::Run(r.id.clone()),
            Some(reason) => Call::Skip(r.id.clone(), reason),
        })
        .collect();
    calls.extend([
        Call::Skip("disconnected".to_string(), NotRun::Disconnected),
        Call::Skip("svg".to_string(), NotRun::Svg),
        Call::Skip("svg-self-closing".to_string(), NotRun::Svg),
    ]);
    (html, calls)
}

/// Acceptance 1.
#[test]
fn each_row_of_q1_gets_its_call_in_document_order() {
    let (html, expected) = every_row();
    let mut hook = Recorder::default();
    parse_document_with_hook(&html, &scripted(), &mut hook);
    assert_eq!(hook.calls(), expected);
    let seen: HashSet<NotRun> = hook
        .calls()
        .into_iter()
        .filter_map(|c| match c {
            Call::Skip(_, reason) => Some(reason),
            Call::Run(_) => None,
        })
        .collect();
    let all_but_eof = [
        NotRun::Empty,
        NotRun::Src,
        NotRun::Type,
        NotRun::Module,
        NotRun::Importmap,
        NotRun::Speculationrules,
        NotRun::Disconnected,
        NotRun::Nomodule,
        NotRun::Event,
        NotRun::Svg,
    ];
    assert_eq!(seen, HashSet::from(all_but_eof));
    // With scripting disabled the hook sees no call.
    let mut hook = Recorder::default();
    parse_document_with_hook(&html, &ParseOptions::default(), &mut hook);
    assert_eq!(hook.calls(), []);
}

/// The words of Q1's line `reasons:` but `inserted`.
#[test]
fn each_reason_has_its_word() {
    let words: Vec<&str> = [
        NotRun::Empty,
        NotRun::Src,
        NotRun::Type,
        NotRun::Module,
        NotRun::Importmap,
        NotRun::Speculationrules,
        NotRun::Disconnected,
        NotRun::Nomodule,
        NotRun::Event,
        NotRun::Svg,
        NotRun::Eof,
    ]
    .iter()
    .map(|r| r.word())
    .collect();
    assert_eq!(
        words,
        [
            "empty",
            "src",
            "type",
            "module",
            "importmap",
            "speculationrules",
            "disconnected",
            "nomodule",
            "event",
            "svg",
            "eof"
        ]
    );
}

/// Acceptance 2.
#[test]
fn eof_reports_open_script() {
    let input = "<script id=open>x = 1";
    let mut hook = Recorder::default();
    let tree = parse_document_with_hook(input, &scripted(), &mut hook);
    assert_eq!(hook.calls(), [Call::Skip("open".to_string(), NotRun::Eof)]);
    assert_eq!(tree, parse_document(input, &scripted()));
    assert_eq!(
        child_names(&tree, by_id(&tree, "open")),
        ["\"x = 1\"".to_string()]
    );
}

/// Acceptance 3.
#[test]
fn no_second_call() {
    let input = "<body><script id=empty></script><script id=data type=text/template>a</script>\
                 <template><script id=held>a</script></template>\
                 <script id=writer>w</script><p>after</p><script id=last>z</script>";
    let mut hook = Recorder::with(|tree, writer, calls| {
        if id_of(tree, writer) != "writer" {
            return;
        }
        let [empty, data, held] = [0, 1, 2].map(|i| calls[i].0);
        let text = tree.create_text_node("late").unwrap();
        tree.append_child(empty, text).unwrap();
        tree.set_attribute(data, "type", "text/javascript").unwrap();
        let body = tree.parent(writer).unwrap();
        tree.append_child(body, held).unwrap();
        let inserted = tree.create_element_ns(Namespace::Html, "script").unwrap();
        tree.set_attribute(inserted, "id", "inserted").unwrap();
        tree.set_text_content(inserted, "i").unwrap();
        tree.append_child(body, inserted).unwrap();
    });
    let tree = parse_document_with_hook(input, &scripted(), &mut hook);
    assert_eq!(
        hook.calls(),
        [
            Call::Skip("empty".to_string(), NotRun::Empty),
            Call::Skip("data".to_string(), NotRun::Type),
            Call::Skip("held".to_string(), NotRun::Disconnected),
            Call::Run("writer".to_string()),
            Call::Run("last".to_string()),
        ]
    );
    // The writes were made: each element now has what would run it in a browser.
    let body = the_element(&tree, "body");
    assert_eq!(
        child_names(&tree, body),
        [
            "script", "script", "template", "script", "script", "script", "p", "script"
        ]
    );
    assert_eq!(
        attribute(&tree, by_id(&tree, "data"), "type"),
        Some("text/javascript")
    );
    assert_eq!(tree.parent(by_id(&tree, "held")), Some(body));
    assert_eq!(
        child_names(&tree, by_id(&tree, "empty")),
        ["\"late\"".to_string()]
    );
}

/// Acceptance 4.
#[test]
fn no_op_hook_same_tree() {
    let (rows, _) = every_row();
    let documents = [
        "<head><script>a</script></head><body>x</body>",
        "<body><p>a<script>b</script>c</p>",
        "<table><script>a</script><tr><td>x</td></tr></table>",
        "<table><tr><script>a</script><td>b",
        "<select><script>a</script><option>x</select>",
        "<template><script>a</script></template><script>b</script>",
        "<svg><script>a</script><script/></svg><math><script>b</script></math>",
        "<b><script>a</script>x</b>y<p>z",
        "<script>x = 1",
        rows.as_str(),
    ];
    for document in documents {
        let without = parse_document(document, &scripted());
        assert_eq!(
            parse_document_with_hook(document, &scripted(), &mut Nothing),
            without,
            "{document}"
        );
        let mut recorder = Recorder::default();
        assert_eq!(
            parse_document_with_hook(document, &scripted(), &mut recorder),
            without,
            "{document}"
        );
    }
}

/// Acceptance 5.
#[test]
fn hook_appends_after_the_script() {
    let input = "<body><script id=s>a</script><p>after</p>";
    let mut hook = Recorder::with(|tree, script, _| {
        let parent = tree.parent(script).unwrap();
        let span = tree.create_element_ns(Namespace::Html, "span").unwrap();
        tree.append_child(parent, span).unwrap();
    });
    let tree = parse_document_with_hook(input, &scripted(), &mut hook);
    let body = the_element(&tree, "body");
    assert_eq!(child_names(&tree, body), ["script", "span", "p"]);
    assert_eq!(child_names(&tree, the_element(&tree, "p")), ["\"after\""]);
}

/// Acceptance 6. The script holds text, so that it is run: an empty one is skipped for
/// `empty`, and `skip` cannot write.
#[test]
fn hook_removes_an_open_ancestor_of_the_script() {
    let input = "<body><div id=d><script id=s>r</script>x</div><p>after</p>";
    let mut hook = Recorder::with(|tree, script, _| {
        let div = tree.parent(script).unwrap();
        tree.remove_node(div).unwrap();
    });
    let tree = parse_document_with_hook(input, &scripted(), &mut hook);
    assert_eq!(hook.calls(), [Call::Run("s".to_string())]);
    let body = the_element(&tree, "body");
    assert_eq!(child_names(&tree, body), ["p"]);
    // The `div` is out of the tree: it is reached from the script.
    let div = tree.parent(hook.calls[0].0).unwrap();
    assert_eq!(id_of(&tree, div), "d");
    assert_eq!(tree.parent(div), None);
    assert_eq!(child_names(&tree, div), ["script", "\"x\""]);
    assert_eq!(tree.stopped_by(), None);
}

/// A `body` start tag after a script merges into the attributes the script left: no name twice,
/// and a name the script removed is added again.
#[test]
fn hook_attribute_writes_meet_the_body_merge() {
    let input = "<body><body data-x=1><script id=s>r</script><body class=b data-x=2 id=i>";
    let mut hook = Recorder::with(|tree, script, _| {
        let body = tree.parent(script).unwrap();
        tree.set_attribute(body, "class", "a").unwrap();
        tree.remove_attribute(body, "data-x").unwrap();
    });
    let tree = parse_document_with_hook(input, &scripted(), &mut hook);
    let body = the_element(&tree, "body");
    let names: Vec<(String, String)> = attributes_of(&tree, body)
        .into_iter()
        .map(|a| (a.local_name, a.value))
        .collect();
    assert_eq!(
        names,
        [
            ("class".to_string(), "a".to_string()),
            ("data-x".to_string(), "2".to_string()),
            ("id".to_string(), "i".to_string()),
        ]
    );
}

// Acceptance 7: one case for each operation of the write surface, with each limit it can reach.

/// The document the write cases start from.
const BASE: &str = "<!DOCTYPE html><body><p id=p>text</p><template id=t><b>in</b></template>";

fn base() -> Tree {
    parse_document(BASE, &scripted())
}

/// The base document parsed with the limits `limits` gives from the counts of the base.
fn base_with(limits: impl Fn(&Tree) -> Limits) -> Tree {
    let tree = parse_document(BASE, &scripted_with(limits(&base())));
    assert_eq!(tree.stopped_by(), None, "the base parses whole");
    tree
}

/// The base with room for `nodes` nodes more and nothing else changed.
fn nodes_left(nodes: usize) -> Tree {
    base_with(|t| Limits {
        max_nodes: t.node_count() + nodes,
        ..Limits::default()
    })
}

/// The base with room for `bytes` stored bytes more.
fn bytes_left(bytes: usize) -> Tree {
    base_with(|t| Limits {
        max_stored_bytes: t.stored_bytes() + bytes,
        ..Limits::default()
    })
}

/// A body holding `p` with text, with room for elements three deep (`html`, `body`, `p`).
fn three_deep() -> Tree {
    let tree = parse_document(
        "<p id=p>text</p>",
        &scripted_with(Limits {
            max_depth: 3,
            ..Limits::default()
        }),
    );
    assert_eq!(tree.stopped_by(), None);
    tree
}

/// The write answers `expected` and leaves the tree as it was.
fn refuses<T: std::fmt::Debug>(
    mut tree: Tree,
    expected: WriteError,
    write: impl FnOnce(&mut Tree) -> Result<T, WriteError>,
) {
    let before = tree.clone();
    assert_eq!(write(&mut tree).unwrap_err(), expected);
    assert_eq!(tree, before);
}

fn text_of(tree: &Tree, node: NodeId) -> &str {
    match tree.data(node) {
        NodeData::Text(data) | NodeData::Comment(data) => data,
        NodeData::ProcessingInstruction { data, .. } => data,
        _ => panic!("not character data"),
    }
}

#[test]
fn write_create_element() {
    let mut tree = base();
    let count = tree.node_count();
    let span = tree.create_element_ns(Namespace::Html, "span").unwrap();
    assert_eq!(tree.parent(span), None);
    assert_eq!(name(&tree, span), "span");
    let template = tree.create_element_ns(Namespace::Html, "template").unwrap();
    assert_eq!(tree.node_count(), count + 3, "a template is two nodes");
    assert!(matches!(
        tree.data(template),
        NodeData::Element {
            template_contents: Some(_),
            ..
        }
    ));
    let rect = tree.create_element_ns(Namespace::Svg, "rect").unwrap();
    assert!(matches!(
        tree.data(rect),
        NodeData::Element {
            namespace: Namespace::Svg,
            ..
        }
    ));
    refuses(nodes_left(0), WriteError::Limit(Limit::Nodes), |t| {
        t.create_element_ns(Namespace::Html, "span")
    });
    refuses(nodes_left(1), WriteError::Limit(Limit::Nodes), |t| {
        t.create_element_ns(Namespace::Html, "template")
    });
    refuses(bytes_left(3), WriteError::Limit(Limit::StoredBytes), |t| {
        t.create_element_ns(Namespace::Html, "span")
    });
}

#[test]
fn write_create_text_node() {
    let mut tree = base();
    let text = tree.create_text_node("abc").unwrap();
    assert_eq!(*tree.data(text), NodeData::Text("abc".to_string()));
    refuses(nodes_left(0), WriteError::Limit(Limit::Nodes), |t| {
        t.create_text_node("abc")
    });
    refuses(bytes_left(2), WriteError::Limit(Limit::StoredBytes), |t| {
        t.create_text_node("abc")
    });
}

#[test]
fn write_create_comment() {
    let mut tree = base();
    let comment = tree.create_comment("abc").unwrap();
    assert_eq!(*tree.data(comment), NodeData::Comment("abc".to_string()));
    refuses(nodes_left(0), WriteError::Limit(Limit::Nodes), |t| {
        t.create_comment("abc")
    });
    refuses(bytes_left(2), WriteError::Limit(Limit::StoredBytes), |t| {
        t.create_comment("abc")
    });
}

#[test]
fn write_create_processing_instruction() {
    let mut tree = base();
    let pi = tree.create_processing_instruction("t", "d").unwrap();
    assert_eq!(
        *tree.data(pi),
        NodeData::ProcessingInstruction {
            target: "t".to_string(),
            data: "d".to_string()
        }
    );
    refuses(nodes_left(0), WriteError::Limit(Limit::Nodes), |t| {
        t.create_processing_instruction("t", "d")
    });
    refuses(bytes_left(1), WriteError::Limit(Limit::StoredBytes), |t| {
        t.create_processing_instruction("t", "d")
    });
}

#[test]
fn write_create_document_fragment() {
    let mut tree = base();
    let fragment = tree.create_document_fragment().unwrap();
    assert_eq!(*tree.data(fragment), NodeData::DocumentFragment);
    refuses(nodes_left(0), WriteError::Limit(Limit::Nodes), |t| {
        t.create_document_fragment()
    });
}

#[test]
fn write_insert_before() {
    let mut tree = base();
    let p = by_id(&tree, "p");
    let first = tree.children(p)[0];
    let em = tree.create_element_ns(Namespace::Html, "em").unwrap();
    tree.insert_before(p, em, Some(first)).unwrap();
    assert_eq!(child_names(&tree, p), ["em", "\"text\""]);
    // A node with a parent leaves it first.
    tree.insert_before(p, first, Some(em)).unwrap();
    assert_eq!(child_names(&tree, p), ["\"text\"", "em"]);
    // A reference that is not a child, a cycle, a parent that holds no children, a document.
    let other = tree.create_element_ns(Namespace::Html, "i").unwrap();
    let body = tree.parent(p).unwrap();
    refuses(tree.clone(), WriteError::NotFound, |t| {
        t.insert_before(p, other, Some(body))
    });
    refuses(tree.clone(), WriteError::HierarchyRequest, |t| {
        t.insert_before(em, body, None)
    });
    refuses(tree.clone(), WriteError::HierarchyRequest, |t| {
        t.insert_before(p, p, None)
    });
    refuses(tree.clone(), WriteError::HierarchyRequest, |t| {
        t.insert_before(first, other, None)
    });
    let root = tree.root();
    refuses(tree.clone(), WriteError::HierarchyRequest, |t| {
        t.insert_before(p, root, None)
    });
    refuses(tree.clone(), WriteError::HierarchyRequest, |t| {
        t.insert_before(root, other, None)
    });
    let text = tree.create_text_node("x").unwrap();
    refuses(tree.clone(), WriteError::HierarchyRequest, |t| {
        t.insert_before(root, text, None)
    });
    // Into template contents through its template: the walk up crosses the host.
    let t = by_id(&tree, "t");
    let contents = match tree.data(t) {
        NodeData::Element {
            template_contents: Some(c),
            ..
        } => *c,
        _ => panic!("a template"),
    };
    let inner = tree.children(contents)[0];
    refuses(tree.clone(), WriteError::HierarchyRequest, |tr| {
        tr.insert_before(inner, t, None)
    });
    // Depth: `p` is the third element down; an element in it would be the fourth.
    let mut deep = three_deep();
    let p = by_id(&deep, "p");
    let text = deep.create_text_node("t").unwrap();
    deep.insert_before(p, text, None).unwrap();
    let span = deep.create_element_ns(Namespace::Html, "span").unwrap();
    refuses(deep, WriteError::Limit(Limit::Depth), |t| {
        t.insert_before(p, span, None)
    });
}

#[test]
fn write_append_child() {
    let mut tree = base();
    let body = the_element(&tree, "body");
    let div = tree.create_element_ns(Namespace::Html, "div").unwrap();
    tree.append_child(body, div).unwrap();
    assert_eq!(child_names(&tree, body), ["p", "template", "div"]);
    // A subtree two deep under `body`, the second element down: four deep.
    let mut deep = three_deep();
    let body = the_element(&deep, "body");
    let div = deep.create_element_ns(Namespace::Html, "div").unwrap();
    let span = deep.create_element_ns(Namespace::Html, "span").unwrap();
    deep.append_child(div, span).unwrap();
    refuses(deep, WriteError::Limit(Limit::Depth), |t| {
        t.append_child(body, div)
    });
}

#[test]
fn write_insert_a_fragments_children() {
    let mut tree = base();
    let body = the_element(&tree, "body");
    let p = by_id(&tree, "p");
    let fragment = tree.create_document_fragment().unwrap();
    for name in ["i", "u"] {
        let element = tree.create_element_ns(Namespace::Html, name).unwrap();
        tree.append_child(fragment, element).unwrap();
    }
    tree.insert_before(body, fragment, Some(p)).unwrap();
    assert_eq!(child_names(&tree, body), ["i", "u", "p", "template"]);
    assert_eq!(tree.children(fragment), []);
    // A fragment that holds an element, into `p` three deep.
    let mut deep = three_deep();
    let p = by_id(&deep, "p");
    let fragment = deep.create_document_fragment().unwrap();
    let span = deep.create_element_ns(Namespace::Html, "span").unwrap();
    deep.append_child(fragment, span).unwrap();
    refuses(deep, WriteError::Limit(Limit::Depth), |t| {
        t.append_child(p, fragment)
    });
}

#[test]
fn write_remove_node() {
    let mut tree = base();
    let body = the_element(&tree, "body");
    let p = by_id(&tree, "p");
    tree.remove_node(p).unwrap();
    assert_eq!(tree.parent(p), None);
    assert_eq!(child_names(&tree, body), ["template"]);
    assert_eq!(child_names(&tree, p), ["\"text\""]);
    // A node without a parent: nothing to do.
    let root = tree.root();
    let before = tree.clone();
    tree.remove_node(root).unwrap();
    tree.remove_node(p).unwrap();
    assert_eq!(tree, before);
}

#[test]
fn write_replace_child() {
    let mut tree = base();
    let body = the_element(&tree, "body");
    let p = by_id(&tree, "p");
    let div = tree.create_element_ns(Namespace::Html, "div").unwrap();
    tree.replace_child(body, div, p).unwrap();
    assert_eq!(child_names(&tree, body), ["div", "template"]);
    assert_eq!(tree.parent(p), None);
    refuses(tree.clone(), WriteError::NotFound, |t| {
        t.replace_child(body, p, p)
    });
    refuses(tree.clone(), WriteError::HierarchyRequest, |t| {
        t.replace_child(div, body, div)
    });
    // Depth: a `span` with an `i` in place of the text of `p`, three deep.
    let mut deep = three_deep();
    let p = by_id(&deep, "p");
    let text = deep.children(p)[0];
    let span = deep.create_element_ns(Namespace::Html, "span").unwrap();
    refuses(deep, WriteError::Limit(Limit::Depth), |t| {
        t.replace_child(p, span, text)
    });
}

#[test]
fn write_move_before() {
    let mut tree = base();
    let body = the_element(&tree, "body");
    let p = by_id(&tree, "p");
    let t = by_id(&tree, "t");
    tree.move_before(body, t, Some(p)).unwrap();
    assert_eq!(child_names(&tree, body), ["template", "p"]);
    assert_eq!(by_id(&tree, "p"), p, "the node keeps its identity");
    // A node out of the tree has another root.
    let span = tree.create_element_ns(Namespace::Html, "span").unwrap();
    refuses(tree.clone(), WriteError::HierarchyRequest, |tr| {
        tr.move_before(body, span, None)
    });
    refuses(tree.clone(), WriteError::NotFound, |tr| {
        tr.move_before(p, t, Some(body))
    });
    refuses(tree.clone(), WriteError::HierarchyRequest, |tr| {
        tr.move_before(p, body, None)
    });
    // Depth: `b` moved from beside `p` into it, three deep.
    let mut deep = parse_document(
        "<p id=p>text</p><b id=b>x</b>",
        &scripted_with(Limits {
            max_depth: 3,
            ..Limits::default()
        }),
    );
    assert_eq!(deep.stopped_by(), None);
    let p = by_id(&deep, "p");
    let b = by_id(&deep, "b");
    let body = the_element(&deep, "body");
    deep.move_before(body, b, Some(p)).unwrap();
    refuses(deep, WriteError::Limit(Limit::Depth), |t| {
        t.move_before(p, b, None)
    });
}

#[test]
fn write_clone_node() {
    let mut tree = base();
    let p = by_id(&tree, "p");
    let count = tree.node_count();
    let shallow = tree.clone_node(p, false).unwrap();
    assert_eq!(tree.data(shallow), tree.data(p));
    assert_eq!(tree.children(shallow), []);
    assert_eq!(tree.parent(shallow), None);
    let deep = tree.clone_node(p, true).unwrap();
    assert_eq!(child_names(&tree, deep), ["\"text\""]);
    assert_eq!(tree.node_count(), count + 3);
    // A template's contents are cloned with it.
    let t = by_id(&tree, "t");
    let copy = tree.clone_node(t, true).unwrap();
    let contents = match tree.data(copy) {
        NodeData::Element {
            template_contents: Some(c),
            ..
        } => *c,
        _ => panic!("a template"),
    };
    assert_eq!(child_names(&tree, contents), ["b"]);
    let root = tree.root();
    refuses(tree.clone(), WriteError::NotSupported, |t| {
        t.clone_node(root, true)
    });
    // `p` and its text are two nodes and `"p" "id" "p" "text"` with one record.
    let bytes = "p".len() + ATTRIBUTE_BYTES + "id".len() + "p".len() + "text".len();
    let tree = nodes_left(1);
    let p = by_id(&tree, "p");
    refuses(tree, WriteError::Limit(Limit::Nodes), |t| {
        t.clone_node(p, true)
    });
    let tree = bytes_left(bytes - 1);
    let p = by_id(&tree, "p");
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.clone_node(p, true)
    });
    let mut tree = bytes_left(bytes);
    let p = by_id(&tree, "p");
    tree.clone_node(p, true).unwrap();
}

#[test]
fn write_import_node() {
    let mut tree = base();
    let p = by_id(&tree, "p");
    let copy = tree.import_node(p, true).unwrap();
    assert_eq!(child_names(&tree, copy), ["\"text\""]);
    let root = tree.root();
    refuses(tree.clone(), WriteError::NotSupported, |t| {
        t.import_node(root, false)
    });
    let tree = nodes_left(0);
    let p = by_id(&tree, "p");
    refuses(tree, WriteError::Limit(Limit::Nodes), |t| {
        t.import_node(p, false)
    });
    let tree = bytes_left(0);
    let p = by_id(&tree, "p");
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.import_node(p, false)
    });
}

#[test]
fn write_adopt_node() {
    let mut tree = base();
    let p = by_id(&tree, "p");
    tree.adopt_node(p).unwrap();
    assert_eq!(tree.parent(p), None);
    // A template's contents have a host and stay where they are.
    let t = by_id(&tree, "t");
    let contents = match tree.data(t) {
        NodeData::Element {
            template_contents: Some(c),
            ..
        } => *c,
        _ => panic!("a template"),
    };
    let before = tree.clone();
    tree.adopt_node(contents).unwrap();
    assert_eq!(tree, before);
    let root = tree.root();
    refuses(tree, WriteError::NotSupported, |t| t.adopt_node(root));
}

#[test]
fn write_normalize() {
    let build = |tree: &mut Tree| {
        let p = by_id(tree, "p");
        for data in ["", "a", "b"] {
            let text = tree.create_text_node(data).unwrap();
            tree.append_child(p, text).unwrap();
        }
        let em = tree.create_element_ns(Namespace::Html, "em").unwrap();
        tree.append_child(p, em).unwrap();
        let empty = tree.create_text_node("").unwrap();
        tree.append_child(em, empty).unwrap();
        p
    };
    let mut tree = base();
    let p = build(&mut tree);
    tree.normalize(p).unwrap();
    assert_eq!(child_names(&tree, p), ["\"textab\"", "em"]);
    assert_eq!(tree.children(the_element(&tree, "em")), []);
    // The merged texts keep their data, so the merge stores "ab" again.
    let mut tree = bytes_left("ab".len() + "em".len() + 1);
    let p = build(&mut tree);
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.normalize(p)
    });
}

#[test]
fn write_split_text() {
    let mut tree = base();
    let p = by_id(&tree, "p");
    let text = tree.children(p)[0];
    let tail = tree.split_text(text, 2).unwrap();
    assert_eq!(child_names(&tree, p), ["\"te\"", "\"xt\""]);
    assert_eq!(tree.children(p)[1], tail);
    refuses(tree.clone(), WriteError::IndexSize, |t| {
        t.split_text(text, 3)
    });
    refuses(tree.clone(), WriteError::InvalidNodeType, |t| {
        t.split_text(p, 0)
    });
    // Offsets count UTF-16 code units: the emoji is two, and cannot be split.
    let emoji = tree.create_text_node("a\u{1F600}b").unwrap();
    refuses(tree.clone(), WriteError::IndexSize, |t| {
        t.split_text(emoji, 2)
    });
    let after = tree.split_text(emoji, 3).unwrap();
    assert_eq!(text_of(&tree, emoji), "a\u{1F600}");
    assert_eq!(text_of(&tree, after), "b");
    let tree = nodes_left(0);
    let text = tree.children(by_id(&tree, "p"))[0];
    refuses(tree, WriteError::Limit(Limit::Nodes), |t| {
        t.split_text(text, 1)
    });
}

/// The text node of `p` in a base with `bytes` stored bytes left.
fn text_with_bytes_left(bytes: usize) -> (Tree, NodeId) {
    let tree = bytes_left(bytes);
    let text = tree.children(by_id(&tree, "p"))[0];
    (tree, text)
}

#[test]
fn write_set_data() {
    let mut tree = base();
    let p = by_id(&tree, "p");
    let text = tree.children(p)[0];
    let stored = tree.stored_bytes();
    tree.set_data(text, "xy").unwrap();
    assert_eq!(text_of(&tree, text), "xy");
    assert_eq!(tree.stored_bytes(), stored - 2);
    let comment = tree.create_comment("c").unwrap();
    tree.set_data(comment, "d").unwrap();
    assert_eq!(text_of(&tree, comment), "d");
    refuses(tree.clone(), WriteError::InvalidNodeType, |t| {
        t.set_data(p, "x")
    });
    let (tree, text) = text_with_bytes_left(1);
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.set_data(text, "texts!")
    });
    let (mut tree, text) = text_with_bytes_left(1);
    tree.set_data(text, "texts").unwrap();
}

#[test]
fn write_append_data() {
    let mut tree = base();
    let text = tree.children(by_id(&tree, "p"))[0];
    tree.append_data(text, "!").unwrap();
    assert_eq!(text_of(&tree, text), "text!");
    let (tree, text) = text_with_bytes_left(0);
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.append_data(text, "!")
    });
}

#[test]
fn write_insert_data() {
    let mut tree = base();
    let text = tree.children(by_id(&tree, "p"))[0];
    tree.insert_data(text, 2, "\u{1F600}").unwrap();
    assert_eq!(text_of(&tree, text), "te\u{1F600}xt");
    tree.insert_data(text, 4, "-").unwrap();
    assert_eq!(text_of(&tree, text), "te\u{1F600}-xt");
    refuses(tree.clone(), WriteError::IndexSize, |t| {
        t.insert_data(text, 8, "x")
    });
    refuses(tree.clone(), WriteError::IndexSize, |t| {
        t.insert_data(text, 3, "x")
    });
    let (tree, text) = text_with_bytes_left(0);
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.insert_data(text, 0, "x")
    });
}

#[test]
fn write_delete_data() {
    let mut tree = base();
    let text = tree.children(by_id(&tree, "p"))[0];
    let stored = tree.stored_bytes();
    tree.delete_data(text, 1, 2).unwrap();
    assert_eq!(text_of(&tree, text), "tt");
    assert_eq!(tree.stored_bytes(), stored - 2);
    // A count past the end deletes to the end.
    tree.delete_data(text, 1, 100).unwrap();
    assert_eq!(text_of(&tree, text), "t");
    refuses(tree.clone(), WriteError::IndexSize, |t| {
        t.delete_data(text, 2, 0)
    });
}

#[test]
fn write_replace_data() {
    let mut tree = base();
    let text = tree.children(by_id(&tree, "p"))[0];
    tree.replace_data(text, 1, 2, "ES").unwrap();
    assert_eq!(text_of(&tree, text), "tESt");
    refuses(tree.clone(), WriteError::IndexSize, |t| {
        t.replace_data(text, 5, 0, "")
    });
    let (tree, text) = text_with_bytes_left(1);
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.replace_data(text, 0, 1, "abc")
    });
    let (mut tree, text) = text_with_bytes_left(1);
    tree.replace_data(text, 0, 1, "ab").unwrap();
    assert_eq!(text_of(&tree, text), "abext");
}

#[test]
fn write_set_text_content() {
    let mut tree = base();
    let body = the_element(&tree, "body");
    let p = by_id(&tree, "p");
    let old = tree.children(p)[0];
    tree.set_text_content(body, "all").unwrap();
    assert_eq!(child_names(&tree, body), ["\"all\""]);
    assert_eq!(tree.parent(p), None);
    tree.set_text_content(p, "").unwrap();
    assert_eq!(tree.children(p), []);
    assert_eq!(tree.parent(old), None);
    // On character data it sets the data.
    tree.set_text_content(old, "new").unwrap();
    assert_eq!(text_of(&tree, old), "new");
    let tree = nodes_left(0);
    let p = by_id(&tree, "p");
    refuses(tree, WriteError::Limit(Limit::Nodes), |t| {
        t.set_text_content(p, "x")
    });
    let tree = bytes_left(2);
    let p = by_id(&tree, "p");
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.set_text_content(p, "xyz")
    });
}

fn attributes_of(tree: &Tree, node: NodeId) -> Vec<Attribute> {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes.clone(),
        _ => panic!("an element"),
    }
}

/// The base with `p` holding as many attributes as the limit allows (its one `id`).
fn one_attribute_allowed() -> (Tree, NodeId) {
    let tree = base_with(|_| Limits {
        max_attributes: 1,
        ..Limits::default()
    });
    let p = by_id(&tree, "p");
    (tree, p)
}

#[test]
fn write_set_attribute() {
    let mut tree = base();
    let p = by_id(&tree, "p");
    let stored = tree.stored_bytes();
    tree.set_attribute(p, "Class", "a").unwrap();
    assert_eq!(
        attribute(&tree, p, "class"),
        Some("a"),
        "lower case on an HTML element"
    );
    assert_eq!(
        tree.stored_bytes(),
        stored + ATTRIBUTE_BYTES + "class".len() + 1
    );
    tree.set_attribute(p, "id", "q").unwrap();
    assert_eq!(attribute(&tree, p, "id"), Some("q"));
    assert_eq!(attributes_of(&tree, p).len(), 2);
    let text = tree.children(p)[0];
    refuses(tree.clone(), WriteError::InvalidNodeType, |t| {
        t.set_attribute(text, "a", "b")
    });
    let (tree, p) = one_attribute_allowed();
    refuses(tree, WriteError::Limit(Limit::Attributes), |t| {
        t.set_attribute(p, "class", "a")
    });
    let (mut tree, p) = one_attribute_allowed();
    tree.set_attribute(p, "id", "other").unwrap();
    let tree = bytes_left(ATTRIBUTE_BYTES + "class".len());
    let p = by_id(&tree, "p");
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.set_attribute(p, "class", "a")
    });
    let tree = bytes_left(0);
    let p = by_id(&tree, "p");
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.set_attribute(p, "id", "pp")
    });
}

#[test]
fn write_set_attribute_ns() {
    let mut tree = base();
    let p = by_id(&tree, "p");
    tree.set_attribute_ns(p, Some(Namespace::XLink), "xlink:href", "#a")
        .unwrap();
    tree.set_attribute_ns(p, Some(Namespace::XLink), "other:href", "#b")
        .unwrap();
    let attributes = attributes_of(&tree, p);
    assert_eq!(attributes.len(), 2);
    assert_eq!(
        attributes[1],
        Attribute {
            namespace: Some(Namespace::XLink),
            prefix: Some("xlink".to_string()),
            local_name: "href".to_string(),
            value: "#b".to_string(),
        }
    );
    tree.set_attribute_ns(p, None, "Data", "x").unwrap();
    assert_eq!(attribute(&tree, p, "Data"), Some("x"), "no lower case");
    refuses(tree.clone(), WriteError::Namespace, |t| {
        t.set_attribute_ns(p, None, "a:b", "x")
    });
    refuses(tree.clone(), WriteError::Namespace, |t| {
        t.set_attribute_ns(p, Some(Namespace::Html), "xml:b", "x")
    });
    refuses(tree.clone(), WriteError::Namespace, |t| {
        t.set_attribute_ns(p, Some(Namespace::Html), "xmlns", "x")
    });
    refuses(tree.clone(), WriteError::Namespace, |t| {
        t.set_attribute_ns(p, Some(Namespace::Xmlns), "a", "x")
    });
    tree.set_attribute_ns(p, Some(Namespace::Xmlns), "xmlns:a", "x")
        .unwrap();
    let (tree, p) = one_attribute_allowed();
    refuses(tree, WriteError::Limit(Limit::Attributes), |t| {
        t.set_attribute_ns(p, Some(Namespace::XLink), "xlink:href", "#")
    });
    let tree = bytes_left(ATTRIBUTE_BYTES + "xlink".len() + "href".len());
    let p = by_id(&tree, "p");
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.set_attribute_ns(p, Some(Namespace::XLink), "xlink:href", "#")
    });
}

#[test]
fn write_remove_attribute() {
    let mut tree = base();
    let p = by_id(&tree, "p");
    let stored = tree.stored_bytes();
    tree.remove_attribute(p, "ID").unwrap();
    assert_eq!(attributes_of(&tree, p), []);
    assert_eq!(
        tree.stored_bytes(),
        stored - ATTRIBUTE_BYTES - "id".len() - "p".len()
    );
    let before = tree.clone();
    tree.remove_attribute(p, "id").unwrap();
    assert_eq!(tree, before);
}

#[test]
fn write_remove_attribute_ns() {
    let mut tree = base();
    let p = by_id(&tree, "p");
    tree.set_attribute_ns(p, Some(Namespace::XLink), "xlink:href", "#")
        .unwrap();
    tree.remove_attribute_ns(p, Some(Namespace::XLink), "href")
        .unwrap();
    tree.remove_attribute_ns(p, Some(Namespace::XLink), "id")
        .unwrap();
    assert_eq!(attributes_of(&tree, p).len(), 1);
    tree.remove_attribute_ns(p, None, "id").unwrap();
    assert_eq!(attributes_of(&tree, p), []);
}

#[test]
fn write_toggle_attribute() {
    let mut tree = base();
    let p = by_id(&tree, "p");
    assert_eq!(tree.toggle_attribute(p, "Hidden", None), Ok(true));
    assert_eq!(attribute(&tree, p, "hidden"), Some(""));
    assert_eq!(tree.toggle_attribute(p, "hidden", Some(true)), Ok(true));
    assert_eq!(tree.toggle_attribute(p, "hidden", None), Ok(false));
    assert_eq!(attribute(&tree, p, "hidden"), None);
    assert_eq!(tree.toggle_attribute(p, "hidden", Some(false)), Ok(false));
    assert_eq!(attributes_of(&tree, p).len(), 1);
    let (tree, p) = one_attribute_allowed();
    refuses(tree, WriteError::Limit(Limit::Attributes), |t| {
        t.toggle_attribute(p, "hidden", None)
    });
    let tree = bytes_left(ATTRIBUTE_BYTES + "hidden".len() - 1);
    let p = by_id(&tree, "p");
    refuses(tree, WriteError::Limit(Limit::StoredBytes), |t| {
        t.toggle_attribute(p, "hidden", None)
    });
}

/// The limits a tree was parsed with are the ones its writes are held to.
#[test]
fn a_tree_keeps_the_limits_of_its_parse() {
    let limits = Limits {
        max_nodes: 99,
        ..Limits::default()
    };
    assert_eq!(parse_document("", &scripted_with(limits)).limits(), limits);
}

// The depth limit bounds the tree (fix round 2), in every parse.

/// Parse options with scripting enabled and a depth limit of `max_depth`.
fn deep(max_depth: usize) -> ParseOptions {
    scripted_with(Limits {
        max_depth,
        ..Limits::default()
    })
}

/// The template contents of `node`, if it is a template.
fn contents_of(tree: &Tree, node: NodeId) -> Option<NodeId> {
    match tree.data(node) {
        NodeData::Element {
            template_contents, ..
        } => *template_contents,
        _ => None,
    }
}

/// The most elements on a path down from the root, a template counted for its contents.
fn depth_of(tree: &Tree) -> usize {
    deepest_from(tree, &[tree.root()])
}

/// The most elements on a path down from one of `roots`, a template counted for its contents;
/// a node met twice ends the walk there (the cycle check is [`cycle_in`]'s).
fn deepest_from(tree: &Tree, roots: &[NodeId]) -> usize {
    let mut most = 0;
    let mut seen = HashSet::new();
    let mut pending: Vec<(NodeId, usize)> = roots.iter().map(|&r| (r, 0)).collect();
    while let Some((node, above)) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        let depth = above + usize::from(matches!(tree.data(node), NodeData::Element { .. }));
        most = most.max(depth);
        pending.extend(tree.children(node).iter().map(|&c| (c, depth)));
        pending.extend(contents_of(tree, node).map(|c| (c, depth)));
    }
    most
}

/// Markup that makes a tree deeper than the depth limit on `main`, with no hook: the copy of
/// a `selectedcontent` (two forms, and a third whose option holds a `template`: the copy of
/// its contents is one deeper than the copy of the `template`), the adoption agency's moves
/// and a case of a random search (the second review's report, (a): 2N − 4 deep at a limit of
/// N, and 18 at 16).
fn deep_markup(n: usize) -> Vec<String> {
    let (k, m) = (n - 5, n - 4);
    vec![
        format!(
            "<select><button><div><selectedcontent></selectedcontent></div></button>\
             <option><template>{}x",
            "<i>".repeat(n - 5)
        ),
        format!(
            "<select><button>{}<selectedcontent></selectedcontent>{}</button><option>{}x",
            "<div>".repeat(k),
            "</div>".repeat(k),
            "<i>".repeat(m)
        ),
        format!(
            "<select><button>{}<selectedcontent></selectedcontent>{}</button><option>{}x",
            "<span>".repeat(k),
            "</span>".repeat(k),
            "<span>".repeat(m)
        ),
        format!("<body><b><p>{}</b>", "<div>".repeat(n)),
        format!("{}</b>", "<b><p>".repeat(n)),
        "<a><div><a>".repeat(n),
        "<form><button><u></i></form><u><u><option></a><dd><code></form><i><ul></p><option><td>\
         <optgroup><s></div><em></template></u><form><div><i><td></table><h1></i><option>\
         </form><table><template><form><object><font color=red></html>"
            .to_string(),
    ]
}

/// Markup alone, no hook: no tree is deeper than the limit at 10 and 16, and a parse that
/// would pass it names `Depth`.
#[test]
fn markup_alone_makes_no_tree_deeper_than_the_depth_limit() {
    let mut wrong = Vec::new();
    for n in [10, 16] {
        for input in deep_markup(n) {
            let tree = parse_document(&input, &deep(n));
            let depth = depth_of(&tree);
            if depth > n {
                wrong.push(format!(
                    "limit {n}: {depth} deep, stopped by {:?}: {input}",
                    tree.stopped_by()
                ));
            }
            let unbounded = parse_document(&input, &deep(512));
            if depth_of(&unbounded) > n && tree.stopped_by() != Some(Limit::Depth) {
                wrong.push(format!("limit {n}: not stopped by Depth: {input}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The adoption agency's iteration that would pass the limit makes no change: the tree holds
/// the furthest block where it was, with its children, and the parse names `Depth`. The hook
/// puts the formatting element `b` under the `html` element (depth 2) and the `body` 4 deep;
/// three `span` then go under the `p` (4 to 6 deep). `</b>` would move the `p` under the
/// `body` (5) with the new `b` under it (6) and the `span` under that: 7 to 9 deep.
#[test]
fn an_adoption_agency_iteration_past_the_limit_moves_nothing() {
    let input = "<body><b id=f><p id=fb><script id=s>x</script><span><span><span>y</b>";
    let hook = || {
        Recorder::with(|tree, script, _| {
            let p = tree.parent(script).unwrap();
            let b = tree.parent(p).unwrap();
            let body = tree.parent(b).unwrap();
            let html = tree.parent(body).unwrap();
            tree.append_child(html, b).unwrap();
            let outer = tree.create_element_ns(Namespace::Html, "div").unwrap();
            let inner = tree.create_element_ns(Namespace::Html, "div").unwrap();
            tree.append_child(html, outer).unwrap();
            tree.append_child(outer, inner).unwrap();
            tree.append_child(inner, body).unwrap();
        })
    };
    let tree = parse_document_with_hook(input, &deep(8), &mut hook());
    assert_eq!(tree.stopped_by(), Some(Limit::Depth));
    assert_eq!(depth_of(&tree), 6);
    let p = by_id(&tree, "fb");
    assert_eq!(tree.parent(p), Some(by_id(&tree, "f")));
    assert_eq!(child_names(&tree, p), ["script", "span"]);
    let tree = parse_document_with_hook(input, &deep(9), &mut hook());
    assert_eq!(tree.stopped_by(), None);
    assert_eq!(depth_of(&tree), 9);
    assert_eq!(
        name(&tree, tree.parent(by_id(&tree, "fb")).unwrap()),
        "body"
    );
}

/// The start tag decides the HTML integration point of a MathML `annotation-xml` (13.2.6):
/// an `encoding` of `text/html` or of `application/xhtml+xml`, in any ASCII case, makes one,
/// so `<foo>` under it is an HTML element; another value does not.
#[test]
fn both_html_encodings_make_an_annotation_xml_an_integration_point() {
    for (encoding, expected) in [
        ("text/html", Namespace::Html),
        ("TEXT/Html", Namespace::Html),
        ("application/xhtml+xml", Namespace::Html),
        ("Application/XHTML+XML", Namespace::Html),
        ("image/svg+xml", Namespace::MathMl),
    ] {
        let input = format!(
            "<math><annotation-xml encoding=\"{encoding}\"><foo id=f></foo></annotation-xml></math>"
        );
        let tree = parse_document(&input, &deep(512));
        let namespace = match tree.data(by_id(&tree, "f")) {
            NodeData::Element { namespace, .. } => *namespace,
            other => panic!("{other:?}"),
        };
        assert_eq!(namespace, expected, "{input}");
    }
}

/// The adoption agency stops before a move that would pass the limit, also when no element of
/// the tree is at the limit yet: the hook moves the furthest block `p` up to the `html`
/// element (2 deep); four `span` go under it (3 to 6), the formatting element `b` stays 6
/// deep under three `div`. `</b>` would put `p` under the third `div` (6), the new `b` under
/// it (7) and the `span` 8 to 11 deep. At a limit of 10 the parse stops at `Depth` with the
/// tree 6 deep; at 11 the iteration is made and the tree is 11 deep.
#[test]
fn an_adoption_agency_move_past_the_limit_stops_with_no_element_at_it() {
    let input = "<body><div><div><div id=ca><b><p id=fb><script>x</script>\
                 <span><span><span><span>y</b><p id=after>z";
    let hook = || {
        Recorder::with(|tree, script, _| {
            let p = tree.parent(script).unwrap();
            let html = tree.children(tree.root())[0];
            tree.append_child(html, p).unwrap();
        })
    };
    let tree = parse_document_with_hook(input, &deep(10), &mut hook());
    assert_eq!(tree.stopped_by(), Some(Limit::Depth));
    assert_eq!(depth_of(&tree), 6);
    assert_eq!(
        name(&tree, tree.parent(by_id(&tree, "fb")).unwrap()),
        "html"
    );
    let tree = parse_document_with_hook(input, &deep(11), &mut hook());
    assert_eq!(tree.stopped_by(), None);
    assert_eq!(depth_of(&tree), 11);
    assert_eq!(tree.parent(by_id(&tree, "fb")), Some(by_id(&tree, "ca")));
    by_id(&tree, "after");
}

/// After a move of the adoption agency that shifts nodes up, the depths the parser reads for
/// its entries are the new ones: `</b>` takes the `span` off the stack, so its first outer
/// iteration moves the ten
/// `div` up by one; after its eight iterations the last two `div` hang under the last new `b`,
/// 12 and 13 deep, and the `em` after them is 14 deep, inside a limit of 14. The tenth `div`
/// is 14 deep before `</b>`, so at 13 the parse stops there. With nine `div`, the ninth is the
/// last iteration's moved child itself, 12 deep, and the `em` 13.
#[test]
fn nodes_the_adoption_agency_moves_up_keep_their_depths() {
    for divs in [10, 9] {
        let input = format!("<body><b><span>{}</b><em>x", "<div>".repeat(divs));
        let tree = parse_document(&input, &deep(divs + 4));
        assert_eq!(tree.stopped_by(), None, "{input}");
        assert_eq!(depth_of(&tree), divs + 4, "{input}");
        let tree = parse_document(&input, &deep(divs + 3));
        assert_eq!(tree.stopped_by(), Some(Limit::Depth), "{input}");
    }
}

/// A deep clone a hook appends counts with its nodes: the `body` moved under the last node of
/// a cloned chain is 5 deep, so markup under it stops at the limit.
#[test]
fn a_cloned_chain_keeps_its_depths() {
    let mut hook = Recorder::with(|tree, script, _| {
        let body = tree.parent(script).unwrap();
        let html = tree.parent(body).unwrap();
        let first = tree.create_element_ns(Namespace::Html, "div").unwrap();
        let mut last = first;
        for _ in 0..2 {
            let next = tree.create_element_ns(Namespace::Html, "div").unwrap();
            tree.append_child(last, next).unwrap();
            last = next;
        }
        let copy = tree.clone_node(first, true).unwrap();
        tree.append_child(html, copy).unwrap();
        let leaf = *tree.descendants(copy).last().unwrap();
        tree.append_child(leaf, body).unwrap();
    });
    let input = format!("<body><script>x</script>{}", "<div>".repeat(5));
    let tree = parse_document_with_hook(&input, &deep(8), &mut hook);
    assert_eq!(depth_of(&tree), 8);
    assert_eq!(tree.stopped_by(), Some(Limit::Depth));
}

/// The depth limit bounds the tree, not the stack of open elements (ruling 1 of fix round 3:
/// the parse stops only where a node would pass the limit in the tree): a `b` foster parented
/// out of a `table` is 3 deep in the tree and the fourth entry of the stack, so at a limit of
/// 4 the `i` under it is made, 4 deep and the fifth entry, and the `u` under that is not.
#[test]
fn the_depth_limit_bounds_the_tree_not_the_stack_of_open_elements() {
    let tree = parse_document("<table><b><i><u>", &deep(4));
    assert_eq!(tree.stopped_by(), Some(Limit::Depth));
    assert_eq!(depth_of(&tree), 4);
    let names: Vec<String> = tree
        .descendants(tree.root())
        .iter()
        .map(|&n| name(&tree, n))
        .collect();
    assert!(names.iter().any(|n| n == "i"), "{names:?}");
    assert!(!names.iter().any(|n| n == "u"), "{names:?}");
}

/// At a depth limit of 0 no element is made, not even the `html` element.
#[test]
fn a_depth_limit_of_zero_makes_no_element() {
    let tree = parse_document("<p>x", &deep(0));
    assert_eq!(tree.stopped_by(), Some(Limit::Depth));
    assert_eq!(depth_of(&tree), 0);
}

/// An adoption agency iteration reads again the entries below its formatting element when a
/// hook left one of them under an entry above it. The hook moves the furthest block `p` up to
/// the `html` element, the `span` `c` (with the `b` under it) under the `body`, and the `div`
/// `d` under the `p`: `d`, an entry below `b`, is under `p`, an entry above it. `</b>` puts
/// the `p` under `c` (4 deep), the new `b` under it (5) and `d` under that (6). After `</p>`
/// and `</span>`, `d` is the current node: one `i` goes under it, 7 deep, and the limit of 7
/// stops the second.
#[test]
fn the_entries_below_an_iteration_are_read_again_when_a_hook_put_one_under_it() {
    let input = "<body><div id=d><span id=c><b><p id=fb><script>x</script></b></p></span>\
                 <i><i><i>";
    let mut hook = Recorder::with(|tree, script, _| {
        let p = tree.parent(script).unwrap();
        let b = tree.parent(p).unwrap();
        let c = tree.parent(b).unwrap();
        let d = tree.parent(c).unwrap();
        let body = tree.parent(d).unwrap();
        let html = tree.parent(body).unwrap();
        tree.append_child(html, p).unwrap();
        tree.append_child(body, c).unwrap();
        tree.append_child(p, d).unwrap();
    });
    let tree = parse_document_with_hook(input, &deep(7), &mut hook);
    assert_eq!(tree.stopped_by(), Some(Limit::Depth));
    assert_eq!(depth_of(&tree), 7);
    assert_eq!(name(&tree, tree.parent(by_id(&tree, "d")).unwrap()), "b");
}

/// When the furthest block holds the place step 4.23 would put it at, the iteration takes it
/// out of the tree and its subtree counts from its own top. The hook moves the furthest block
/// `p` up to the `html` element and the common ancestor `d` (with the `b` under it) under the
/// `p`: `</b>` leaves the `p` 1 deep, the new `b` 2 and `d` 3, and the `i` goes under the new
/// `b`, inside a limit of 6. Counted from `d`'s place, the moved `b` would be 7 deep.
#[test]
fn a_furthest_block_that_holds_the_place_counts_from_its_own_top() {
    let input = "<body><div id=d><b><p id=fb><script>x</script></b><i id=after>";
    let mut hook = Recorder::with(|tree, script, _| {
        let p = tree.parent(script).unwrap();
        let b = tree.parent(p).unwrap();
        let d = tree.parent(b).unwrap();
        let html = tree.parent(tree.parent(d).unwrap()).unwrap();
        tree.append_child(html, p).unwrap();
        tree.append_child(p, d).unwrap();
    });
    let tree = parse_document_with_hook(input, &deep(6), &mut hook);
    assert_eq!(tree.stopped_by(), None);
    assert_eq!(depth_of(&tree), 2);
}

/// The walk of an iteration that moves the furthest block's children down is steps, each node
/// one (`limits.rs`, `Steps`): the hook moved `p` up to `html`, so `</b>` moves its 1,002
/// children down and walks them once. Without the move nothing is walked.
#[test]
fn the_walk_of_a_move_down_is_steps() {
    const S: usize = 1_000;
    let input = format!(
        "<b><i><p id=fb><script id=s>x</script>{}</b>",
        "<br>".repeat(S)
    );
    let moved = parse_document_with_hook(
        &input,
        &deep(512),
        &mut Recorder::with(|tree, script, _| {
            let p = tree.parent(script).unwrap();
            let html = tree.children(tree.root())[0];
            tree.append_child(html, p).unwrap();
        }),
    );
    let still = parse_document_with_hook(&input, &deep(512), &mut Recorder::with(|_, _, _| {}));
    assert_eq!(moved.stopped_by(), None);
    assert_eq!(still.stopped_by(), None);
    let more = moved.steps() - still.steps();
    assert!((S + 2..2 * (S + 2)).contains(&more), "{more} more steps");
}

/// A `head` put back on the stack of open elements (after head, 13.2.6.4.6) leaves the stack
/// ordered: a later adoption agency iteration reads only the entries from its formatting
/// element up, as many steps as with no `head` put back.
#[test]
fn a_head_put_back_on_the_stack_leaves_it_ordered() {
    let steps = |input: &str| parse_document(input, &deep(512)).steps();
    let iteration = |prefix: &str| {
        steps(&format!("{prefix}<a><span><div></a>")) - steps(&format!("{prefix}<a><span><div>"))
    };
    assert_eq!(iteration("<head></head><link>"), iteration(""));
}

/// A write that sets the text content of an element and so takes away the elements under it
/// makes the parser read its entries' depths again: the hook empties the `body`, so the three
/// `div` are out of the document, the last one 3 deep in its own tree, and three `i` go under
/// it inside a limit of 6.
#[test]
fn a_text_content_write_that_takes_away_elements_is_read_again() {
    let input = "<body><div><div><div><script>x</script><i><i><i>";
    let mut hook = Recorder::with(|tree, script, _| {
        let mut body = script;
        for _ in 0..4 {
            body = tree.parent(body).unwrap();
        }
        tree.set_text_content(body, "t").unwrap();
    });
    let tree = parse_document_with_hook(input, &deep(6), &mut hook);
    assert_eq!(tree.stopped_by(), None);
}

/// The `frameset` start tag in body (13.2.6.4.7) takes the `body` out: when a hook put the
/// `html` element under the `body`, the parser reads its depth again. The hook removes the
/// `head`, puts the `body` under a new `div` and the `html` element under the `body`; the
/// `frameset` then goes under the `html` element, 2 deep in the `body`'s tree, so 3 deep,
/// inside a limit of 3.
#[test]
fn the_frameset_rule_reads_the_html_elements_depth_again() {
    let input = "</body><script>x</script><frameset>";
    let mut hook = Recorder::with(|tree, script, _| {
        let body = tree.parent(script).unwrap();
        let html = tree.parent(body).unwrap();
        let head = tree.children(html)[0];
        tree.remove_node(head).unwrap();
        let top = tree.create_element_ns(Namespace::Html, "div").unwrap();
        tree.append_child(top, body).unwrap();
        tree.append_child(body, html).unwrap();
    });
    let tree = parse_document_with_hook(input, &deep(3), &mut hook);
    assert_eq!(tree.stopped_by(), None);
}

// The step limit bounds the work on the stack of open elements (fix round 4, ruling 1).

/// A hook that moves the script's parent to the end of the `body` and takes the element that
/// held it out of the tree: every element of the markup stays open, all but the last without
/// a parent.
struct KeepOpen;

impl ScriptHook for KeepOpen {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
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

/// Each entry of the stack of open elements read again after a hook run is a step, also an
/// entry with no parent, which no hop reaches: with [`KeepOpen`], run `k` reads about `k`
/// entries, so 200 rounds more after 200 take at least 200 × 200 steps more.
#[test]
fn each_entry_read_again_after_a_hook_run_is_a_step() {
    let steps = |rounds: usize| {
        let input = format!("<body>{}", "<span><script>x</script>".repeat(rounds));
        let tree = parse_document_with_hook(&input, &deep(512), &mut KeepOpen);
        assert_eq!(tree.stopped_by(), None);
        tree.steps()
    };
    let (two, four) = (steps(200), steps(400));
    assert!(
        four - two >= 200 * 200,
        "{two} steps at 200 rounds, {four} at 400"
    );
}

/// The page of the fourth review's red case with the default limits: 40,000 rounds of
/// `<span><script>x</script>` (960 KB) with [`KeepOpen`] would read about 8 × 10^8 entries;
/// the step limit (2^27) ends the parse at `Steps` after about 16,000 runs, within 10 s in a
/// test build (1.4 s measured).
#[test]
fn a_hook_that_keeps_every_element_open_ends_at_the_step_limit() {
    const ROUNDS: usize = 40_000;
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let input = format!("<body>{}", "<span><script>x</script>".repeat(ROUNDS));
        let start = std::time::Instant::now();
        let tree = parse_document_with_hook(&input, &scripted(), &mut KeepOpen);
        let _ = send.send((tree.stopped_by(), tree.steps(), start.elapsed()));
    });
    let (stopped, steps, elapsed) = receive
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("the parse ends within 10 s");
    eprintln!("stopped by {stopped:?}, {steps} steps, {elapsed:?}");
    assert_eq!(stopped, Some(Limit::Steps));
}

/// Each scope boundary the adoption agency's changes to the stack of open elements visit is a
/// step (the fourth review's probe `oremove:K:A`): K `object` elements kept open by
/// [`KeepOpen`], each a boundary of "has an element in scope", then A times
/// `<b><p>x</b></p>`, an adoption agency run each, which removes an entry and inserts one.
/// The steps of the A runs grow with K times A.
#[test]
fn each_scope_boundary_the_adoption_agency_visits_is_a_step() {
    const A: usize = 50;
    let steps = |k: usize, a: usize| {
        let input = format!(
            "<body>{}{}",
            "<object><script>x</script>".repeat(k),
            "<b><p>x</b></p>".repeat(a)
        );
        let tree = parse_document_with_hook(&input, &deep(512), &mut KeepOpen);
        assert_eq!(tree.stopped_by(), None);
        tree.steps()
    };
    let runs = |k: usize| steps(k, A) - steps(k, 0);
    let (few, many) = (runs(20), runs(200));
    assert!(
        many - few >= (200 - 20) * A,
        "{few} steps for {A} runs over 20 objects, {many} over 200"
    );
}

/// Each entry above the furthest block that the adoption agency's changes to the stack move is
/// a step: `</a>` over `a`, `span`, `div` and k `span` costs 10 steps more for each `span`
/// above the `div`. First outer iteration: the stack down to the `a` (4.4) and the scope (4.5)
/// 1 each; removing the lower `span` (4.18.5) and the `a` (4.26) moves it, 1 each, and
/// inserting the new `a` (4.26) 1; its depth read again, the entry and its parent, 2. Second
/// outer iteration (the new `a`, no furthest block): the stack down to it, the scope and the
/// search for a furthest block, 1 each. From 1 `span` on (the `div`'s one child, which step
/// 4.30 moves, is there from the first).
#[test]
fn each_entry_the_adoption_agency_moves_on_the_stack_is_a_step() {
    let steps = |input: &str| parse_document(input, &deep(512)).steps();
    let run = |k: usize| {
        let prefix = format!("<a><span><div>{}", "<span>".repeat(k));
        steps(&format!("{prefix}</a>")) - steps(&prefix)
    };
    assert_eq!(run(9) - run(1), 8 * 10);
}

/// A scope boundary above an entry removed from inside the stack keeps its place: after head,
/// a `template` start tag puts the `head` back on the stack, pushes the `template` above it and
/// removes the `head` from under it. The `input` in the template contents asks whether a
/// `select` is in scope, which reads the lowest boundary, the `template`.
#[test]
fn a_boundary_above_an_entry_removed_keeps_its_place() {
    let tree = parse_document("<head></head><template><input id=i></template>", &deep(512));
    let input = tree
        .descendants(tree.root())
        .into_iter()
        .chain(
            tree.descendants(tree.root())
                .into_iter()
                .filter_map(|n| contents_of(&tree, n))
                .flat_map(|c| tree.descendants(c)),
        )
        .find(|&n| attribute(&tree, n, "id") == Some("i"));
    assert!(input.is_some());
}

/// The `frameset` rule pops the `body` and every entry above it, a step each: with k `div`
/// open (k ≥ 1, so that a `body` is made), the `frameset` costs k + 1 steps for the pops (the
/// `body` and k `div`) and 1 for taking the `body` out of the `html` element's children (its
/// last child, the first the scan visits) more than the parse without it.
#[test]
fn the_pops_of_the_frameset_rule_are_steps() {
    let steps = |input: &str| parse_document(input, &deep(512)).steps();
    let frameset = |k: usize| {
        let divs = "<div>".repeat(k);
        steps(&format!("{divs}<frameset>")) as isize - steps(&divs) as isize
    };
    for k in [1, 2, 8] {
        assert_eq!(frameset(k), k as isize + 1 + 1, "{k} div");
    }
}

/// The nodes reached from the root, also through template contents.
fn reachable(tree: &Tree) -> usize {
    let mut seen = HashSet::new();
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        if seen.insert(node) {
            pending.extend(tree.children(node).iter().copied());
            pending.extend(contents_of(tree, node));
        }
    }
    seen.len()
}

/// Each loop over the stack of open elements or the list of active formatting elements reads
/// the step limit before each entry it visits, and ends there (fix round 5;
/// `limits.rs`, `max_steps`): a parse whose limit falls inside one such loop of its last token
/// ends at `Steps` 1 step past the limit, and every node the parse made is in the tree. Each
/// case is a page whose last token runs that loop over about 2,000 entries (the depth limit
/// raised so that markup alone keeps them open; the entries read again after a hook run with
/// [`KeepOpen`]); the limit is the steps of the page without the token and the given share of
/// what the token costs, which falls inside the loop (a negative share: that many times 2,000
/// steps before the token's last). The place of step 4.19 of the adoption agency is the scan
/// of the stack for the foster parenting `table`, made a second time after the inner loop.
#[test]
fn each_loop_over_the_stack_or_the_list_ends_at_the_step_limit() {
    const L: usize = 2_000;
    let mut wrong = Vec::new();
    for (what, page, token, share) in loop_cases(L) {
        let whole = format!("{page}{token}");
        let before = parse_deep(&page, usize::MAX).steps();
        let after = parse_deep(&whole, usize::MAX).steps();
        // A negative share counts back from the end of the token's steps, in entries: the
        // end-of-file token of the page without `</template>` runs the same loop.
        let max = if share < 0.0 {
            after - (-share * L as f64) as usize
        } else {
            assert!(
                after >= before + L,
                "{what}: {before} steps, {after} with the token"
            );
            before + ((after - before) as f64 * share) as usize
        };
        let tree = parse_deep(&whole, max);
        let hook_removed = what == "the entries read again";
        if tree.stopped_by() != Some(Limit::Steps)
            || tree.steps() != max + 1
            || (!hook_removed && reachable(&tree) != tree.node_count())
        {
            wrong.push(format!(
                "{what}: limit {max}, {} steps, stopped by {:?}, {} of {} nodes in the tree",
                tree.steps(),
                tree.stopped_by(),
                reachable(&tree),
                tree.node_count()
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A parse of `input` with scripting on, [`KeepOpen`] as the hook, a depth limit of 100,000
/// and the step limit `max_steps`.
fn parse_deep(input: &str, max_steps: usize) -> Tree {
    let options = scripted_with(Limits {
        max_depth: 100_000,
        max_steps,
        ..Limits::default()
    });
    parse_document_with_hook(input, &options, &mut KeepOpen)
}

/// The pages of [`each_loop_over_the_stack_or_the_list_ends_at_the_step_limit`] with `l`
/// entries for each loop: what the case reaches, the page, its last token, and the share.
fn loop_cases(l: usize) -> Vec<(&'static str, String, &'static str, f64)> {
    let span = "<span>".repeat(l);
    let rb = "<rb>".repeat(l);
    let g = "<g>".repeat(l);
    let bs: String = (0..l).map(|i| format!("<b id={i}>")).collect();
    vec![
        ("any other end tag", span.clone(), "</q>", 0.75),
        ("has an element in scope", span.clone(), "<p>", 0.5),
        ("a select in scope after a scan", span.clone(), "<hr>", 0.5),
        (
            "generate implied end tags",
            format!("<div>{rb}"),
            "</div>",
            0.75,
        ),
        ("pop until", format!("<div>{span}"), "</div>", 0.75),
        (
            "reset the insertion mode",
            format!("{span}<table>"),
            "</table>",
            0.75,
        ),
        (
            "the place in the stack",
            format!("<form>{span}"),
            "</form>",
            0.6,
        ),
        ("Noah's Ark", bs.clone(), "<b>", 0.75),
        (
            "the reconstruction's rewind",
            format!("<div>{bs}</div>"),
            "x",
            0.25,
        ),
        (
            "the reconstruction's advance",
            format!("<div>{bs}</div>"),
            "x",
            0.75,
        ),
        (
            "clear the list to the last marker",
            format!("<object>{bs}"),
            "</object>",
            0.9,
        ),
        ("an a in the list", bs.clone(), "<a>", 0.25),
        ("the formatting element", bs.clone(), "</i>", 0.25),
        (
            "the formatting element's entry of step 4.24",
            format!("<i><div>{bs}</div><p>"),
            "</i>",
            0.5,
        ),
        (
            "an a left in the list",
            format!("<a><div>{bs}</div>"),
            "<a>",
            0.5,
        ),
        ("the furthest block", format!("<b>{span}"), "</b>", 0.9),
        ("the inner loop", format!("<b>{span}<div>"), "</b>", 0.9),
        (
            "the place of step 4.19",
            format!("<table><b><div>{span}"),
            "</b>",
            0.32,
        ),
        (
            "the place of foster parenting",
            format!("<table><b>{span}<div>"),
            "</b>",
            0.45,
        ),
        (
            "the entries read again",
            format!("<body>{span}<span><script>x"),
            "</script>",
            0.75,
        ),
        (
            "clear the stack back to a table context",
            format!("<table>{span}"),
            "<tr>",
            0.75,
        ),
        (
            "has an element in table scope",
            format!("<table>{span}"),
            "</table>",
            0.25,
        ),
        ("a template on the stack", span.clone(), "<html>", 0.75),
        (
            "all implied end tags",
            format!("<template>{rb}"),
            "</template>",
            -0.5,
        ),
        ("li", span.clone(), "<li>", 0.25),
        ("dd", span.clone(), "<dd>", 0.25),
        (
            "the breakout of foreign content",
            format!("<svg>{g}"),
            "<div>",
            0.75,
        ),
        (
            "an end tag in foreign content",
            format!("<svg>{g}"),
            "</svg>",
            0.75,
        ),
    ]
}

/// What keeps a parsed tree from being a tree, as the review's checker of fix round 3
/// (`adversary_script_hook_3`, `faults`) reads it, among the nodes reached from the root: a
/// child whose parent is another node, a child listed twice, a node listed by a parent it does
/// not name, a document that is not the root, a template's contents with a parent, a root
/// with a parent, and a cycle of parents and template hosts.
fn tree_faults(tree: &Tree) -> Vec<String> {
    let bound = tree.node_count();
    let root = tree.root();
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    let mut hosts = std::collections::HashMap::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        if seen.len() > bound {
            found.push("more nodes reached than the arena holds".to_string());
            break;
        }
        let mut listed = HashSet::new();
        for &child in tree.children(node) {
            if !listed.insert(child) {
                found.push(format!("{child:?} listed twice by {node:?}"));
            }
            if tree.parent(child) != Some(node) {
                found.push(format!("{child:?} listed by {node:?} names another parent"));
            }
            pending.push(child);
        }
        if let Some(contents) = contents_of(tree, node) {
            hosts.insert(contents, node);
            if tree.parent(contents).is_some() {
                found.push(format!("the contents {contents:?} have a parent"));
            }
            pending.push(contents);
        }
        if let Some(parent) = tree.parent(node) {
            if !tree.children(parent).contains(&node) {
                found.push(format!("{node:?} names {parent:?}, which does not list it"));
            }
            pending.push(parent);
        }
        if node != root && *tree.data(node) == NodeData::Document {
            found.push(format!("{node:?} is a document and not the root"));
        }
    }
    if tree.parent(root).is_some() {
        found.push("the root has a parent".to_string());
    }
    for &node in &seen {
        let mut at = node;
        let mut steps = 0;
        while let Some(up) = tree.parent(at).or_else(|| hosts.get(&at).copied()) {
            at = up;
            steps += 1;
            if steps > bound {
                found.push(format!("{node:?} is on a cycle of parents and hosts"));
                break;
            }
        }
    }
    found
}

/// A stop inside a token leaves a tree that holds the review's invariants ([`tree_faults`]),
/// with every node the parse made in it (but the elements [`KeepOpen`] takes out), and a parse
/// that does not pass the limit ends whole: for the pages of
/// [`each_loop_over_the_stack_or_the_list_ends_at_the_step_limit`] with 40 entries each, every
/// step limit from the steps of the page without its last token to those with it, so every
/// stop the last token can reach: in each loop, between two reads, and in the adoption
/// agency's putting back of its clones.
#[test]
fn every_stop_inside_a_token_leaves_a_tree() {
    let mut wrong = Vec::new();
    for (what, page, token, _) in loop_cases(40) {
        let whole = format!("{page}{token}");
        let before = parse_deep(&page, usize::MAX).steps();
        let after = parse_deep(&whole, usize::MAX).steps();
        for max in before.min(after)..=after {
            let tree = parse_deep(&whole, max);
            let mut faults = tree_faults(&tree);
            if what != "the entries read again" && reachable(&tree) != tree.node_count() {
                faults.push(format!(
                    "{} of {} nodes in the tree",
                    reachable(&tree),
                    tree.node_count()
                ));
            }
            let ended = tree.stopped_by() == Some(Limit::Steps)
                || (tree.stopped_by().is_none() && tree.steps() <= max);
            if !ended {
                faults.push(format!(
                    "{} steps, stopped by {:?}",
                    tree.steps(),
                    tree.stopped_by()
                ));
            }
            if !faults.is_empty() {
                wrong.push(format!("{what}, limit {max}: {faults:?}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} stops: {:#?}",
        wrong.len(),
        &wrong[..wrong.len().min(10)]
    );
}

/// A parse that has passed the step limit stops at it and names no other limit. For each limit
/// other than the steps that a scan the step limit ends could reach, a page whose last token
/// acts on that scan's answer, with the limit set so that the answer "not found" passes it and
/// the true answer does not: the whole parse ends with no limit, and with every step limit
/// from 0 to its steps the parse ends whole or at `Steps`. The attribute limit: the `html` and
/// `body` start tags merge only when no `template` is open, a scan from the stack's first
/// entry. The depth limit: `<p>` over a `p` closes it when it is in button scope, else goes
/// under the top `span`, one deeper than the limit allows. The node and stored-byte limits:
/// `</p>` with a `p` in button scope makes nothing, else it makes a `p`, one node and one byte
/// over the limits. And a step counted between the read of the step limit and the check of the
/// depth limit (below).
#[test]
fn a_parse_past_the_step_limit_names_no_other_limit() {
    let span = "<span>".repeat(40);
    let div = "<div>".repeat(40);
    let at = |limits: Limits, page: &str| parse_document(page, &scripted_with(limits));
    let depth_page = format!("<p>{span}<p>");
    let max_depth = (1..)
        .find(|&max_depth| {
            let limits = Limits {
                max_depth,
                ..Limits::default()
            };
            at(limits, &depth_page).stopped_by().is_none()
        })
        .unwrap();
    let end_page = format!("<p>{span}</p>");
    let whole = at(Limits::default(), &end_page);
    let cases = [
        (
            Limit::Attributes,
            format!("<html a=1 b=1>{div}<template><html c=1>"),
            Limits {
                max_attributes: 2,
                ..Limits::default()
            },
        ),
        (
            Limit::Attributes,
            format!("<body a=1 b=1>{div}<template><body c=1>"),
            Limits {
                max_attributes: 2,
                ..Limits::default()
            },
        ),
        (
            Limit::Depth,
            depth_page.clone(),
            Limits {
                max_depth,
                ..Limits::default()
            },
        ),
        (
            Limit::Nodes,
            end_page.clone(),
            Limits {
                max_nodes: whole.node_count(),
                ..Limits::default()
            },
        ),
        (
            Limit::StoredBytes,
            end_page,
            Limits {
                max_stored_bytes: whole.stored_bytes(),
                ..Limits::default()
            },
        ),
    ];
    let mut wrong = Vec::new();
    for (kind, page, limits) in cases {
        let all = at(limits, &page);
        assert_eq!(all.stopped_by(), None, "{kind:?}: {page}");
        let mut stops = 0;
        for max_steps in 0..=all.steps() {
            let tree = at(
                Limits {
                    max_steps,
                    ..limits
                },
                &page,
            );
            match tree.stopped_by() {
                None => {}
                Some(Limit::Steps) => stops += 1,
                Some(other) => wrong.push(format!("{kind:?}, step limit {max_steps}: {other:?}")),
            }
        }
        assert!(stops > 0, "{kind:?}: no stop at Steps");
    }
    // A counted walk between the read of the step limit and the check of the depth limit: the
    // depth of a template's contents is read by a step to the template. With a depth limit the
    // `span` passes, the whole parse stops at `Depth`; a parse whose steps passed its step limit
    // names `Steps`, also the one whose limit that step passes.
    let limits = Limits {
        max_depth: 3,
        ..Limits::default()
    };
    let page = "<template><span>";
    let all = at(limits, page);
    assert_eq!(all.stopped_by(), Some(Limit::Depth));
    for max_steps in 0..=all.steps() {
        let tree = at(
            Limits {
                max_steps,
                ..limits
            },
            page,
        );
        if tree.steps() > max_steps && tree.stopped_by() != Some(Limit::Steps) {
            wrong.push(format!(
                "{page}, step limit {max_steps}: {} steps, {:?}",
                tree.steps(),
                tree.stopped_by()
            ));
        }
    }
    // And the adoption agency's check of a move down, after the walk of the nodes under the
    // furthest block, a step each: [`KeepOpen`] takes the `p` out of the `i` to the body, so
    // the clone of the `i` puts the `p` and the 20 `span` under it one deeper than the depth
    // limit allows, and the whole parse stops at `Depth` there.
    let page = format!("<b><i><p><script>x</script>{}</b>", "<span>".repeat(20));
    let keep_open =
        |limits: Limits| parse_document_with_hook(&page, &scripted_with(limits), &mut KeepOpen);
    let limits = Limits {
        max_depth: 23,
        ..Limits::default()
    };
    let all = keep_open(limits);
    assert_eq!(all.stopped_by(), Some(Limit::Depth));
    for max_steps in 0..=all.steps() {
        let tree = keep_open(Limits {
            max_steps,
            ..limits
        });
        if tree.steps() > max_steps && tree.stopped_by() != Some(Limit::Steps) {
            wrong.push(format!(
                "the move down, step limit {max_steps}: {} steps, {:?}",
                tree.steps(),
                tree.stopped_by()
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Past the step limit the parser writes nothing to the tree: the node a page's last token
/// makes is not in a tree whose parse stopped at `Steps`, for every step limit. A scan of the
/// stack or the list the limit ends inside the token answers as if it found nothing, and the
/// write that follows reads the limit: the scope scan of `<p>` over open `span`s (read at the
/// element's place and at its admission, `room_at` and `admit`), the reconstruction's rewind
/// before `x` over closed `b`s (the character's read and `admit`). An element, a character, a
/// comment and a processing instruction after an end tag that scans the whole stack: the read
/// after each token stops the parse before them.
#[test]
fn nothing_is_written_past_the_step_limit() {
    let span = "<span>".repeat(40);
    let after_q = format!("{span}</q>");
    let closed: String = (0..40).map(|i| format!("<b id={i}>")).collect();
    let closed = format!("<div>{closed}</div>");
    // The node the token makes, as the tree prints it.
    let made = |tree: &Tree, node: NodeId| match tree.data(node) {
        NodeData::Element { local_name, .. } => format!("<{local_name}>"),
        NodeData::Text(text) => text.clone(),
        NodeData::Comment(data) => format!("<!--{data}-->"),
        NodeData::ProcessingInstruction { target, .. } => format!("<?{target}>"),
        _ => String::new(),
    };
    let cases = [
        (&span, "<p>", "<p>"),
        (&closed, "x", "x"),
        (&after_q, "<i>", "<i>"),
        (&after_q, "x", "x"),
        (&after_q, "<!--c-->", "<!--c-->"),
        (&after_q, "<?t d?>", "<?t>"),
    ];
    let mut wrong = Vec::new();
    for (page, token, node) in cases {
        let whole = format!("{page}{token}");
        let all = parse_deep(&whole, usize::MAX);
        assert_eq!(all.stopped_by(), None, "{token}");
        let (mut stops, mut whole_parses) = (0, 0);
        for max in 0..=all.steps() {
            let tree = parse_deep(&whole, max);
            let found = tree
                .descendants(tree.root())
                .into_iter()
                .filter(|&n| made(&tree, n) == node)
                .count();
            if tree.stopped_by() == Some(Limit::Steps) {
                stops += 1;
                if found != 0 {
                    wrong.push(format!("{token}: limit {max}, {} steps", tree.steps()));
                }
            } else {
                whole_parses += usize::from(found == 1);
            }
        }
        assert!(
            stops > 0 && whole_parses > 0,
            "{token}: {stops} stops, {whole_parses} whole"
        );
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// An adoption agency iteration whose inner loop the step limit ends after it made clones puts
/// them back into the tree (step 4.23, at the place of step 4.14), with the furthest block
/// under the last: whatever the limit, every node the parse made stays in the tree (the crate
/// documentation, "Limits"), and the parse ends whole or at `Steps`. Four formatting elements
/// between the `b` and the furthest block give three clones; every limit from the steps of
/// the page without the `</b>` to those with it is tried.
#[test]
fn a_limit_inside_the_adoption_agency_leaves_every_node_in_the_tree() {
    let page = "<body><b><i><u><s><em><div>x";
    let parse = |input: &str, max_steps: usize| {
        let options = ParseOptions {
            scripting_disabled: true,
            limits: Limits {
                max_steps,
                ..Limits::default()
            },
        };
        parse_document(input, &options)
    };
    let before = parse(page, usize::MAX).steps();
    let whole = format!("{page}</b>");
    let after = parse(&whole, usize::MAX).steps();
    let mut wrong = Vec::new();
    for max in before..=after {
        let tree = parse(&whole, max);
        let stop_ok = tree.stopped_by() == Some(Limit::Steps)
            || (tree.stopped_by().is_none() && tree.steps() <= max);
        if !stop_ok || reachable(&tree) != tree.node_count() {
            wrong.push(format!(
                "limit {max}: {} steps, stopped by {:?}, {} of {} nodes in the tree",
                tree.steps(),
                tree.stopped_by(),
                reachable(&tree),
                tree.node_count()
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A tiny xorshift generator: the property test is seeded and gives the same inputs each run.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

/// The alphabet of the random markup: formatting elements, tables, `select` with
/// `selectedcontent`, `template`, `svg`, `math` and scripts.
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
    "<table>",
    "</table>",
    "<tr>",
    "<td>",
    "<caption>",
    "<template>",
    "</template>",
    "<select>",
    "</select>",
    "<option>",
    "<button>",
    "</button>",
    "<selectedcontent>",
    "<svg>",
    "</svg>",
    "<math>",
    "<mi>",
    "<annotation-xml encoding=text/html>",
    "<foreignObject>",
    "<font color=red>",
    "<form>",
    "</form>",
    "<em>",
    "<u>",
    "x",
    "</body>",
    "<script>s</script>",
    "<script>s</script>",
    "<script type=module>m</script>",
    "<svg><script>v</script>",
    "<svg><script/>",
    "<math><script>q</script>",
    "<template><script>t</script></template>",
    "<table><script>u</script>",
];

/// A hook that writes at random on the nodes of the tree, the scripts it was called for and
/// the nodes it made, and records its calls.
struct Writer {
    rng: Rng,
    calls: Vec<NodeId>,
    known: Vec<NodeId>,
    made: HashSet<NodeId>,
}

impl Writer {
    fn write(&mut self, tree: &mut Tree) {
        let pick = |w: &mut Writer| w.known[w.rng.below(w.known.len())];
        let (a, b, c) = (pick(self), pick(self), pick(self));
        // Each write may be refused; a refusal is part of the case.
        let _ = match self.rng.below(8) {
            0 => tree.append_child(a, b),
            1 => tree.insert_before(a, b, Some(c)),
            2 => tree.remove_node(a),
            3 => tree.move_before(a, b, None),
            4 => tree.replace_child(a, b, c),
            5 => tree.set_attribute(a, "encoding", "text/html"),
            6 => match tree.clone_node(a, true) {
                Ok(copy) => {
                    // The copy and every node under it, template contents included.
                    let mut pending = vec![copy];
                    while let Some(node) = pending.pop() {
                        self.made.insert(node);
                        pending.extend(tree.children(node).iter().copied());
                        pending.extend(contents_of(tree, node));
                    }
                    self.known.push(copy);
                    tree.append_child(b, copy)
                }
                Err(error) => Err(error),
            },
            _ => match tree.create_element_ns(Namespace::Html, "div") {
                Ok(div) => {
                    self.made.insert(div);
                    self.known.push(div);
                    tree.append_child(a, div)
                }
                Err(error) => Err(error),
            },
        };
    }
}

impl ScriptHook for Writer {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
        self.calls.push(script);
        self.known.push(script);
        self.known.extend(tree.descendants(tree.root()));
        for _ in 0..1 + self.rng.below(3) {
            self.write(tree);
        }
    }

    fn skip(&mut self, _: &Tree, script: NodeId, _: NotRun) {
        self.calls.push(script);
        self.known.push(script);
    }
}

/// Whether a walk up from a node of `nodes`, through parents and template hosts, takes more
/// steps than the arena has nodes.
fn cycle_in(tree: &Tree, nodes: &HashSet<NodeId>) -> bool {
    let hosts: std::collections::HashMap<NodeId, NodeId> = nodes
        .iter()
        .filter_map(|&n| contents_of(tree, n).map(|c| (c, n)))
        .collect();
    nodes.iter().any(|&node| {
        let mut at = node;
        for _ in 0..=tree.node_count() {
            match tree.parent(at).or_else(|| hosts.get(&at).copied()) {
                Some(up) => at = up,
                None => return false,
            }
        }
        true
    })
}

/// The tops of the trees of `nodes`, through parents and template hosts (no cycle: the caller
/// checked with [`cycle_in`]).
fn tops(tree: &Tree, nodes: &HashSet<NodeId>) -> Vec<NodeId> {
    let hosts: std::collections::HashMap<NodeId, NodeId> = nodes
        .iter()
        .filter_map(|&n| contents_of(tree, n).map(|c| (c, n)))
        .collect();
    let mut found: Vec<NodeId> = nodes
        .iter()
        .map(|&node| {
            let mut at = node;
            while let Some(up) = tree.parent(at).or_else(|| hosts.get(&at).copied()) {
                at = up;
            }
            at
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

/// Whether `node` is an HTML or SVG `script` element.
fn is_script(tree: &Tree, node: NodeId) -> bool {
    matches!(
        tree.data(node),
        NodeData::Element { namespace: Namespace::Html | Namespace::Svg, local_name, .. }
            if local_name == "script"
    )
}

/// One random case: the faults of its tree and its calls.
fn random_case(seed: u64, n: usize, writes: bool) -> Vec<String> {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let input: String = (0..4 + rng.below(36))
        .map(|_| PIECES[rng.below(PIECES.len())])
        .collect();
    let mut faults = Vec::new();
    let mut fault = |what: String| faults.push(format!("seed {seed}, limit {n}: {what}: {input}"));
    if !writes {
        let tree = parse_document(&input, &deep(n));
        let depth = depth_of(&tree);
        if depth > n {
            fault(format!("{depth} deep"));
        }
        if tree.stopped_by() == Some(Limit::Depth) && depth != n {
            fault(format!("stopped by Depth, {depth} deep"));
        }
        return faults;
    }
    let mut hook = Writer {
        rng,
        calls: Vec::new(),
        known: Vec::new(),
        made: HashSet::new(),
    };
    let tree = parse_document_with_hook(&input, &deep(n), &mut hook);
    // Every node reached from the root and from the nodes the hook knew, through children,
    // template contents and parents.
    let mut reached: HashSet<NodeId> = HashSet::new();
    let mut pending: Vec<NodeId> = hook.known.clone();
    pending.push(tree.root());
    while let Some(node) = pending.pop() {
        if reached.insert(node) {
            pending.extend(tree.children(node).iter().copied());
            pending.extend(contents_of(&tree, node));
            pending.extend(tree.parent(node));
        }
    }
    if cycle_in(&tree, &reached) {
        fault("a cycle".to_string());
        return faults;
    }
    let depth = depth_of(&tree);
    if depth > n {
        fault(format!("{depth} deep"));
    }
    // The trees the hook holds, each counted from its own top.
    let held = deepest_from(&tree, &tops(&tree, &reached));
    if held > n {
        fault(format!("a tree the hook holds is {held} deep"));
    }
    if tree.stopped_by() == Some(Limit::Depth) && held != n {
        fault(format!("stopped by Depth, {held} deep"));
    }
    let mut called = HashSet::new();
    for &script in &hook.calls {
        if !called.insert(script) {
            fault(format!("{script:?} called twice"));
        }
    }
    for &node in &reached {
        if is_script(&tree, node) && !hook.made.contains(&node) && !called.contains(&node) {
            fault(format!("{node:?} not called"));
        }
    }
    faults
}

/// Random markup from the alphabet of [`PIECES`] at depth limits 8, 10 and 16, each input
/// parsed without a hook and with a hook that writes at random: after each parse no node of
/// the tree, nor of a tree the hook holds (counted from its own top), is deeper than the
/// limit; a parse stopped by `Depth` holds an element at the limit (ruling 1 of fix round 3);
/// the nodes reached from the root and from the nodes the hook knew have no cycle of parents
/// and hosts; and each script element the parser inserted had one call. Seeded: 3 limits × 2
/// ways × 1,000 seeds, 6,000 parses. The stop check holds for these seeds; the adoption
/// agency's stop on a tree a hook built may hold none at the limit
/// (`an_adoption_agency_move_past_the_limit_stops_with_no_element_at_it`).
#[test]
fn random_markup_and_writes_keep_the_depth_limit_a_tree_and_one_call() {
    let mut faults = Vec::new();
    for n in [8, 10, 16] {
        for writes in [false, true] {
            for seed in 0..1_000 {
                faults.extend(random_case(seed, n, writes));
            }
        }
    }
    assert!(
        faults.is_empty(),
        "{} faults: {:#?}",
        faults.len(),
        &faults[..faults.len().min(5)]
    );
}

// The reads of an element's attributes and the owners of the rules the writes share
// (story `scripts-dom-attributes`, decision 3).

/// A page with an HTML element `p` and an SVG element whose names the parser keeps in mixed
/// case, and an SVG `use` with an `xlink:href`.
const READS: &str = "<!DOCTYPE html><body><p id=p class=c data-x=1>text</p>\
<svg id=s viewBox=\"0 0 1 1\"><use id=u xlink:href=\"#a\"></use></svg>";

fn reads() -> Tree {
    parse_document(READS, &scripted())
}

/// The source of the tree with every line comment, and the rest of a line after `//`, cut.
fn tree_code() -> String {
    include_str!("../src/tree.rs")
        .lines()
        .map(|line| line.find("//").map_or(line, |at| &line[..at]))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The text of the function whose line holds `head`, from that line to the next line that
/// starts a function (`fn ` or `pub fn ` after its blanks).
fn function_text(code: &str, head: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.contains(head))
        .unwrap_or_else(|| panic!("tree.rs holds no `{head}`"));
    let end = lines[start + 1..]
        .iter()
        .position(|line| {
            let line = line.trim_start();
            line.starts_with("fn ") || line.starts_with("pub fn ")
        })
        .map_or(lines.len(), |at| start + 1 + at);
    lines[start..end].join("\n")
}

/// Asserts that the doc comment right above the function `head` has a line that starts with
/// the token of a cost (`Cost:`).
fn assert_cost_line(head: &str) {
    let source = include_str!("../src/tree.rs");
    let lines: Vec<&str> = source.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.contains(head))
        .unwrap_or_else(|| panic!("tree.rs holds no `{head}`"));
    let comment: Vec<&str> = lines[..at]
        .iter()
        .rev()
        .map(|line| line.trim_start())
        .take_while(|line| line.starts_with("///") || line.starts_with("#["))
        .collect();
    let token = ["Cost", ":"].concat();
    assert!(
        comment.iter().any(|line| line
            .trim_start_matches('/')
            .trim_start()
            .starts_with(&token)),
        "the doc comment of `{head}` has no line that starts with the cost token: {comment:#?}"
    );
}

#[test]
fn read_attribute_position() {
    let mut tree = reads();
    let p = by_id(&tree, "p");
    let s = by_id(&tree, "s");
    assert_eq!(tree.attribute_position(p, "id"), Some(0));
    assert_eq!(
        tree.attribute_position(p, "CLASS"),
        Some(1),
        "lowered on HTML"
    );
    assert_eq!(tree.attribute_position(p, "Data-X"), Some(2));
    assert_eq!(tree.attribute_position(p, "none"), None);
    assert_eq!(tree.attribute_position(s, "viewBox"), Some(1));
    assert_eq!(
        tree.attribute_position(s, "viewbox"),
        None,
        "not lowered on SVG"
    );
    tree.set_attribute_ns(p, None, "FOO", "1").unwrap();
    assert_eq!(
        tree.attribute_position(p, "FOO"),
        None,
        "the stored byte is not folded"
    );
    assert_eq!(tree.attribute_position(p, "foo"), None);
    assert_eq!(tree.attribute_position_ns(p, None, "FOO"), Some(3));
    let u = by_id(&tree, "u");
    assert_eq!(tree.attribute_position(u, "xlink:href"), Some(1));
    assert_eq!(tree.attribute_position(u, "href"), None);
    let text = tree.children(p)[0];
    assert_eq!(tree.attribute_position(text, "id"), None, "a text node");
    assert_eq!(
        tree.attribute_position(tree.root(), "id"),
        None,
        "the document"
    );
    assert_cost_line("pub fn attribute_position(");
}

#[test]
fn read_attribute_position_ns() {
    let mut tree = reads();
    let u = by_id(&tree, "u");
    assert_eq!(
        tree.attribute_position_ns(u, Some(Namespace::XLink), "href"),
        Some(1)
    );
    assert_eq!(tree.attribute_position_ns(u, None, "href"), None);
    assert_eq!(
        tree.attribute_position_ns(u, Some(Namespace::XLink), "xlink:href"),
        None,
        "by local name, not by qualified name"
    );
    tree.set_attribute_ns(u, Some(Namespace::XLink), "other:href", "#b")
        .unwrap();
    assert_eq!(
        tree.attribute_position_ns(u, Some(Namespace::XLink), "href"),
        Some(1),
        "a prefix plays no part: the value was replaced in place"
    );
    assert_eq!(attributes_of(&tree, u)[1].value, "#b");
    let p = by_id(&tree, "p");
    assert_eq!(tree.attribute_position_ns(p, None, "class"), Some(1));
    let text = tree.children(p)[0];
    assert_eq!(tree.attribute_position_ns(text, None, "id"), None);
    assert_eq!(tree.attribute_position_ns(tree.root(), None, "id"), None);
    assert_cost_line("pub fn attribute_position_ns(");
}

#[test]
fn read_attribute_names() {
    let mut tree = reads();
    let u = by_id(&tree, "u");
    assert_eq!(tree.attribute_names(u), ["id", "xlink:href"]);
    tree.set_attribute_ns(u, Some(Namespace::XLink), "id", "x")
        .unwrap();
    tree.set_attribute(u, "a", "1").unwrap();
    tree.set_attribute(u, "id", "v").unwrap();
    let names = tree.attribute_names(u);
    assert_eq!(
        names,
        ["id", "xlink:href", "id", "a"],
        "order and duplicates kept"
    );
    let attributes = attributes_of(&tree, u);
    for (name, attribute) in names.iter().zip(&attributes) {
        assert_eq!(name.len(), attribute.qualified_len());
    }
    let p = by_id(&tree, "p");
    let text = tree.children(p)[0];
    assert!(tree.attribute_names(text).is_empty());
    assert!(tree.attribute_names(tree.root()).is_empty());
    assert_cost_line("pub fn attribute_names(");
}

#[test]
fn attribute_stored_bytes() {
    let mut tree = reads();
    let p = by_id(&tree, "p");
    let s = by_id(&tree, "s");
    let added = |tree: &Tree, node: NodeId| attributes_of(tree, node).last().cloned().unwrap();
    let before = tree.stored_bytes();
    tree.set_attribute(p, "Title", "abc").unwrap();
    assert_eq!(added(&tree, p).stored_bytes(), tree.stored_bytes() - before);
    assert_eq!(
        added(&tree, p).stored_bytes(),
        ATTRIBUTE_BYTES + "title".len() + 3
    );
    let before = tree.stored_bytes();
    tree.set_attribute_ns(s, Some(Namespace::XLink), "xlink:title", "t\u{e9}")
        .unwrap();
    assert_eq!(added(&tree, s).stored_bytes(), tree.stored_bytes() - before);
    assert_eq!(
        added(&tree, s).stored_bytes(),
        ATTRIBUTE_BYTES + "xlink".len() + "title".len() + 3
    );
    let before = tree.stored_bytes();
    assert_eq!(tree.toggle_attribute(p, "hidden", None), Ok(true));
    assert_eq!(added(&tree, p).stored_bytes(), tree.stored_bytes() - before);
    assert_cost_line("pub fn stored_bytes(&self) -> usize");
}

#[test]
fn attribute_qualified_len() {
    let plain = Attribute {
        namespace: None,
        prefix: None,
        local_name: "href".to_string(),
        value: "#".to_string(),
    };
    let prefixed = Attribute {
        namespace: Some(Namespace::XLink),
        prefix: Some("xlink".to_string()),
        ..plain.clone()
    };
    assert_eq!(plain.qualified_len(), 4);
    assert_eq!(prefixed.qualified_len(), "xlink:href".len());
    assert_cost_line("pub fn qualified_len(");
}

#[test]
fn namespace_from_url() {
    for namespace in [
        Namespace::Html,
        Namespace::MathMl,
        Namespace::Svg,
        Namespace::XLink,
        Namespace::Xml,
        Namespace::Xmlns,
    ] {
        assert_eq!(Namespace::from_url(namespace.url()), Some(namespace));
        let mut near = namespace.url().as_bytes().to_vec();
        let last = near.len() - 1;
        near[last] ^= 1;
        let near = String::from_utf8(near).unwrap();
        assert_eq!(Namespace::from_url(&near), None, "{near}");
    }
    assert_eq!(Namespace::from_url(""), None);
    assert_eq!(Namespace::from_url("urn:x"), None);
    assert_cost_line("pub fn from_url(");
}

/// One rule for `NamespaceError`: the four conditions of "validate and extract" and a triple
/// that passes, judged by `Tree::namespace_error` and by `set_attribute_ns`, which agree; and
/// the function makes no pass over a name.
#[test]
fn namespace_error_conditions() {
    let xml = Namespace::Xml.url();
    let xmlns = Namespace::Xmlns.url();
    let svg = Namespace::Svg.url();
    // (namespace, prefix, local name, NamespaceError)
    let table: [(Option<&str>, Option<&str>, &str, bool); 10] = [
        (None, Some("a"), "b", true),
        (Some(svg), Some("xml"), "b", true),
        (Some(xml), Some("xml"), "b", false),
        (Some(svg), None, "xmlns", true),
        (Some(svg), Some("xmlns"), "a", true),
        (Some(xmlns), None, "a", true),
        (Some(xmlns), Some("p"), "a", true),
        (Some(xmlns), None, "xmlns", false),
        (Some(xmlns), Some("xmlns"), "a", false),
        (Some(svg), Some("p"), "a", false),
    ];
    for (namespace, prefix, local_name, error) in table {
        assert_eq!(
            Tree::namespace_error(namespace, prefix, local_name),
            error,
            "{namespace:?} {prefix:?} {local_name}"
        );
        let mut tree = reads();
        let p = by_id(&tree, "p");
        let qualified = match prefix {
            Some(prefix) => format!("{prefix}:{local_name}"),
            None => local_name.to_string(),
        };
        let namespace = namespace.map(|url| Namespace::from_url(url).unwrap());
        let written = tree.set_attribute_ns(p, namespace, &qualified, "v");
        assert_eq!(
            written == Err(WriteError::Namespace),
            error,
            "{namespace:?} {qualified}"
        );
    }
    let code = tree_code();
    let text = function_text(&code, "pub fn namespace_error(");
    for token in [
        "split",
        "find(",
        "rfind(",
        "chars()",
        "char_indices(",
        "bytes()",
        "contains(",
        "matches(",
        "trim",
    ] {
        assert!(
            !text.contains(token),
            "namespace_error holds `{token}`:\n{text}"
        );
    }
    assert_cost_line("pub fn namespace_error(");
}

/// One matching rule: a read finds an attribute exactly when the write of the same name finds
/// it, at the same position.
#[test]
fn one_matching_rule_for_reads_and_writes() {
    let mut tree = reads();
    let p = by_id(&tree, "p");
    let s = by_id(&tree, "s");
    let u = by_id(&tree, "u");
    tree.set_attribute_ns(p, None, "FOO", "1").unwrap();
    tree.set_attribute_ns(p, Some(Namespace::XLink), "x:id", "2")
        .unwrap();
    tree.set_attribute_ns(s, Some(Namespace::XLink), "viewBox", "3")
        .unwrap();
    let count = |tree: &Tree, node: NodeId| attributes_of(tree, node).len();
    let names = [
        "id",
        "ID",
        "Class",
        "FOO",
        "foo",
        "x:id",
        "X:ID",
        "viewBox",
        "viewbox",
        "xlink:href",
        "href",
        "absent",
    ];
    for element in [p, s, u] {
        for name in names {
            let found = tree.attribute_position(element, name);
            let mut removed = tree.clone();
            removed.remove_attribute(element, name).unwrap();
            assert_eq!(
                found.is_some(),
                count(&removed, element) < count(&tree, element),
                "{name}"
            );
            if let Some(at) = found {
                let mut set = tree.clone();
                set.set_attribute(element, name, "new").unwrap();
                assert_eq!(attributes_of(&set, element)[at].value, "new", "{name}");
                assert_eq!(set.attribute_names(element), tree.attribute_names(element));
            }
        }
        for namespace in [None, Some(Namespace::XLink), Some(Namespace::Svg)] {
            for local in ["id", "FOO", "foo", "viewBox", "href", "absent"] {
                let found = tree.attribute_position_ns(element, namespace, local);
                let mut removed = tree.clone();
                removed
                    .remove_attribute_ns(element, namespace, local)
                    .unwrap();
                assert_eq!(
                    found.is_some(),
                    count(&removed, element) < count(&tree, element),
                    "{namespace:?} {local}"
                );
                if let Some(at) = found {
                    let mut set = tree.clone();
                    set.set_attribute_ns(element, namespace, local, "new")
                        .unwrap();
                    assert_eq!(attributes_of(&set, element)[at].value, "new");
                    assert_eq!(set.attribute_names(element), tree.attribute_names(element));
                }
            }
        }
    }
}

/// The position rule lowers the wanted name byte by byte and copies nothing, and no removal
/// copies the name: the texts of `position_of`, `position_of_ns`, `remove_attribute`,
/// `remove_attribute_ns` and the removing arm of `toggle_attribute` hold no copy and call no
/// function of the file but the few the rule needs.
#[test]
fn removals_copy_no_name() {
    const FORBIDDEN: [&str; 23] = [
        "attribute_name(",
        "to_ascii_lowercase(",
        "to_lowercase(",
        "make_ascii_lowercase(",
        ".to_owned(",
        ".to_string(",
        "ToString::to_string(",
        "to_vec(",
        ".clone(",
        ".cloned(",
        ".into(",
        ".into_owned(",
        ".repeat(",
        ".push_str(",
        ".extend(",
        "String::",
        "Vec::",
        "Box::",
        "Rc::from(",
        "Arc::from(",
        "Cow",
        "format!",
        ".collect",
    ];
    const CALLABLE: [&str; 7] = [
        "lower_byte",
        "element_attributes",
        "drop_attribute",
        "position_of",
        "position_of_ns",
        "qualified_len",
        "qualified_name_is",
    ];
    let code = tree_code();
    let toggle = function_text(&code, "pub fn toggle_attribute(");
    let lines: Vec<&str> = toggle.lines().collect();
    let adding = lines
        .iter()
        .position(|line| line.trim_start().starts_with("None =>"))
        .unwrap_or_else(|| panic!("toggle_attribute has no arm that starts `None =>`:\n{toggle}"));
    let removing = lines
        .iter()
        .position(|line| line.trim_start().starts_with("Some(at) =>"))
        .unwrap_or_else(|| {
            panic!("toggle_attribute has no arm that starts `Some(at) =>`:\n{toggle}")
        });
    assert!(
        lines[removing].trim_end().ends_with('{'),
        "the line of `Some(at) =>` ends with no `{{`:\n{toggle}"
    );
    let first_lowering = lines
        .iter()
        .position(|line| line.contains("attribute_name("));
    assert!(
        first_lowering.is_some_and(|at| at > adding),
        "toggle_attribute lowers the name before its adding arm:\n{toggle}"
    );
    let arm = braced(&lines[removing..].join("\n"));
    let texts = [
        ("position_of", function_text(&code, "fn position_of(")),
        ("position_of_ns", function_text(&code, "fn position_of_ns(")),
        (
            "remove_attribute",
            function_text(&code, "pub fn remove_attribute("),
        ),
        (
            "remove_attribute_ns",
            function_text(&code, "pub fn remove_attribute_ns("),
        ),
        ("the removing arm of toggle_attribute", arm),
    ];
    let functions = function_names(&code);
    for (name, text) in &texts {
        for token in FORBIDDEN {
            assert!(!text.contains(token), "{name} holds `{token}`:\n{text}");
        }
        for call in calls(text) {
            assert!(
                !functions.contains(&call) || CALLABLE.contains(&call.as_str()),
                "{name} calls `{call}` of tree.rs:\n{text}"
            );
        }
    }
}

/// The text from the first `{` of `text` to the `}` that closes it, by counted braces.
fn braced(text: &str) -> String {
    let start = text.find('{').expect("a brace");
    let mut depth = 0_usize;
    for (at, c) in text[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return text[start..start + at + 1].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("no closing brace:\n{text}");
}

/// The names after `fn ` in `code`.
fn function_names(code: &str) -> HashSet<String> {
    code.match_indices("fn ")
        .filter(|(at, _)| *at == 0 || !code.as_bytes()[at - 1].is_ascii_alphanumeric())
        .map(|(at, _)| identifier_at(&code[at + 3..]))
        .filter(|name| !name.is_empty())
        .collect()
}

fn identifier_at(text: &str) -> String {
    text.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

/// The calls in `text`: a name before `(` with no receiver, or with the receiver `self.` or
/// `Self::`, or a path, and `.qualified_len(`; a name after `fn ` is a definition, not a call.
fn calls(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    for (open, _) in text.match_indices('(') {
        let mut start = open;
        while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
            start -= 1;
        }
        let name = &text[start..open];
        if name.is_empty() || name.as_bytes()[0].is_ascii_digit() {
            continue;
        }
        let before = &text[..start];
        if before.ends_with("fn ") {
            continue;
        }
        let method = before.ends_with('.');
        let on_self = before.ends_with("self.");
        if !method || on_self || name == "qualified_len" {
            found.push(name.to_string());
        }
    }
    found
}

/// The room of the attribute list of `node`, in records.
fn room_of(tree: &Tree, node: NodeId) -> usize {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes.capacity(),
        _ => 0,
    }
}

/// The bytes the tree holds by the charge's rule, counted afresh: the strings of every node and
/// [`ATTRIBUTE_BYTES`] for every record slot an attribute list holds room for.
/// Every node of the tree is under its root here.
fn held_afresh(tree: &Tree) -> usize {
    let lists: usize = tree
        .descendants(tree.root())
        .into_iter()
        .map(|node| match tree.data(node) {
            NodeData::Element { attributes, .. } => {
                (attributes.capacity() - attributes.len()) * ATTRIBUTE_BYTES
            }
            _ => 0,
        })
        .sum();
    tree.stored_bytes() + lists
}

/// The bytes of the strings of the attribute `name` (no prefix) of `node`.
fn strings_of(tree: &Tree, node: NodeId, name: &str) -> usize {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes
            .iter()
            .find(|attribute| attribute.local_name == name)
            .map(Attribute::string_bytes)
            .expect("the attribute"),
        _ => panic!("an element"),
    }
}

/// The room reads say, before each write, what the write does to the list's allocation and to
/// `held_bytes`: an add gains `attribute_room_grown` (a list the parser built at its exact
/// length doubles at the first add, a short list grows by the one record, a list with room
/// gains nothing), a removal gives back `attribute_room_freed` (all of it when the list
/// empties), `held_bytes` moves by those and the strings, and a node that is no element has no
/// room.
#[test]
fn read_attribute_room() {
    let names: String = (0..512).map(|i| format!(" a{i}")).collect();
    let page = format!("<!DOCTYPE html><body><p id=r{names}></p><i id=e></i>");
    let mut tree = parse_document(&page, &scripted());
    let p = by_id(&tree, "r");
    assert_eq!(
        room_of(&tree, p),
        513,
        "the parser builds the list at its length"
    );
    assert_eq!(
        tree.held_bytes(),
        tree.stored_bytes(),
        "no room past a parsed list"
    );
    assert_eq!(tree.attribute_room_grown(p), 513 * ATTRIBUTE_BYTES);
    let mut added = Vec::new();
    for n in 0..400 {
        let (before, held) = (room_of(&tree, p), tree.held_bytes());
        let grown = tree.attribute_room_grown(p);
        let name = format!("z{n}");
        tree.set_attribute(p, &name, "").unwrap();
        let after = room_of(&tree, p);
        assert_eq!((after - before) * ATTRIBUTE_BYTES, grown, "add {n}");
        assert_eq!(tree.held_bytes(), held + grown + name.len(), "add {n}");
        added.push(name);
    }
    assert_eq!(room_of(&tree, p), 1026, "one doubling in 400 adds");
    assert_eq!(tree.held_bytes(), held_afresh(&tree));
    let mut shrunk = 0;
    let all: Vec<String> = (0..512)
        .map(|i| format!("a{i}"))
        .chain(added)
        .chain(["id".to_string()])
        .collect();
    for name in &all {
        let (before, held) = (room_of(&tree, p), tree.held_bytes());
        let freed = tree.attribute_room_freed(p);
        let strings = strings_of(&tree, p, name);
        tree.remove_attribute(p, name).unwrap();
        let after = room_of(&tree, p);
        assert_eq!(
            freed,
            (before - after) * ATTRIBUTE_BYTES,
            "remove {name}: {before} -> {after} records"
        );
        assert_eq!(tree.held_bytes(), held - freed - strings, "remove {name}");
        if after < before {
            shrunk += 1;
        }
    }
    assert!(attributes_of(&tree, p).is_empty());
    assert_eq!(room_of(&tree, p), 0, "an emptied list gives its room back");
    assert!(shrunk >= 3, "{shrunk} shrinks");
    let e = by_id(&tree, "e");
    let mut rooms = Vec::new();
    for n in 0..6 {
        let before = room_of(&tree, e);
        let grown = tree.attribute_room_grown(e);
        tree.set_attribute(e, &format!("b{n}"), "").unwrap();
        let after = room_of(&tree, e);
        assert_eq!((after - before) * ATTRIBUTE_BYTES, grown, "short add {n}");
        rooms.push(after);
    }
    assert_eq!(
        rooms,
        [2, 3, 4, 8, 8, 8],
        "1 (the id), 2, 3, 4, then 8 records"
    );
    // Shrunk to its entries at 3 left of 8; the last removal empties it.
    let removals = [
        ("b5", 8),
        ("b4", 8),
        ("b3", 8),
        ("b2", 3),
        ("b1", 3),
        ("b0", 3),
        ("id", 0),
    ];
    for (name, kept) in removals {
        let freed = tree.attribute_room_freed(e);
        let before = room_of(&tree, e);
        tree.remove_attribute(e, name).unwrap();
        assert_eq!(room_of(&tree, e), kept, "remove {name}");
        assert_eq!(freed, (before - kept) * ATTRIBUTE_BYTES, "remove {name}");
    }
    assert_eq!(tree.held_bytes(), held_afresh(&tree));
    assert_eq!(tree.attribute_room_grown(tree.root()), 0);
    assert_eq!(tree.attribute_room_freed(tree.root()), 0);
    assert_cost_line("pub fn attribute_room_grown(");
    assert_cost_line("pub fn attribute_room_freed(");
    assert_cost_line("pub fn held_bytes(");
}
