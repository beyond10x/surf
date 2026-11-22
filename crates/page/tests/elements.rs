//! The title and the actionable elements of a parsed document. No case uses a table, a
//! template, `svg`, `math`, a `select` inside a `select`, an `a` inside an `a` or misnested
//! formatting elements: the tree for those changes with the later tree stories.

use std::fmt::Write as _;

use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_page::{
    ActionableElement, Kind, actionable_elements, handles, hidden_roots, is_hidden, title,
};

fn parse(input: &str) -> Tree {
    parse_document(input, &ParseOptions::default())
}

/// One actionable element as a line: the local name, `#` and the `id` attribute if it has
/// one, the kind, the target and the value when they are `Some`, then each flag that is true.
fn line(tree: &Tree, element: &ActionableElement) -> String {
    let NodeData::Element {
        local_name,
        attributes,
        ..
    } = tree.data(element.node)
    else {
        panic!("an actionable element that is not an element");
    };
    let mut out = local_name.clone();
    if let Some(id) = attributes.iter().find(|a| a.local_name == "id") {
        write!(out, "#{}", id.value).unwrap();
    }
    write!(out, " {}", element.kind.name()).unwrap();
    if let Some(target) = &element.target {
        write!(out, " target={target:?}").unwrap();
    }
    if let Some(value) = &element.value {
        write!(out, " value={value:?}").unwrap();
    }
    for (set, flag) in [
        (element.hidden, "hidden"),
        (element.disabled, "disabled"),
        (element.checked, "checked"),
        (element.required, "required"),
    ] {
        if set {
            write!(out, " {flag}").unwrap();
        }
    }
    out
}

/// The lines of the actionable elements; each row's `hidden` flag is checked against
/// [`is_hidden`] of its node.
fn listing(tree: &Tree) -> Vec<String> {
    actionable_elements(tree)
        .iter()
        .map(|element| {
            assert_eq!(
                is_hidden(tree, element.node),
                element.hidden,
                "is_hidden and the row's flag for {}",
                line(tree, element)
            );
            line(tree, element)
        })
        .collect()
}

fn list(input: &str) -> Vec<String> {
    listing(&parse(input))
}

/// The one element under the root whose `id` attribute is `id`.
fn by_id(tree: &Tree, id: &str) -> NodeId {
    let found: Vec<NodeId> = tree
        .descendants(tree.root())
        .into_iter()
        .filter(|&node| match tree.data(node) {
            NodeData::Element { attributes, .. } => attributes
                .iter()
                .any(|a| a.local_name == "id" && a.value == id),
            _ => false,
        })
        .collect();
    assert_eq!(found.len(), 1, "one element with id {id}");
    found[0]
}

fn local_name(tree: &Tree, node: NodeId) -> &str {
    match tree.data(node) {
        NodeData::Element { local_name, .. } => local_name,
        _ => panic!("not an element"),
    }
}

#[test]
fn title_is_the_first_title_with_whitespace_collapsed() {
    let tree = parse("<title>\n  a \t b&amp;c\n </title><p>x<title>second</title>");
    assert_eq!(title(&tree), "a b&c");
    let tree = parse("<p>x<title> only \n in body </title><title>later</title>");
    assert_eq!(title(&tree), "only in body");
}

#[test]
fn a_document_without_title_has_an_empty_title() {
    assert_eq!(title(&parse("<p>no title</p>")), "");
    assert_eq!(title(&parse("")), "");
    assert_eq!(title(&parse("<title></title><title>second</title>")), "");
}

#[test]
fn links_are_a_and_area_with_href() {
    assert_eq!(
        list(
            "<a id=a href=\"/x?y=1#z\">x</a><a id=e href=\"\">e</a>\
             <a id=s href=\" spaced \">s</a><map name=m><area id=r href=r.html></map>"
        ),
        [
            "a#a link target=\"/x?y=1#z\"",
            "a#e link target=\"\"",
            "a#s link target=\" spaced \"",
            "area#r link target=\"r.html\"",
        ]
    );
}

