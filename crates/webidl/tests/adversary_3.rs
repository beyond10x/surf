//! Second review of `story:webidl-bindings`, on its fix round 1: the page reader and the cut of
//! the HTML Standard read as tags (`html.rs`), and the names and doc text the generator writes.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use surf_webidl::{Model, Selection, cut_idl_blocks, extract_selected, generate, read};

fn wanted() -> BTreeMap<String, BTreeSet<String>> {
    let set = |names: &[&str]| names.iter().map(|name| name.to_string()).collect();
    BTreeMap::from([("Window".to_string(), set(&["a", "b"]))])
}

const A: &str = "interface Window { readonly attribute boolean a; };";
const B: &str = "interface Window { readonly attribute boolean b; };";

/// The blocks of `page` and the text read, or `None` when the page is refused.
fn blocks(page: &[u8]) -> Option<(usize, String)> {
    extract_selected(page, "page.html", &wanted())
        .ok()
        .map(|idl| (idl.blocks, idl.text))
}

/// Pages a tag reader can get wrong. Each is `(what, bytes)`.
fn pages() -> Vec<(&'static str, Vec<u8>)> {
    let page = |text: String| text.into_bytes();
    vec![
        (
            "--!> ends a comment",
            page(format!("<!-- x --!><pre class=idl>{A}</pre><!-- y -->\n")),
        ),
        ("<!-->", page(format!("<!--><pre class=idl>{A}</pre>\n"))),
        ("<!--->", page(format!("<!---><pre class=idl>{A}</pre>\n"))),
        (
            "textarea",
            page(format!("<textarea><pre class=idl>{A}</pre></textarea>\n")),
        ),
        (
            "title",
            page(format!("<title><pre class=idl>{A}</pre></title>\n")),
        ),
        (
            "script closed in capitals",
            page(format!(
                "<script>x = '<pre class=idl>{A}</pre>';</SCRIPT><pre class=idl>{B}</pre>\n"
            )),
        ),
        (
            "style",
            page(format!(
                "<style>p::before {{ content: '<pre class=idl>{A}</pre>'; }}</style>\
                 <pre class=idl>{B}</pre>\n"
            )),
        ),
        (
            "</scriptx> does not end a script",
            page(format!("<script></scriptx><pre class=idl>{A}</pre></script>\n")),
        ),
        (
            "capitals",
            page(format!("<PRE CLASS=\"idl\">{A}</PRE>\n")),
        ),
        (
            "> in a single-quoted value",
            page(format!("<pre title='a>b' class='idl'>{A}</pre>\n")),
        ),
        (
            "> ends an unquoted value",
            page(format!("<pre title=a>b class=idl>{A}</pre>\n")),
        ),
        (
            "code.idl inside pre",
            page(format!("<pre><code class=idl>{A}</code></pre>\n")),
        ),
        (
            "an example",
            page(format!("<pre class=\"idl extract\">{A}</pre>\n")),
        ),
        (
            "a comment inside a block hides its </pre>",
            page(
                "<pre class=idl>interface Window { readonly attribute boolean b<!-- </pre> -->; };</pre>\n"
                    .to_string(),
            ),
        ),
        (
            "an unterminated quote at the end",
            page(format!("<pre class=idl>{A}</pre>\n<p title=\"open")),
        ),
        (
            "an unterminated comment at the end",
            page(format!("<pre class=idl>{A}</pre>\n<!-- open")),
        ),
        (
            "an unterminated block at the end",
            page(format!("<pre class=idl>{A}</pre>\n<pre class=idl>{B}")),
        ),
        (
            "a NUL in a tag and in a block",
            page(format!("<pre class=idl\0>{A}</pre>\n<pre class=idl>\0{B}</pre>\n")),
        ),
        ("bytes that are not UTF-8 between blocks", {
            let mut bytes = b"\xff\xfe<p>\x80</p>".to_vec();
            bytes.extend_from_slice(format!("<pre class=idl>{A}</pre>").as_bytes());
            bytes.push(0xff);
            bytes
        }),
        ("bytes that are not UTF-8 in a block", {
            let mut bytes = b"<pre class=idl>".to_vec();
            bytes.extend_from_slice(A.as_bytes());
            bytes.extend_from_slice(b"\xff</pre>\n");
            bytes
        }),
        (
            "a pre start tag cut by the end of the input",
            page(format!("<pre class=idl>{A}</pre>\n<pre class=\"idl")),
        ),
    ]
}

