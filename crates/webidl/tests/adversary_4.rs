//! Third review of `story:webidl-bindings`, on its fix round 2: the reader of a page
//! (`html.rs`, HTML 13.2.5 and 13.2.6.4.7) beyond the cases of the second review, its time on
//! 8,000 of each construct, and the Rust names the generator writes (`identifier`, `claim`).

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use surf_webidl::{Model, Selection, cut_idl_blocks, extract_selected, generate, read};

fn wanted() -> BTreeMap<String, BTreeSet<String>> {
    let set = |names: &[&str]| names.iter().map(|name| name.to_string()).collect();
    BTreeMap::from([("Window".to_string(), set(&["a", "b"]))])
}

const A: &str = "interface Window { readonly attribute boolean a; };";
const B: &str = "interface Window { readonly attribute boolean b; };";

/// The pages of the tokenizer rules beyond the second review's cases, each with the blocks
/// HTML's tokenizer finds and the attribute names read from them.
fn tokenizer_pages() -> Vec<(&'static str, String, &'static [&'static str])> {
    vec![
        // 13.2.5.11 to 13.2.5.13: the RCDATA end tag name is compared in ASCII lowercase.
        (
            "TEXTAREA in capitals",
            format!("<TEXTAREA><pre class=idl>{A}</pre></TEXTAREA><pre class=idl>{B}</pre>\n"),
            &["b"],
        ),
        (
            "an end tag of mixed case and a space",
            format!("<xmp><pre class=idl>{A}</pre></XmP ><pre class=idl>{B}</pre>\n"),
            &["b"],
        ),
        (
            "an end tag ended by a solidus",
            format!("<title><pre class=idl>{A}</pre></title/><pre class=idl>{B}</pre>\n"),
            &["b"],
        ),
        (
            "an end tag ended by a tab",
            format!("<iframe><pre class=idl>{A}</pre></iframe\t><pre class=idl>{B}</pre>\n"),
            &["b"],
        ),
        (
            "an end tag ended by a form feed",
            format!("<noembed><pre class=idl>{A}</pre></noembed\x0c><pre class=idl>{B}</pre>\n"),
            &["b"],
        ),
        // U+000B is no whitespace of HTML: `</textarea` and VT is no appropriate end tag.
        (
            "an end tag followed by a vertical tab",
            format!("<textarea></textarea\x0b><pre class=idl>{A}</pre></textarea>\n"),
            &[],
        ),
        (
            "</textarea at the end of the input",
            format!("<pre class=idl>{A}</pre><textarea><pre class=idl>{B}</pre></textarea"),
            &["a"],
        ),
        (
            "</textarea and a letter at the end of the input",
            format!("<pre class=idl>{A}</pre><textarea><pre class=idl>{B}</pre></textareas"),
            &["a"],
        ),
        // An element of the list inside another: the outer one's text ends at its own end tag.
        (
            "title inside textarea",
            format!(
                "<textarea><title></title><pre class=idl>{A}</pre></textarea><pre class=idl>{B}</pre>\n"
            ),
            &["b"],
        ),
        (
            "textarea inside title",
            format!("<title><textarea></title><pre class=idl>{A}</pre>\n"),
            &["a"],
        ),
        (
            "style inside script",
            format!("<script><style></script><pre class=idl>{A}</pre></style>\n"),
            &["a"],
        ),
        (
            "plaintext inside noframes",
            format!("<noframes><plaintext></noframes><pre class=idl>{A}</pre>\n"),
            &["a"],
        ),
        // 13.2.5.43: `<plaintext>` inside a comment or as an end tag is no plaintext.
        (
            "plaintext inside a comment",
            format!("<!-- <plaintext> --><pre class=idl>{A}</pre>\n"),
            &["a"],
        ),
        (
            "an end tag of plaintext",
            format!("</plaintext><pre class=idl>{A}</pre>\n"),
            &["a"],
        ),
        (
            "PLAINTEXT in capitals",
            format!("<pre class=idl>{A}</pre><PLAINTEXT><pre class=idl>{B}</pre>\n"),
            &["a"],
        ),
        // 13.2.5.44: `<!-->` ends at once; a `--!>` after it is text.
        (
            "--!> after <!-->",
            format!("<!-->--!><pre class=idl>{A}</pre>\n"),
            &["a"],
        ),
        (
            "--!> after <!--->",
            format!("<!--->--!><pre class=idl>{A}</pre>\n"),
            &["a"],
        ),
        // 13.2.5.52: `--!-->` ends at its `-->`, after the comment end bang state.
        (
            "--!--> ends a comment at its end",
            format!("<!-- x --!--><pre class=idl>{A}</pre>\n"),
            &["a"],
        ),
        (
            "--!x does not end a comment",
            format!("<!-- x --!x <pre class=idl>{A}</pre> --><pre class=idl>{B}</pre>\n"),
            &["b"],
        ),
        (
            "<!---> inside a comment does not end it early",
            format!("<!-- a <!---> <pre class=idl>{A}</pre>\n"),
            &["a"],
        ),
    ]
}

/// The attribute names of `Window` read from `page`, or `None` when the page is refused.
fn read_names(page: &[u8]) -> Option<Vec<String>> {
    let idl = extract_selected(page, "page.html", &wanted()).ok()?;
    let mut names = Vec::new();
    for name in ["a", "b"] {
        if idl.text.contains(&format!("boolean {name};")) {
            names.push(name.to_string());
        }
    }
    Some(names)
}

/// Held. HTML 13.2.5 and 13.2.6.4.7 beyond the second review's cases: end tags in capitals or
/// mixed case, ended by a space, a tab, a form feed or a solidus but not a vertical tab;
/// `</textarea` at the end of the input; an element of the list inside another; `<plaintext>`
/// inside a comment, as an end tag and in capitals; `--!>` after `<!-->` and `<!--->`; `--!-->`
/// and `--!x`. Each page gives the blocks HTML's tokenizer finds, and its cut gives what the
/// page gives.
#[test]
fn the_tokenizer_rules_beyond_the_second_review() {
    let mut wrong = Vec::new();
    for (what, page, names) in tokenizer_pages() {
        let got = read_names(page.as_bytes());
        let expected: Vec<String> = names.iter().map(|name| name.to_string()).collect();
        if got.as_ref() != Some(&expected) {
            wrong.push(format!("{what}: {got:?}, HTML reads {expected:?}"));
        }
        let from_page = extract_selected(page.as_bytes(), "page.html", &wanted());
        let cut = cut_idl_blocks(page.as_bytes());
        let from_cut = extract_selected(&cut, "page.html", &wanted());
        let same = match (&from_page, &from_cut) {
            (Ok(a), Ok(b)) => a == b,
            (Err(_), Err(_)) => true,
            _ => false,
        };
        if !same {
            wrong.push(format!("{what}: page {from_page:?}, cut {from_cut:?}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Note (documented in `html.rs:173-175`, not HTML). HTML 13.2.5.6, end tag open state: `</`
/// followed by anything but an ASCII letter or `>` starts a bogus comment, which ends at the
/// next `>`; so in `</ <pre class=idl>...` the `pre` start tag is inside a comment, and HTML
/// has no block there. The reader reads the `<` of `</ ` as text and finds one. Held here as
/// the documented reading: the cut gives what the page gives.
#[test]
fn an_end_tag_open_without_a_letter_is_read_as_documented() {
    let page = format!("</ <pre class=idl>{A}</pre><pre class=idl>{B}</pre>\n");
    assert_eq!(
        read_names(page.as_bytes()),
        Some(vec!["a".to_string(), "b".to_string()])
    );
    let cut = cut_idl_blocks(page.as_bytes());
    assert_eq!(read_names(&cut), read_names(page.as_bytes()));
}

/// Held. Note 2 as ruled, for the constructs the round's case does not name: 8,000 of each
/// (`<!doctype` open and closed, `<![CDATA[`, `<?`, `</ >`, `</` alone, an attribute without a
/// value closed and open, an unquoted value the end cuts, a `<` in text, a `pre` of the class
/// `idl` that no end tag closes, a `pre` followed by a `code` whose quote no quote ends, and
/// `<pre class=idl></pre>` blocks) are cut in under a second each.
#[test]
fn eight_thousand_of_each_other_construct_are_cut_in_under_a_second() {
    let pages = [
        ("<!doctype open", "<!doctype ".repeat(8_000)),
        ("<!doctype closed", "<!doctype html>".repeat(8_000)),
        ("<![CDATA[", "<![CDATA[ x ".repeat(8_000)),
        ("<?", "<?x ".repeat(8_000)),
        ("</ >", "</ >".repeat(8_000)),
        ("</ alone", "</".repeat(8_000)),
        ("an attribute without a value", "<a b>".repeat(8_000)),
        (
            "an attribute without a value, open",
            "<a b c d ".repeat(8_000),
        ),
        ("an unquoted value the end cuts", "<a b=c".repeat(8_000)),
        ("< in text", "a < b ".repeat(8_000)),
        ("an open pre.idl", "<pre class=idl>".repeat(8_000)),
        (
            "pre then code with an open quote",
            "<pre><code class=\"".repeat(8_000),
        ),
        (
            "end tags of script without a space",
            format!("<script>{}", "</script".repeat(8_000)),
        ),
        ("comments of --!", format!("<!--{}", "--!".repeat(8_000))),
    ];
    let mut slow = Vec::new();
    for (what, page) in &pages {
        let started = Instant::now();
        let cut = cut_idl_blocks(page.as_bytes());
        let took = started.elapsed();
        assert!(cut.is_empty(), "{what}: {} bytes cut", cut.len());
        if took >= Duration::from_secs(1) {
            slow.push(format!("{what}: {took:?}"));
        }
    }
    let blocks = "<pre class=idl></pre>".repeat(8_000);
    let started = Instant::now();
    let cut = cut_idl_blocks(blocks.as_bytes());
    let took = started.elapsed();
    assert_eq!(cut.len(), blocks.len() + 8_000);
    if took >= Duration::from_secs(1) {
        slow.push(format!("8,000 empty blocks: {took:?}"));
    }
    assert!(slow.is_empty(), "{slow:#?}");
}

fn model(idl: &str) -> Result<Model, String> {
    let definitions = read(idl, "small.idl").map_err(|error| format!("read: {error}"))?;
    Model::new(definitions).map_err(|error| format!("model: {error}"))
}

const DOM_EXCEPTION: &str = "interface DOMException { readonly attribute DOMString name; };\n";

/// The bindings of `extra` IDL beside `EventTarget`, `Node`, `Window` and `DOMException`, with
/// `definitions` more and `rows`.
fn generated_with(extra: &str, definitions: &str, rows: &str) -> Result<String, String> {
    let idl = format!(
        "interface EventTarget {{}};\ninterface Window : EventTarget {{}};\n{extra}{DOM_EXCEPTION}"
    );
    let selection = Selection::read(
        &format!(
            "definitions: EventTarget Window DOMException {definitions}\nglobal: Window\n{rows}"
        ),
        "s.txt",
    )
    .map_err(|error| format!("selection: {error}"))?;
    generate(&model(&idl)?, &selection, &[]).map_err(|error| format!("generate: {error}"))
}

/// The parameter names of each `fn` of `text` whose parameters start with `&mut self`.
fn trait_methods(text: &str) -> Vec<(String, Vec<String>)> {
    let joined = text.replace('\n', " ");
    let mut out = Vec::new();
    let mut rest = joined.as_str();
    while let Some(at) = rest.find("fn ") {
        rest = &rest[at + 3..];
        let Some(open) = rest.find('(') else { break };
        let name = rest[..open].trim().to_string();
        let after = &rest[open + 1..];
        if !after.trim_start().starts_with("&mut self") {
            continue;
        }
        let mut depth = 1usize;
        let mut end = after.len();
        for (index, c) in after.char_indices() {
            match c {
                '(' | '<' => depth += 1,
                ')' | '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = index;
                        break;
                    }
                }
                _ => {}
            }
        }
        let params = after[..end]
            .split(',')
            .filter_map(|param| {
                param
                    .split_once(':')
                    .map(|(name, _)| name.trim().to_string())
            })
            .filter(|name| !name.is_empty())
            .collect();
        out.push((name, params));
    }
    out
}

/// What rustc refuses by name in `text`: a trait method with two parameters of one name
/// (E0415), two free functions of one name (E0428), two trait methods of one name in one trait
/// (E0428), two variants of `Interface` of one name (E0428).
fn names_twice(text: &str) -> Vec<String> {
    let mut wrong = Vec::new();
    for (name, params) in trait_methods(text) {
        let unique: BTreeSet<&String> = params.iter().collect();
        if unique.len() != params.len() {
            wrong.push(format!("fn {name}({})", params.join(", ")));
        }
    }
    let mut free = BTreeSet::new();
    let mut methods = BTreeSet::new();
    let mut variants = BTreeSet::new();
    let mut in_interface = false;
    let mut block = String::new();
    for line in text.lines() {
        let name_of = |rest: &str| rest.split(['(', '<', ' ']).next().unwrap_or("").to_string();
        if line.starts_with("pub trait ") || line.starts_with("impl") {
            block = line.to_string();
        }
        if let Some(rest) = line
            .strip_prefix("fn ")
            .or_else(|| line.strip_prefix("pub fn "))
        {
            let name = name_of(rest);
            if !free.insert(name.clone()) {
                wrong.push(format!("free fn {name} twice"));
            }
        }
        let trimmed = line.trim_start();
        if line.starts_with("    ")
            && let Some(rest) = trimmed.strip_prefix("fn ")
        {
            let name = name_of(rest);
            if !methods.insert((block.clone(), name.clone())) {
                wrong.push(format!("fn {name} twice in `{block}`"));
            }
        }
        if line.starts_with("pub enum Interface") {
            in_interface = true;
            continue;
        }
        if in_interface {
            if line.starts_with('}') {
                in_interface = false;
            } else if let Some(variant) = trimmed.strip_suffix(',')
                && !trimmed.starts_with("//")
                && !trimmed.starts_with('#')
                && !variants.insert(variant.to_string())
            {
                wrong.push(format!("Interface::{variant} twice"));
            }
        }
    }
    wrong
}

/// Red (note). The round renames an argument named like the generator's own parameters with a
/// `_` (`cx_`, `this_`), the rule of `self`; the duplicate check of one member's arguments
/// (`generate.rs`, "the arguments ... are both written") must then see the renamed name meet an
/// argument that is written so already (`cx` beside `cx_`, `this` beside `this_`, `self` beside
/// `self_`, `Self` beside `self_`): refused naming the row, or two distinct names.
#[test]
fn a_renamed_argument_that_meets_another_argument_is_refused() {
    let mut wrong = Vec::new();
    for (one, two) in [
        ("cx", "cx_"),
        ("this", "this_"),
        ("self", "self_"),
        ("cx_", "cx"),
    ] {
        let extra = format!(
            "interface Node : EventTarget {{\n  undefined a(DOMString {one}, DOMString {two});\n}};\n"
        );
        match generated_with(&extra, "Node", "Node a mutation\n") {
            Ok(text) => wrong.extend(
                names_twice(&text)
                    .into_iter()
                    .map(|w| format!("{one}/{two}: {w}")),
            ),
            Err(error) if error.contains("are both written") => {}
            Err(error) => wrong.push(format!("{one}/{two}: {error}")),
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Held. Note 4 as fixed, across interfaces: `Node.fooBar` and `NodeFoo.bar` are both
/// `node_foo_bar`, and the setter of `Node.x` and the getter of `SetNode.x` are both
/// `set_node_x`: refused naming both members; and across a mixin and a partial interface.
#[test]
fn member_names_of_two_interfaces_or_a_mixin_or_a_partial_are_distinct() {
    let cases = [
        (
            "interface Node : EventTarget {\n  undefined fooBar();\n};\ninterface NodeFoo {\n  undefined bar();\n};\n",
            "Node NodeFoo",
            "Node fooBar mutation\nNodeFoo bar mutation\n",
        ),
        (
            "interface Node : EventTarget {\n  attribute DOMString x;\n};\ninterface SetNode {\n  readonly attribute DOMString x;\n};\n",
            "Node SetNode",
            "Node x attributes\nSetNode x reading\n",
        ),
        (
            "interface Node : EventTarget {\n  undefined fooBar();\n};\ninterface mixin M {\n  undefined foo_bar();\n};\nNode includes M;\n",
            "Node",
            "Node fooBar mutation\nNode foo_bar mutation\n",
        ),
        (
            "interface Node : EventTarget {\n  undefined fooBar();\n};\npartial interface Node {\n  undefined foo_bar();\n};\n",
            "Node",
            "Node fooBar mutation\nNode foo_bar mutation\n",
        ),
    ];
    let mut wrong = Vec::new();
    for (extra, definitions, rows) in cases {
        match generated_with(extra, definitions, rows) {
            Ok(text) => wrong.push(format!("{rows}: generated, {:?}", names_twice(&text))),
            Err(error) if error.contains("are both written") => {}
            Err(error) => wrong.push(format!("{rows}: {error}")),
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Red (note). The round claims the names of attributes and operations; the names the generator
/// writes per interface are not claimed: `HTMLFoo` and `HtmlFoo` (two Web IDL identifiers) are
/// both `html_foo` and `HtmlFoo`, so the file holds two variants `Interface::HtmlFoo` and two
/// `fn interface_html_foo`, which rustc refuses (E0428). Refused naming both, or distinct.
#[test]
fn two_interfaces_of_one_rust_name_are_refused() {
    let extra = "interface HTMLFoo {\n  readonly attribute DOMString x;\n};\n\
                 interface HtmlFoo {\n  readonly attribute DOMString y;\n};\n";
    match generated_with(
        extra,
        "HTMLFoo HtmlFoo",
        "HTMLFoo x reading\nHtmlFoo y reading\n",
    ) {
        Ok(text) => {
            let wrong = names_twice(&text);
            assert!(wrong.is_empty(), "{wrong:#?}");
        }
        Err(error) => assert!(
            error.contains("HTMLFoo") && error.contains("HtmlFoo"),
            "a refusal naming neither: {error}"
        ),
    }
}

/// Red (note). Note 8's rule covers the dictionary members, and the converter of a dictionary
/// binds each member to a local of the member's Rust name
/// (`let {name} = dictionary_member(cx, object, ...)`, `generate.rs:1630`) after its own local
/// `object`, the dictionary every member is read from. A member `_object` (Web IDL 2.2: an
/// escaped identifier, the name `object`) is written `let object = dictionary_member(...)` and
/// then `let object = ... to_boolean(...)`, so the next member is read from a `bool` and the
/// file does not compile (E0308). Renamed or refused. (`member` and `value` shadow the
/// parameters too; neither is read after the first member in the converters of this file.)
#[test]
fn a_dictionary_member_named_like_a_local_of_its_converter_is_renamed() {
    let extra = "dictionary D {\n  boolean _object = false;\n  boolean z = false;\n};\n\
                 interface Node : EventTarget {\n  undefined a(optional D d = {});\n};\n";
    match generated_with(extra, "Node", "Node a mutation\n") {
        Ok(text) => {
            let start = text.find("fn convert_d(").expect("a converter of D");
            let body = &text[start..start + text[start..].find("\n}").unwrap_or(0)];
            assert!(
                !body.contains("let object = dictionary_member("),
                "the member `object` shadows the dictionary:\n{body}"
            );
        }
        Err(error) => assert!(error.contains("object"), "{error}"),
    }
}

/// Held. Web IDL 2.5.3 allows overloaded operations: two operations of one identifier on
/// one interface. One row names both (`Node x mutation`); the generator writes them as one
/// method or refuses the row naming the overload, never two trait methods `node_x` or two
/// functions `operation_node_x` (E0428), nor one that takes the arguments of one overload and
/// silently drops the other. The reader refuses the second operation ("an overload").
#[test]
fn an_overloaded_operation_is_written_once_or_refused() {
    let extra =
        "interface Node : EventTarget {\n  undefined x();\n  undefined x(DOMString s);\n};\n";
    match generated_with(extra, "Node", "Node x mutation\n") {
        Ok(text) => {
            let wrong = names_twice(&text);
            assert!(wrong.is_empty(), "{wrong:#?}");
            let methods: Vec<_> = trait_methods(&text)
                .into_iter()
                .filter(|(name, _)| name == "node_x")
                .collect();
            assert_eq!(methods.len(), 1, "{methods:?}");
        }
        Err(error) => assert!(error.contains("an overload"), "{error}"),
    }
}