#[test]
fn an_a_without_href_is_not_a_link() {
    assert_eq!(
        list(
            "<a id=n name=x>n</a><a id=y href=y>y</a><map><area id=m alt=m></map>\
             <link href=l.css rel=stylesheet>"
        ),
        ["a#y link target=\"y\""]
    );
}

#[test]
fn input_type_states_map_to_kinds() {
    let keywords = [
        "text",
        "search",
        "tel",
        "url",
        "email",
        "password",
        "date",
        "month",
        "week",
        "time",
        "datetime-local",
        "number",
        "range",
        "color",
        "checkbox",
        "radio",
        "file",
        "submit",
        "image",
        "reset",
        "button",
    ];
    let mut input = String::new();
    for keyword in keywords {
        write!(input, "<input type={keyword} id={keyword}>").unwrap();
    }
    input.push_str(
        "<input id=missing><input type=foo id=unknown><input type=PassWord id=upper>\
         <button id=b type=reset>b</button><select id=s><option>o</select>\
         <textarea id=t>x</textarea>",
    );
    assert_eq!(
        list(&input),
        [
            "input#text text value=\"\"",
            "input#search text value=\"\"",
            "input#tel text value=\"\"",
            "input#url text value=\"\"",
            "input#email text value=\"\"",
            "input#password password value=\"\"",
            "input#date text value=\"\"",
            "input#month text value=\"\"",
            "input#week text value=\"\"",
            "input#time text value=\"\"",
            "input#datetime-local text value=\"\"",
            "input#number text value=\"\"",
            "input#range text value=\"\"",
            "input#color text value=\"\"",
            "input#checkbox checkbox",
            "input#radio radio",
            "input#file file value=\"\"",
            "input#submit button",
            "input#image button",
            "input#reset button",
            "input#button button",
            "input#missing text value=\"\"",
            "input#unknown text value=\"\"",
            "input#upper password value=\"\"",
            "button#b button",
            "select#s select",
            "textarea#t textarea value=\"x\"",
        ]
    );
    assert_eq!(
        [
            Kind::Link,
            Kind::Button,
            Kind::Text,
            Kind::Password,
            Kind::Checkbox,
            Kind::Radio,
            Kind::File,
            Kind::Select,
            Kind::Textarea,
        ]
        .map(Kind::name),
        [
            "link", "button", "text", "password", "checkbox", "radio", "file", "select", "textarea"
        ]
    );
}

#[test]
fn a_hidden_input_is_not_listed() {
    let tree = parse(
        "<input type=hidden id=h value=v><input type=HIDDEN id=u>\
         <input type=\" hidden\" id=w><input id=t>",
    );
    assert_eq!(
        listing(&tree),
        ["input#w text value=\"\"", "input#t text value=\"\""]
    );
    assert!(is_hidden(&tree, by_id(&tree, "h")));
    assert!(is_hidden(&tree, by_id(&tree, "u")));
    assert!(!is_hidden(&tree, by_id(&tree, "w")));
}

#[test]
fn textarea_value_is_its_default_value() {
    assert_eq!(
        list(
            "<textarea id=a>\nhello\n  world &amp; more</textarea><textarea id=b></textarea>\
             <textarea id=c>\n\nx</textarea><textarea id=d><b>not a tag</b></textarea>"
        ),
        [
            "textarea#a textarea value=\"hello\\n  world & more\"",
            "textarea#b textarea value=\"\"",
            "textarea#c textarea value=\"\\nx\"",
            "textarea#d textarea value=\"<b>not a tag</b>\"",
        ]
    );
}

