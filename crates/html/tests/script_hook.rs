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