/// Held. Note 9 as fixed: the extraction and the cut read the page with one scan of its tags,
/// so for every page here the cut gives what the page gives (both refused, or the same
/// blocks, text and skipped parts).
#[test]
fn every_page_and_its_cut_give_one_model() {
    let mut wrong = Vec::new();
    for (what, page) in pages() {
        let from_page = extract_selected(&page, "page.html", &wanted());
        let cut = cut_idl_blocks(&page);
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

/// Held. The pages HTML's tokenizer reads as the scan does (HTML 13.2.5: the comment start
/// states, the script data and RAWTEXT states, attribute values): the blocks it finds and the
/// text read.
#[test]
fn the_blocks_html_reads_are_the_blocks() {
    let expected: &[(&str, Option<(usize, &str)>)] = &[
        ("<!-->", Some((1, "boolean a"))),
        ("<!--->", Some((1, "boolean a"))),
        ("script closed in capitals", Some((1, "boolean b"))),
        ("style", Some((1, "boolean b"))),
        ("capitals", Some((1, "boolean a"))),
        ("> in a single-quoted value", Some((1, "boolean a"))),
        ("> ends an unquoted value", Some((0, ""))),
        ("code.idl inside pre", Some((1, "boolean a"))),
        ("an example", Some((0, ""))),
        (
            "a comment inside a block hides its </pre>",
            Some((1, "boolean b")),
        ),
        ("an unterminated comment at the end", Some((1, "boolean a"))),
        (
            "bytes that are not UTF-8 between blocks",
            Some((1, "boolean a")),
        ),
        ("bytes that are not UTF-8 in a block", None),
    ];
    let pages = pages();
    let mut wrong = Vec::new();
    for (what, holds) in expected {
        let page = &pages
            .iter()
            .find(|(name, _)| name == what)
            .expect("a page")
            .1;
        let got = blocks(page);
        let fine = match (holds, &got) {
            (None, None) => true,
            (Some((count, text)), Some((blocks, read))) => {
                blocks == count && read.contains(text) && (*count > 0 || read.is_empty())
            }
            _ => false,
        };
        if !fine {
            wrong.push(format!("{what}: {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Red (note). `html.rs:160-162` says the scan skips comments and the text of `script` and
/// `style` "as HTML's tokenizer reads them". HTML's tokenizer ends a comment at `--!>` (13.2.5.52,
/// comment end bang state); reads the content of `textarea` and `title` as RCDATA and of `xmp`,
/// `iframe`, `noembed` and `noframes` as RAWTEXT (13.2.6.4.7, "A start tag whose tag name is
/// ..."), and everything after `plaintext` as text; and ends script data only at `</script`
/// followed by a space, `/` or `>` (13.2.5.17). The scan finds a block HTML does not have, or
/// misses one it has. Not on the snapshot (213 blocks byte for byte).
#[test]
fn the_blocks_are_those_of_htmls_tokenizer() {
    let mut cases: Vec<(String, Vec<u8>, usize)> = pages()
        .into_iter()
        .filter_map(|(what, page)| {
            let count = match what {
                "--!> ends a comment" => 1,
                "textarea" | "title" | "</scriptx> does not end a script" => 0,
                _ => return None,
            };
            Some((what.to_string(), page, count))
        })
        .collect();
    for name in ["xmp", "iframe", "noembed", "noframes", "plaintext"] {
        cases.push((
            name.to_string(),
            format!("<{name}><pre class=idl>{A}</pre></{name}>\n").into_bytes(),
            0,
        ));
    }
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(what, page, count)| {
            let got = blocks(page).map(|(blocks, _)| blocks);
            (got != Some(*count)).then(|| format!("{what}: {got:?} blocks, HTML has {count}"))
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The time of the cut of `<a b="` written `count` times: a start tag whose quote no quote ends.
fn time_of_open_quotes(count: usize) -> Duration {
    let page = "<a b=\"".repeat(count);
    let started = Instant::now();
    let cut = cut_idl_blocks(page.as_bytes());
    let took = started.elapsed();
    assert!(cut.is_empty());
    took
}

/// Red (note). `tag_at` (`html.rs:209-285`) looks for the end of a quoted value to the end of
/// the input and answers `None` there; `Tags::next` (`:192-194`) then goes on one byte after the
/// `<`, so each `<` of a run of start tags with an open quote reads the rest of the page: four
/// times the input takes about sixteen times as long (measured in the test build: 500 tags
/// 0.13 s, 2,000 tags 1.4 s, 8,000 tags, 48 KB, 24.7 s). HTML's tokenizer reads such a page
/// once (13.2.5.36, "EOF in tag").
#[test]
fn a_page_of_open_quotes_is_read_in_linear_time() {
    let small = time_of_open_quotes(500);
    let large = time_of_open_quotes(2_000);
    assert!(
        large < Duration::from_millis(100) || large < small * 8,
        "500 tags: {small:?}, 2,000 tags: {large:?}"
    );
}

fn model(idl: &str) -> Model {
    Model::new(read(idl, "small.idl").expect("read")).expect("model")
}

const DOM_EXCEPTION: &str = "interface DOMException { readonly attribute DOMString name; };\n";

fn generated(members: &str, rows: &str) -> Result<String, String> {
    let idl = format!(
        "interface EventTarget {{}};\ninterface Node : EventTarget {{\n{members}\n}};\ninterface Window : EventTarget {{}};\n{DOM_EXCEPTION}"
    );
    let selection = Selection::read(
        &format!("definitions: EventTarget Node Window DOMException\nglobal: Window\n{rows}"),
        "s.txt",
    )
    .map_err(|error| format!("selection: {error}"))?;
    generate(&model(&idl), &selection, &[]).map_err(|error| format!("generate: {error}"))
}

/// The parameter lists of the slices' trait methods: `(name, [parameter names])`.
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

/// Held. The rule of note 8 as fixed: `self`, `Self`, `crate` and `super`, which have no raw
/// form, get a `_` at their end; `-` is written `_`; a keyword with a raw form is raw.
#[test]
fn argument_names_rust_keeps_for_itself_get_a_name() {
    let text = generated(
        "  undefined a(DOMString self);\n  undefined b(DOMString Self);\n\
           undefined c(DOMString crate);\n  undefined d(DOMString super);\n\
           undefined e(DOMString my-arg, DOMString async);",
        "Node a mutation\nNode b mutation\nNode c mutation\nNode d mutation\nNode e mutation\n",
    )
    .expect("generates");
    let methods = trait_methods(&text);
    let params = |name: &str| {
        methods
            .iter()
            .find(|(method, _)| method == name)
            .map(|(_, params)| params.clone())
            .unwrap_or_default()
    };
    assert_eq!(params("node_a"), ["cx", "this", "self_"]);
    assert_eq!(params("node_b"), ["cx", "this", "self_"]);
    assert_eq!(params("node_c"), ["cx", "this", "crate_"]);
    assert_eq!(params("node_d"), ["cx", "this", "super_"]);
    assert_eq!(params("node_e"), ["cx", "this", "my_arg", "r#async"]);
}

/// Red (note). The generator gives every trait method the parameters `cx` and `this`
/// (`generate.rs:1674-1687`) and then one per argument by `identifier` (`:1690-1696`); an
/// argument named `cx` or `this` (Web IDL identifiers) gives a method with two parameters of
/// one name, which rustc refuses (E0415). The duplicate check of fix round 1 compares the
/// arguments with each other only (`:784-797`).
#[test]
fn an_argument_named_like_a_parameter_of_the_method_is_refused_or_renamed() {
    let answer = generated(
        "  undefined a(DOMString cx);\n  undefined b(DOMString this);",
        "Node a mutation\nNode b mutation\n",
    );
    let Ok(text) = answer else {
        return;
    };
    let twice: Vec<String> = trait_methods(&text)
        .into_iter()
        .filter(|(_, params)| {
            let unique: BTreeSet<&String> = params.iter().collect();
            unique.len() != params.len()
        })
        .map(|(name, params)| format!("{name}({})", params.join(", ")))
        .collect();
    assert!(twice.is_empty(), "parameters of one name: {twice:?}");
}

/// Red (note). Note 8's rule covers the arguments and the dictionary fields; the names of
/// members still go through `snake` alone (`generate.rs:933`, `:940`, `:971`, `:1005`, `:1053`,
/// `:1080`), with no check that two give one name: `fooBar` and `foo_bar` (both Web IDL
/// identifiers) give two trait methods `node_foo_bar` and two host functions
/// `operation_node_foo_bar`, which rustc refuses (E0201, E0428). A member named with `-` is
/// refused before (it is checked here too).
#[test]
fn member_names_are_written_as_distinct_rust_identifiers() {
    let mut wrong = Vec::new();
    for (members, rows) in [
        ("  undefined foo-bar();", "Node foo-bar mutation\n"),
        (
            "  undefined fooBar();\n  undefined foo_bar();",
            "Node fooBar mutation\nNode foo_bar mutation\n",
        ),
    ] {
        let Ok(text) = generated(members, rows) else {
            continue;
        };
        let mut seen = BTreeSet::new();
        for line in text.lines() {
            let Some(at) = line.find("fn ") else { continue };
            let Some(name) = line[at + 3..].split(['(', '<']).next() else {
                continue;
            };
            let name = name.trim();
            if !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '#')
            {
                wrong.push(format!("{members}: `fn {name}`"));
            }
            if name.starts_with("node_") && !seen.insert(name.to_string()) {
                wrong.push(format!("{members}: `fn {name}` twice"));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The characters rustc's deny-by-default lint `text_direction_codepoint_in_comment` refuses
/// in a comment.
const BIDI: [char; 9] = [
    '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2066}', '\u{2067}', '\u{2068}',
    '\u{2069}',
];

/// A note (`deviation:`) and a reason (`reason:`) with each of `characters`: the lines of the
/// generated text that hold a bare control character but the line feed, or a bidirectional
/// control character in a comment. A refusal naming the line is the other answer the ruling
/// allows, and counts as right.
fn notes_written_bare(characters: impl IntoIterator<Item = char>) -> Vec<String> {
    let mut wrong = Vec::new();
    for c in characters {
        let rows = format!(
            "Node nodeType reading deviation: a{c}b */ c\nNode foo throws reason: d{c}e */ f\n"
        );
        let Ok(text) = generated(
            "  readonly attribute unsigned short nodeType;\n  undefined foo();",
            &rows,
        ) else {
            continue;
        };
        for line in text.split('\n') {
            let bare_control = line.chars().any(char::is_control);
            let comment = line.trim_start().starts_with("//");
            let bidi = comment && line.chars().any(|c| BIDI.contains(&c));
            if bare_control || bidi {
                wrong.push(format!("U+{:04X}: {line:?}", u32::from(c)));
                break;
            }
        }
    }
    wrong
}

/// Held. Note 8 as fixed: every C0 and C1 control character but the line feed, DEL, U+2028,
/// U+2029, a backslash, a quote and a backtick in a note or a reason, each followed by `*/`:
/// the generated text holds none of the controls bare (`doc_text`, `generate.rs:383-394`).
#[test]
fn a_note_with_a_control_character_writes_it_escaped() {
    let characters = (0u32..0x20)
        .chain([0x7f])
        .chain(0x80..0xa0)
        .filter_map(char::from_u32)
        .filter(|c| *c != '\n')
        .chain(['\u{2028}', '\u{2029}', '\\', '"', '`']);
    let wrong = notes_written_bare(characters);
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Red (note). rustc refuses a bidirectional control character in a comment
/// (`text_direction_codepoint_in_comment`, deny by default); `doc_text` (`generate.rs:383-394`)
/// escapes `char::is_control` only, so a `///` line keeps U+202A to U+202E and U+2066 to U+2069
/// bare and the file does not compile. The reason's string literal is safe (Debug escapes them).
#[test]
fn a_note_with_a_bidirectional_control_writes_a_file_rustc_takes() {
    let wrong = notes_written_bare(BIDI);
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Held. Note 10 as fixed: `[LegacyUnforgeable]` off the global is refused also when the member
/// reaches the interface through a mixin it includes or through a partial interface.
#[test]
fn legacy_unforgeable_through_a_mixin_or_a_partial_is_refused() {
    let cases = [
        "interface mixin M {\n  [LegacyUnforgeable] readonly attribute DOMString y;\n};\nNode includes M;\n",
        "partial interface Node {\n  [LegacyUnforgeable] readonly attribute DOMString y;\n};\n",
    ];
    for extra in cases {
        let idl = format!(
            "interface EventTarget {{}};\ninterface Node : EventTarget {{}};\n\
             interface Window : EventTarget {{}};\n{extra}{DOM_EXCEPTION}"
        );
        let selection = Selection::read(
            "definitions: EventTarget Node Window DOMException\nglobal: Window\nNode y reading\n",
            "s.txt",
        )
        .expect("reads");
        let answer = generate(&model(&idl), &selection, &[]);
        let error = answer.expect_err(extra).to_string();
        assert!(
            error.contains("[LegacyUnforgeable]` off the global"),
            "{extra}: {error}"
        );
    }
}