#[test]
fn text_control_values_are_their_sanitized_default_values() {
    assert_eq!(
        list(
            "<input id=t value=\"a&#10;b&#13;c \">\
             <input id=n>\
             <input type=url id=u value=\"  http://e.org/a&#10;b  \">\
             <input type=file id=f value=x.txt>\
             <input type=password id=p value=\"p&#10;w \">\
             <input type=search id=s value=\" s&#13;&#10;\">\
             <input type=tel id=tel value=\"1&#10;2\">\
             <input type=email id=e value=\" a@b.c&#10; \">\
             <input type=email multiple id=m value=\" a@b.c , d@e.f ,\">\
             <input type=number id=num value=\" 5&#10;\">"
        ),
        [
            "input#t text value=\"abc \"",
            "input#n text value=\"\"",
            "input#u text value=\"http://e.org/ab\"",
            "input#f file value=\"\"",
            "input#p password value=\"pw \"",
            "input#s text value=\" s\"",
            "input#tel text value=\"12\"",
            "input#e text value=\"a@b.c\"",
            "input#m text value=\"a@b.c,d@e.f\"",
            "input#num text value=\" 5\\n\"",
        ]
    );
}

#[test]
fn a_disabled_attribute_disables_a_control() {
    assert_eq!(
        list(
            "<button id=b disabled>b</button><input id=i disabled=false>\
             <select id=s disabled><option>o</select><textarea id=t disabled></textarea>\
             <input id=e><a href=x id=a disabled>a</a>"
        ),
        [
            "button#b button disabled",
            "input#i text value=\"\" disabled",
            "select#s select disabled",
            "textarea#t textarea value=\"\" disabled",
            "input#e text value=\"\"",
            "a#a link target=\"x\"",
        ]
    );
}

#[test]
fn a_control_in_a_disabled_fieldset_is_disabled() {
    assert_eq!(
        list(
            "<fieldset disabled><input id=a><div><button id=b>b</button></div>\
             <a href=x id=l>l</a><fieldset><input id=n></fieldset></fieldset>\
             <fieldset><input id=c></fieldset><input id=d>"
        ),
        [
            "input#a text value=\"\" disabled",
            "button#b button disabled",
            "a#l link target=\"x\"",
            "input#n text value=\"\" disabled",
            "input#c text value=\"\"",
            "input#d text value=\"\"",
        ]
    );
}

#[test]
fn a_control_in_the_first_legend_is_not_disabled() {
    assert_eq!(
        list(
            "<fieldset disabled><legend><input id=a></legend><legend><input id=b></legend>\
             <input id=c><div><legend><input id=d></legend></div></fieldset>\
             <fieldset disabled><input id=h><legend><input id=i></legend></fieldset>"
        ),
        [
            "input#a text value=\"\"",
            "input#b text value=\"\" disabled",
            "input#c text value=\"\" disabled",
            "input#d text value=\"\" disabled",
            "input#h text value=\"\" disabled",
            "input#i text value=\"\"",
        ]
    );
    assert_eq!(
        list(
            "<fieldset disabled><legend><fieldset disabled><legend><input id=e></legend>\
             <input id=f></fieldset></legend></fieldset>\
             <fieldset disabled><fieldset disabled><legend><input id=g></legend></fieldset>\
             </fieldset>"
        ),
        [
            "input#e text value=\"\"",
            "input#f text value=\"\" disabled",
            "input#g text value=\"\" disabled",
        ]
    );
}

#[test]
fn checked_is_the_default_checkedness_of_a_checkbox_and_a_radio_button() {
    assert_eq!(
        list(
            "<input type=checkbox id=a checked><input type=checkbox id=b>\
             <input type=CheckBox id=u checked>\
             <input type=radio name=g id=c checked><input type=radio name=g id=d checked>\
             <input type=radio id=e><input id=f checked><input type=submit id=s checked>"
        ),
        [
            "input#a checkbox checked",
            "input#b checkbox",
            "input#u checkbox checked",
            "input#c radio checked",
            "input#d radio checked",
            "input#e radio",
            "input#f text value=\"\"",
            "input#s button",
        ]
    );
}

#[test]
fn required_applies_only_where_the_standard_says() {
    assert_eq!(
        list(
            "<input id=t required><input type=checkbox id=c required>\
             <input type=range id=r required><input type=submit id=s required>\
             <button id=b required>b</button><input type=color id=co required>\
             <input type=file id=f required><select id=se required><option>o</select>\
             <textarea id=ta required></textarea><input type=number id=n required>\
             <input type=radio id=ra required><input type=image id=im required>\
             <a href=x id=l required>l</a>"
        ),
        [
            "input#t text value=\"\" required",
            "input#c checkbox required",
            "input#r text value=\"\"",
            "input#s button",
            "button#b button",
            "input#co text value=\"\"",
            "input#f file value=\"\" required",
            "select#se select required",
            "textarea#ta textarea value=\"\" required",
            "input#n text value=\"\" required",
            "input#ra radio required",
            "input#im button",
            "a#l link target=\"x\"",
        ]
    );
}

#[test]
fn elements_come_in_document_order() {
    let tree = parse(
        "<a href=1 id=l1>one</a><input id=i1><p><button id=b1>b</button>\
         <select id=s1><option>o</select></p><textarea id=t1>t</textarea>\
         <div><a href=2 id=l2>two</a></div><input type=checkbox id=c1>",
    );
    assert_eq!(
        listing(&tree),
        [
            "a#l1 link target=\"1\"",
            "input#i1 text value=\"\"",
            "button#b1 button",
            "select#s1 select",
            "textarea#t1 textarea value=\"t\"",
            "a#l2 link target=\"2\"",
            "input#c1 checkbox",
        ]
    );
    let order = tree.descendants(tree.root());
    let nodes: Vec<NodeId> = actionable_elements(&tree).iter().map(|e| e.node).collect();
    let expected: Vec<NodeId> = order
        .iter()
        .copied()
        .filter(|&n| nodes.contains(&n))
        .collect();
    assert_eq!(nodes, expected);
}

#[test]
fn hidden_attribute_on_an_ancestor_hides_an_element() {
    let tree = parse(
        "<div hidden id=hd><p><a href=a id=a>a</a></p></div>\
         <div hidden=false><input id=b></div><section><input id=c></section>\
         <embed hidden id=em>",
    );
    assert_eq!(
        listing(&tree),
        [
            "a#a link target=\"a\" hidden",
            "input#b text value=\"\" hidden",
            "input#c text value=\"\"",
        ]
    );
    let link = by_id(&tree, "a");
    let text = tree.children(link)[0];
    assert!(matches!(tree.data(text), NodeData::Text(t) if t == "a"));
    assert!(is_hidden(&tree, text));
    assert!(is_hidden(&tree, by_id(&tree, "hd")));
    assert!(!is_hidden(&tree, by_id(&tree, "em")));
    assert!(!is_hidden(&tree, tree.root()));
}

#[test]
fn hidden_until_found_hides_an_element() {
    assert_eq!(
        list(
            "<div hidden=until-found><input id=a></div>\
             <div hidden=UNTIL-FOUND><button id=b>b</button></div><input id=c>"
        ),
        [
            "input#a text value=\"\" hidden",
            "button#b button hidden",
            "input#c text value=\"\"",
        ]
    );
}

#[test]
fn aria_hidden_true_on_an_ancestor_hides_an_element() {
    assert_eq!(
        list(
            "<div aria-hidden=true><p><button id=a>a</button></div>\
             <div aria-hidden=false><button id=b>b</button></div>\
             <div aria-hidden=\"\"><button id=c>c</button></div>\
             <a href=d id=d aria-hidden=true>d</a>\
             <div aria-hidden=TRUE><button id=e>e</button></div>"
        ),
        [
            "button#a button hidden",
            "button#b button",
            "button#c button",
            "a#d link target=\"d\" hidden",
            "button#e button hidden",
        ]
    );
    // WAI-ARIA 1.3, `aria-hidden`: user agents must not expose the hidden state when it is
    // specified on the root element or on the element that contains the document's contents.
    let tree = parse("<html aria-hidden=true><body aria-hidden=true><a href=x id=x>x</a>");
    assert_eq!(listing(&tree), ["a#x link target=\"x\""]);
    let body = tree.parent(by_id(&tree, "x")).unwrap();
    assert_eq!(local_name(&tree, body), "body");
    assert!(!is_hidden(&tree, body));
    assert!(!is_hidden(&tree, tree.parent(body).unwrap()));
    // Only those two: a child of the body element is read.
    let tree = parse(
        "<!--c--><html aria-hidden=true><!--c--><body aria-hidden=true>\
         <div aria-hidden=true id=dv><a href=y id=y>y</a></div>",
    );
    assert_eq!(listing(&tree), ["a#y link target=\"y\" hidden"]);
    assert!(is_hidden(&tree, by_id(&tree, "dv")));
    assert!(is_hidden(&tree, tree.children(by_id(&tree, "y"))[0]));
}

/// WAI-ARIA 1.3 6.3.1: each value is a keyword; appendix A maps true/false values in HTML to
/// keyword and enumerated attributes (HTML 2.3.3: an ASCII case-insensitive match, the value not
/// stripped); ARIA in HTML, "Case requirements": browsers treat `aria-*` values ASCII
/// case-insensitively.
#[test]
fn aria_hidden_true_is_matched_ascii_case_insensitively() {
    let tree = parse(
        "<div id=u aria-hidden=TRUE><button id=a>a</button></div>\
         <div id=m aria-hidden=True><a href=x id=b>b</a></div>\
         <div aria-hidden=\" true\"><button id=c>c</button></div>\
         <div aria-hidden=\"true \"><button id=d>d</button></div>\
         <div aria-hidden=1><button id=e>e</button></div>\
         <div aria-hidden=yes><button id=f>f</button></div>\
         <div aria-hidden><button id=g>g</button></div>\
         <div aria-hidden=truE><button id=h>h</button></div>",
    );
    assert_eq!(
        listing(&tree),
        [
            "button#a button hidden",
            "a#b link target=\"x\" hidden",
            "button#c button",
            "button#d button",
            "button#e button",
            "button#f button",
            "button#g button",
            "button#h button hidden",
        ]
    );
    assert!(is_hidden(&tree, by_id(&tree, "u")));
    assert!(is_hidden(&tree, by_id(&tree, "m")));
}

/// HTML 15.3.3 "Flow content": `dialog:not([open]) { display: none; }`.
#[test]
fn a_closed_dialog_hides_its_content() {
    let tree = parse(
        "<dialog id=d1><button id=a>a</button>t</dialog>\
         <dialog open><button id=b>b</button></dialog>\
         <dialog open=\"\"><button id=c>c</button></dialog>\
         <dialog open=false><button id=d>d</button></dialog>\
         <dialog><dialog open id=d2><button id=e>e</button></dialog></dialog>\
         <dialog open><dialog id=d3><button id=f>f</button></dialog></dialog>\
         <dialog open><dialog open><button id=g>g</button></dialog></dialog>\
         <button id=h>h</button>",
    );
    assert_eq!(
        listing(&tree),
        [
            "button#a button hidden",
            "button#b button",
            "button#c button",
            "button#d button",
            "button#e button hidden",
            "button#f button hidden",
            "button#g button",
            "button#h button",
        ]
    );
    let d1 = by_id(&tree, "d1");
    assert!(is_hidden(&tree, d1));
    assert!(is_hidden(&tree, tree.children(d1)[1]));
    assert!(is_hidden(&tree, by_id(&tree, "d2")));
    assert!(is_hidden(&tree, by_id(&tree, "d3")));
    assert!(!is_hidden(&tree, tree.parent(by_id(&tree, "d3")).unwrap()));
}

/// HTML 15.3.3 "Flow content": `[popover]:not(:popover-open):not(dialog[open]) { display: none;
/// }`. surf runs no script and no user action, so no popover is open: an element with a
/// `popover` attribute, whatever its value, is hidden, but an open `dialog`.
#[test]
fn a_popover_is_hidden() {
    let tree = parse(
        "<div popover id=p1><button id=a>a</button></div>\
         <div popover=manual><button id=b>b</button></div>\
         <div popover=nonsense><button id=c>c</button></div>\
         <dialog open popover id=d1><button id=d>d</button></dialog>\
         <dialog popover><button id=e>e</button></dialog>\
         <button popover id=f>f</button>\
         <button id=g>g</button>",
    );
    assert_eq!(
        listing(&tree),
        [
            "button#a button hidden",
            "button#b button hidden",
            "button#c button hidden",
            "button#d button",
            "button#e button hidden",
            "button#f button hidden",
            "button#g button",
        ]
    );
    assert!(is_hidden(&tree, by_id(&tree, "p1")));
    assert!(!is_hidden(&tree, by_id(&tree, "d1")));
    let hidden = surf_page::hidden_roots(&tree);
    assert!(hidden.contains(&by_id(&tree, "p1")));
    assert!(hidden.contains(&by_id(&tree, "f")));
    assert!(!hidden.contains(&by_id(&tree, "d1")));
}

/// surf's reading, not the standard's rendering: the copy of the selected option that the
/// parser puts into each `selectedcontent` of a `select` is hidden and lists no row; the option
/// lists its own. A `selectedcontent` the parser fills nothing into (outside a `select`) is not
/// hidden.
#[test]
fn the_copy_in_a_selectedcontent_is_hidden_and_lists_nothing() {
    let tree = parse(
        "<label for=s>Size</label>\
         <select id=s><button><selectedcontent id=c1></selectedcontent>\
         <selectedcontent id=c2></selectedcontent></button>\
         <option><a href=/a>a</a></option></select>\
         <div><selectedcontent id=c3><a href=/b id=b>b</a></selectedcontent></div>",
    );
    assert_eq!(
        listing(&tree),
        [
            "select#s select",
            "button button",
            "a link target=\"/a\"",
            "a#b link target=\"/b\""
        ]
    );
    let hidden = surf_page::hidden_roots(&tree);
    for id in ["c1", "c2"] {
        let copy = by_id(&tree, id);
        assert!(is_hidden(&tree, copy) && hidden.contains(&copy), "{id}");
        let link = tree.children(copy)[0];
        assert!(is_hidden(&tree, link) && !hidden.contains(&link), "{id}");
    }
    for id in ["b", "c3"] {
        assert!(!is_hidden(&tree, by_id(&tree, id)), "{id}");
    }
    assert!(!hidden.contains(&by_id(&tree, "c3")));
}

/// HTML 15.5.5 "The details and summary elements": the second slot of the shadow tree takes
/// every child but the first `summary` element child and has `content-visibility: hidden` while
/// the `details` element has no `open` attribute.
#[test]
fn the_content_of_a_closed_details_is_hidden_but_its_first_summary() {
    let tree = parse(
        "<details id=d1><summary id=s1>s</summary><input id=a></details>\
         <details open><summary>s</summary><input id=b></details>\
         <details id=d2><summary><input id=c></summary><summary><input id=d></summary>x</details>\
         <details><input id=e><summary><input id=f></summary></details>\
         <details><div><summary><input id=g></summary></div></details>\
         <details open><summary>s</summary>\
         <details><summary><input id=h></summary><input id=i></details></details>\
         <details><summary>s</summary>\
         <details open><summary><input id=j></summary><input id=k></details></details>\
         <input id=l>",
    );
    assert_eq!(
        listing(&tree),
        [
            "input#a text value=\"\" hidden",
            "input#b text value=\"\"",
            "input#c text value=\"\"",
            "input#d text value=\"\" hidden",
            "input#e text value=\"\" hidden",
            "input#f text value=\"\"",
            "input#g text value=\"\" hidden",
            "input#h text value=\"\"",
            "input#i text value=\"\" hidden",
            "input#j text value=\"\" hidden",
            "input#k text value=\"\" hidden",
            "input#l text value=\"\"",
        ]
    );
    let d1 = by_id(&tree, "d1");
    let s1 = by_id(&tree, "s1");
    assert!(!is_hidden(&tree, d1));
    assert!(!is_hidden(&tree, s1));
    assert!(!is_hidden(&tree, tree.children(s1)[0]));
    let d2 = by_id(&tree, "d2");
    let text = *tree.children(d2).last().unwrap();
    assert!(matches!(tree.data(text), NodeData::Text(t) if t == "x"));
    assert!(is_hidden(&tree, text));
}

#[test]
fn an_element_inside_datalist_is_hidden() {
    let tree =
        parse("<datalist id=dl><input id=a><select id=s><option>o</select></datalist><input id=b>");
    assert_eq!(
        listing(&tree),
        [
            "input#a text value=\"\" hidden",
            "select#s select hidden",
            "input#b text value=\"\"",
        ]
    );
    assert!(is_hidden(&tree, by_id(&tree, "dl")));
}

#[test]
fn an_area_link_is_not_hidden_by_its_own_element() {
    let tree = parse(
        "<map name=m><area href=a id=a></map><img usemap=#m>\
         <datalist><map><area href=b id=b></map></datalist>",
    );
    assert_eq!(
        listing(&tree),
        [
            "area#a link target=\"a\"",
            "area#b link target=\"b\" hidden"
        ]
    );
    assert!(!is_hidden(&tree, by_id(&tree, "a")));
    assert!(is_hidden(&tree, by_id(&tree, "b")));
}

#[test]
fn noscript_content_is_not_hidden() {
    let tree = parse("<p><noscript id=ns><a href=n id=n>n</a></noscript>");
    assert_eq!(listing(&tree), ["a#n link target=\"n\""]);
    assert!(!is_hidden(&tree, by_id(&tree, "ns")));
    // A `noscript` in head: with scripting disabled the parser closes it and moves the `a` to
    // body.
    let tree = parse("<head><noscript><a href=h id=h>h</a></noscript></head>");
    assert_eq!(listing(&tree), ["a#h link target=\"h\""]);
    let parent = tree.parent(by_id(&tree, "h")).unwrap();
    assert_eq!(local_name(&tree, parent), "body");
}

/// The stack of the thread the deep walks run in: 256 KiB.
const DEEP_STACK: usize = 256 << 10;

/// The depth of the deep trees: 4,000. A walk of `actionable_elements` that recursed over the
/// depth was measured at 736 bytes a frame (this crate at opt-level 0, 2026-11-22), so 4,000
/// levels need 2.9 MiB, 11 times the stack; 1,425 would be 4 times. Any frame over 65 bytes
/// overflows it.
const DEEP: usize = 4_000;

#[test]
fn a_deep_tree_is_read_without_recursion() {
    // In a thread of `DEEP_STACK` bytes: a walk over the depth overflows it and aborts the test.
    std::thread::Builder::new()
        .stack_size(DEEP_STACK)
        .spawn(deep_trees_are_read)
        .expect("a thread")
        .join()
        .expect("the walks end");
}

fn deep_trees_are_read() {
    let mut input = String::from("<div hidden>");
    input.push_str(&"<span>".repeat(DEEP));
    input.push_str("<a href=deep id=deep>x</a>");
    let deep = ParseOptions {
        limits: surf_html::limits::Limits {
            max_depth: DEEP + 10,
            ..surf_html::limits::Limits::default()
        },
        ..ParseOptions::default()
    };
    let tree = parse_document(&input, &deep);
    assert_eq!(listing(&tree), ["a#deep link target=\"deep\" hidden"]);
    let link = by_id(&tree, "deep");
    assert!(is_hidden(&tree, tree.children(link)[0]));
    assert_eq!(title(&tree), "");

    // `DEEP` nested `span` elements, an `input` in each: `DEEP` rows, the last at depth `DEEP`.
    let tree = parse_document(&"<span><input>".repeat(DEEP), &deep);
    let rows = actionable_elements(&tree);
    assert_eq!(rows.len(), DEEP);
    assert!(
        rows.iter()
            .all(|row| row.kind == Kind::Text && !row.hidden && !row.disabled)
    );
    let deepest = rows.last().unwrap().node;
    let mut spans = 0;
    let mut current = tree.parent(deepest);
    while let Some(node) = current {
        if matches!(tree.data(node), NodeData::Element { local_name, .. } if local_name == "span") {
            spans += 1;
        }
        current = tree.parent(node);
    }
    assert_eq!(spans, DEEP);
    assert!(!is_hidden(&tree, deepest));
    assert_eq!(title(&tree), "");
}

/// `story:page-text-outline` case 2: `l<n>` for a link, `b<n>` for a button, `f<n>` for the
/// other seven kinds, per letter in document order, hidden elements included.
#[test]
fn handles_are_per_letter_in_document_order() {
    let tree = parse(
        "<a href=1>a</a><button>b</button><input><a href=2>c</a><select></select>\
         <a hidden href=3>d</a>",
    );
    let names: Vec<String> = handles(&tree).into_iter().map(|(_, name)| name).collect();
    assert_eq!(names, ["l1", "b1", "f1", "l2", "f2", "l3"]);
    let rows = actionable_elements(&tree);
    let nodes: Vec<NodeId> = handles(&tree).into_iter().map(|(node, _)| node).collect();
    assert_eq!(
        nodes,
        rows.iter().map(|row| row.node).collect::<Vec<_>>(),
        "one handle per row, in the rows' order"
    );
}

/// `hidden_roots` holds the nodes `is_hidden` calls hidden whose parent it does not: a node is
/// hidden exactly when it or an ancestor is in the set.
#[test]
fn hidden_roots_agree_with_is_hidden() {
    for input in [
        "<p>a</p><p hidden>h<b>i</b></p><div aria-hidden=\"true\"><p>h2</p></div>\
         <script>s</script><template><p>t</p></template><input type=hidden>\
         <dialog><p>d</p></dialog><dialog open><p>o</p></dialog>",
        "<details id=d1><summary id=s1>s</summary><input id=a></details>\
         <details><input id=e><summary><input id=f></summary>x</details>\
         <details open><summary>s</summary>\
         <details><summary><input id=h></summary><input id=i></details></details>",
        "<html aria-hidden=true><body aria-hidden=TRUE><p>x</p><div aria-hidden=true>y</div>",
    ] {
        let tree = parse(input);
        let roots = hidden_roots(&tree);
        let mut pending = vec![tree.root()];
        let mut hidden = 0;
        while let Some(node) = pending.pop() {
            let mut in_roots = false;
            let mut current = Some(node);
            while let Some(at) = current {
                in_roots |= roots.contains(&at);
                current = tree.parent(at);
            }
            assert_eq!(in_roots, is_hidden(&tree, node), "{input}: {node:?}");
            if let Some(parent) = tree.parent(node) {
                assert_eq!(
                    roots.contains(&node),
                    is_hidden(&tree, node) && !is_hidden(&tree, parent),
                    "{input}: {node:?} is a root exactly when its parent is not hidden"
                );
            }
            hidden += usize::from(in_roots);
            pending.extend(tree.children(node).iter().rev());
        }
        assert!(hidden > 0, "{input}: some node is hidden");
    }
}
