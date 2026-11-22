//! The label of each actionable element: its accessible name (Accessible Name and Description
//! Computation 1.2, 4.3.2, with HTML-AAM 4.1). Like `tests/elements.rs`, no case uses a table, a
//! template, `svg`, `math` or misnested formatting, and no tree is deeper than 512 elements but
//! the last case's.

use std::time::{Duration, Instant};

use surf_html::{NodeData, ParseOptions, Tree, parse_document};
use surf_page::{
    ActionableElement, LABEL_BYTES_MAX, LABEL_WORK_MAX, actionable_elements, attribute_visits,
};

fn parse(input: &str) -> Tree {
    parse_document(input, &ParseOptions::default())
}

/// One row as `id="label"`, the `id` attribute or `-` when the element has none, then ` cut`
/// when `label_cut` is set.
fn line(tree: &Tree, row: &ActionableElement) -> String {
    let NodeData::Element { attributes, .. } = tree.data(row.node) else {
        panic!("an actionable element that is not an element");
    };
    let id = attributes
        .iter()
        .find(|a| a.local_name == "id")
        .map_or("-", |a| a.value.as_str());
    let cut = if row.label_cut { " cut" } else { "" };
    format!("{id}={:?}{cut}", row.label)
}

fn names(input: &str) -> Vec<String> {
    let tree = parse(input);
    actionable_elements(&tree)
        .iter()
        .map(|row| line(&tree, row))
        .collect()
}

// One case per source.

#[test]
fn aria_labelledby_names_from_the_referenced_elements() {
    assert_eq!(
        names(
            "<span id=a>Alpha</span><span id=b>Beta</span>\
             <button id=x aria-labelledby=\"a nothing b\">ignored</button>\
             <input id=y aria-labelledby=\" b&#9;a \"><input id=z aria-labelledby=nothing title=T>"
        ),
        ["x=\"Alpha Beta\"", "y=\"Beta Alpha\"", "z=\"T\""]
    );
}

#[test]
fn aria_label_names_the_element() {
    assert_eq!(
        names(
            "<a href=x id=l aria-label=\"Close dialog\">X</a>\
             <button id=b aria-label=\" &#10; \">Text</button><input id=i aria-label=Query>"
        ),
        ["l=\"Close dialog\"", "b=\"Text\"", "i=\"Query\""]
    );
}

#[test]
fn label_for_names_a_control() {
    assert_eq!(
        names(
            "<label for=n>Name</label><input id=n>\
             <label for=s>Pick</label><select id=s><option>o</select>\
             <label for=t>Notes</label><textarea id=t>x</textarea>\
             <label for=b>Press</label><button id=b>Text</button>"
        ),
        ["n=\"Name\"", "s=\"Pick\"", "t=\"Notes\"", "b=\"Press\""]
    );
}

#[test]
fn wrapping_label_names_a_control() {
    assert_eq!(
        names(
            "<label>Email <input id=e type=email></label>\
             <label><span>Agree</span><input type=checkbox id=c></label>\
             <label>First <input id=f> second <input id=g></label>"
        ),
        ["e=\"Email\"", "c=\"Agree\"", "f=\"First second\"", "g=\"\""]
    );
}

#[test]
fn every_label_of_a_control_is_joined_in_tree_order() {
    assert_eq!(
        names(
            "<label for=p>First</label><label>Second <input id=p></label>\
             <label for=p>Third</label>"
        ),
        ["p=\"First Second Third\""]
    );
}

#[test]
fn button_input_value_names_it() {
    assert_eq!(
        names(
            "<input type=submit id=s value=Send title=T><input type=reset id=r value=Clear>\
             <input type=button id=b value=Go><input type=BUTTON id=u value=\" Up \">"
        ),
        ["s=\"Send\"", "r=\"Clear\"", "b=\"Go\"", "u=\"Up\""]
    );
}

#[test]
fn submit_button_without_value_is_named_submit() {
    assert_eq!(
        names(
            "<input type=submit id=s><input type=submit id=e value=\"\">\
             <input type=submit id=t value=\"\" title=T><input type=submit id=w title=W>"
        ),
        ["s=\"Submit\"", "e=\"\"", "t=\"T\"", "w=\"Submit\""]
    );
}

#[test]
fn reset_button_without_value_is_named_reset() {
    assert_eq!(
        names("<input type=reset id=r><input type=button id=b><input type=button id=t title=T>"),
        ["r=\"Reset\"", "b=\"\"", "t=\"T\""]
    );
}

#[test]
fn image_button_is_named_by_alt_then_title_then_submit() {
    assert_eq!(
        names(
            "<input type=image id=a alt=Search title=T><input type=image id=t alt=\" \" title=Find>\
             <input type=image id=s><input type=image id=e alt=\"\" title=\"\">"
        ),
        ["a=\"Search\"", "t=\"Find\"", "s=\"Submit\"", "e=\"Submit\""]
    );
}

#[test]
fn area_alt_names_the_link() {
    assert_eq!(
        names(
            "<map name=m><area href=a id=a alt=Home title=T><area href=b id=b title=Back>\
             <area href=c id=c alt=\" \" title=C><area href=d id=d></map>"
        ),
        ["a=\"Home\"", "b=\"Back\"", "c=\"C\"", "d=\"\""]
    );
}

#[test]
fn img_alt_inside_a_link_names_it() {
    assert_eq!(
        names(
            "<a href=x id=a><img src=i.png alt=Logo></a>\
             <a href=y id=b><img src=i.png title=Tip></a>\
             <a href=z id=c title=T><img src=i.png alt=\"\" title=Ignored></a>\
             <a href=w id=d>Go <img src=i.png alt=home> now</a>"
        ),
        ["a=\"Logo\"", "b=\"Tip\"", "c=\"T\"", "d=\"Go home now\""]
    );
}

#[test]
fn link_text_names_the_link() {
    assert_eq!(
        names("<a href=x id=a title=T>Read <b>more</b></a><a href=y id=b><span>in</span>side</a>"),
        ["a=\"Read more\"", "b=\"inside\""]
    );
}

#[test]
fn button_text_names_the_button() {
    assert_eq!(
        names(
            "<button id=b title=T>Save <span>now</span></button>\
             <button id=n><span title=Inner></span></button>"
        ),
        ["b=\"Save now\"", "n=\"Inner\""]
    );
}

#[test]
fn title_names_an_element_without_another_name() {
    assert_eq!(
        names(
            "<a href=x id=a title=Tip></a><a href=y id=w title=W> </a>\
             <button id=b title=B></button><select id=s title=S><option>o</select>\
             <input type=checkbox id=c title=C><input id=t title=T placeholder=P>\
             <textarea id=ta title=TA></textarea><map><area href=r id=r title=R></map>"
        ),
        [
            "a=\"Tip\"",
            "w=\"W\"",
            "b=\"B\"",
            "s=\"S\"",
            "c=\"C\"",
            "t=\"T\"",
            "ta=\"TA\"",
            "r=\"R\"",
        ]
    );
}

#[test]
fn placeholder_names_a_text_control_without_another_name() {
    assert_eq!(
        names(
            "<input id=p placeholder=Search><input id=q aria-placeholder=Q>\
             <input id=b placeholder=P aria-placeholder=A><input id=e placeholder=\"\" aria-placeholder=A>\
             <textarea id=t placeholder=TA></textarea><input type=number id=n placeholder=N>\
             <input type=password id=w placeholder=W>\
             <input type=date id=d placeholder=D><select id=s placeholder=S><option>o</select>"
        ),
        [
            "p=\"Search\"",
            "q=\"Q\"",
            "b=\"P\"",
            "e=\"A\"",
            "t=\"TA\"",
            "n=\"N\"",
            "w=\"W\"",
            "d=\"\"",
            "s=\"\"",
        ]
    );
}

// The rules.

#[test]
fn aria_label_wins_over_label_element() {
    assert_eq!(
        names(
            "<label for=i>Label</label><input id=i aria-label=Aria>\
             <span id=s>Referenced</span><label for=j>Label</label>\
             <input id=j aria-label=Aria aria-labelledby=s>"
        ),
        ["i=\"Aria\"", "j=\"Referenced\""]
    );
}

/// Accname 4.3.2, example 3.
#[test]
fn aria_labelledby_is_not_followed_twice() {
    assert_eq!(
        names(
            "<button id=el1 aria-labelledby=el3></button><button id=el2 aria-labelledby=el1></button>\
             <span id=el3> hello </span>"
        ),
        ["el1=\"hello\"", "el2=\"\""]
    );
}

/// Accname 4.3.2, example 5.
#[test]
fn aria_labelledby_naming_the_element_itself_uses_its_aria_label() {
    assert_eq!(
        names(
            "<h1>Files</h1><ul>\
             <li><a id=file_row1 href=./files/Documentation.pdf>Documentation.pdf</a>\
             <button id=del_row1 aria-label=Delete aria-labelledby=\"del_row1 file_row1\"></button>\
             <li><a id=file_row2 href=./files/HolidayLetter.pdf>HolidayLetter.pdf</a>\
             <button id=del_row2 aria-label=Delete aria-labelledby=\"del_row2 file_row2\"></button>\
             </ul>"
        ),
        [
            "file_row1=\"Documentation.pdf\"",
            "del_row1=\"Delete Documentation.pdf\"",
            "file_row2=\"HolidayLetter.pdf\"",
            "del_row2=\"Delete HolidayLetter.pdf\"",
        ]
    );
}

#[test]
fn a_label_with_for_labels_only_its_target() {
    assert_eq!(
        names("<label for=other>Other <input id=w></label><input id=other>"),
        ["w=\"\"", "other=\"Other\""]
    );
    // The first element with the ID is not labelable: the label labels nothing, and not its
    // descendant either.
    assert_eq!(
        names("<div id=d></div><label for=d>L <input id=x></label>"),
        ["x=\"\""]
    );
    assert_eq!(
        names("<span id=dup></span><label for=dup>D</label><input id=dup>"),
        ["dup=\"\""]
    );
    assert_eq!(
        names("<label for=\"\">E <input id=y></label><label for=Z>Z</label><input id=z>"),
        ["y=\"\"", "z=\"\""]
    );
}

/// HTML-AAM 4.1.1 to 4.1.4: a control encapsulated by its label leaves its own value, `alt` or
/// subtree out; accname 4.3.2, example 4, with an `input` as the embedded control.
#[test]
fn a_control_inside_its_label_leaves_its_own_value_out() {
    assert_eq!(
        names(
            "<label>Name <input id=n value=v></label>\
             <label>Go <button id=b>Inner</button></label>\
             <label>Find <input type=image id=im alt=Glass></label>\
             <label>Send <input type=submit id=s value=Now></label>"
        ),
        ["n=\"Name\"", "b=\"Go\"", "im=\"Find\"", "s=\"Send\""]
    );
    assert_eq!(
        names(
            "<label for=flash><input type=checkbox id=flash> Flash the screen \
             <input id=times value=5> times.</label>"
        ),
        ["flash=\"Flash the screen 5 times.\"", "times=\"\""]
    );
    // Embedded controls of other kinds: a range by `aria-valuetext`, then `aria-valuenow`, then
    // `value`; a `select` gives the empty string; a `textarea` its value.
    assert_eq!(
        names(
            "<label for=c><input type=checkbox id=c>A<input type=range aria-valuetext=low \
             aria-valuenow=1 value=2>B<input type=number aria-valuenow=3 value=4>\
             C<input type=range value=6>D<select><option>opt</select>E<textarea>tx</textarea>\
             </label>"
        ),
        [
            "c=\"AlowB3C6DEtx\"",
            "-=\"\"",
            "-=\"\"",
            "-=\"\"",
            "-=\"\"",
            "-=\"\""
        ]
    );
}

/// The `hidden` attribute, `aria-hidden` true in another case, the content of a closed
/// `details` but its first `summary`.
#[test]
fn hidden_subtree_is_left_out_of_link_text() {
    assert_eq!(
        names(
            "<a href=x id=a>Shown<span hidden>Gone</span></a>\
             <a href=y id=b>One<span aria-hidden=\"TRUE\">Two</span></a>\
             <a href=z id=c><details><summary>Sum</summary>Body<p>More</p></details></a>\
             <a href=w id=d>Open<details open><summary>S</summary>B</details></a>"
        ),
        ["a=\"Shown\"", "b=\"One\"", "c=\"Sum\"", "d=\"OpenSB\""]
    );
}

/// Accname 4.3.2, examples 1 and 2.
#[test]
fn hidden_node_referenced_by_aria_labelledby_names_with_its_hidden_subtree() {
    assert_eq!(
        names(
            "<button id=el1 aria-labelledby=el2></button>\
             <div id=el2 hidden><span hidden>hello</span></div>"
        ),
        ["el1=\"hello\""]
    );
    assert_eq!(
        names(
            "<button id=el1 aria-labelledby=el2></button>\
             <div id=el2><span hidden>hello</span></div>"
        ),
        ["el1=\"\""]
    );
}

#[test]
fn hidden_label_still_names_its_control() {
    assert_eq!(
        names(
            "<label for=i hidden>Hidden <span aria-hidden=true>label</span></label><input id=i>\
             <label for=j>Shown <span hidden>not this</span></label><input id=j>"
        ),
        ["i=\"Hidden label\"", "j=\"Shown\""]
    );
}

#[test]
fn hidden_element_is_named_from_its_own_subtree() {
    let tree = parse(
        "<div hidden><a href=x id=a>Secret <span>link</span></a></div>\
         <a href=y id=b aria-hidden=true>Also <span hidden>this</span></a>\
         <a href=z id=c>Not <span hidden>this</span></a>",
    );
    let rows = actionable_elements(&tree);
    assert_eq!(
        rows.iter().map(|r| line(&tree, r)).collect::<Vec<_>>(),
        ["a=\"Secret link\"", "b=\"Also this\"", "c=\"Not\""]
    );
    assert_eq!(
        rows.iter().map(|r| r.hidden).collect::<Vec<_>>(),
        [true, true, false]
    );
}

#[test]
fn element_without_any_source_has_an_empty_name() {
    assert_eq!(
        names(
            "<a href=x id=a></a><button id=b></button><input id=i value=typed>\
             <select id=s><option>o</select><textarea id=t>text</textarea>\
             <input type=checkbox id=c><input type=button id=bb><input type=file id=f>\
             <a href=y id=e><!--comment--></a>"
        ),
        [
            "a=\"\"", "b=\"\"", "i=\"\"", "s=\"\"", "t=\"\"", "c=\"\"", "bb=\"\"", "f=\"\"",
            "e=\"\"",
        ]
    );
}

/// Accname 4.3.2: a text node gives its textual contents (2G) and each child's result is
/// appended without a space (2F); the name is a flat string.
#[test]
fn whitespace_in_a_name_is_collapsed() {
    assert_eq!(
        names(
            "<a href id=w> a\n\tb <span>c</span>d</a>\
             <button id=l aria-label=\"  x &#10; y\t\">b</button>\
             <a href id=s><span> one </span><span>two</span>\r\n</a>"
        ),
        ["w=\"a b cd\"", "l=\"x y\"", "s=\"one two\""]
    );
}

#[test]
fn a_reference_cycle_ends() {
    // An element whose `aria-labelledby` names its ancestor.
    assert_eq!(
        names("<div id=p>Parent <a href=x id=a aria-labelledby=p>child</a></div>"),
        ["a=\"Parent child\""]
    );
    // Two links naming each other.
    assert_eq!(
        names(
            "<a href=1 id=one aria-labelledby=two>One</a><a href=2 id=two aria-labelledby=one>Two</a>"
        ),
        ["one=\"Two\"", "two=\"One\""]
    );
    // A repeated IDREF: the node is read once.
    assert_eq!(
        names("<span id=r>R</span><button id=b aria-labelledby=\"r r r\"></button>"),
        ["b=\"R\""]
    );
    // A descendant of the link names an ancestor of the link: the nodes read already are not
    // read again, so the ancestor gives nothing and the descendant falls back to its own text.
    assert_eq!(
        names("<div id=top><a href=x id=c>A<span aria-labelledby=top>S</span></a></div>"),
        ["c=\"AS\""]
    );
}

// The bounds.

#[test]
fn a_name_is_cut_at_label_bytes_max() {
    let mut input = String::from("<a href=x id=a>a");
    input.push_str(&"\u{e9}".repeat(2_600));
    input.push_str("</a><a href=y id=b>short</a>");
    let tree = parse(&input);
    let rows = actionable_elements(&tree);
    assert_eq!(rows.len(), 2);
    assert_eq!(LABEL_BYTES_MAX, 1_024);
    // 1 + 2 * 511 = 1,023 bytes: the next two-byte character would end at 1,025.
    let expected = format!("a{}", "\u{e9}".repeat(511));
    assert_eq!(rows[0].label.len(), 1_023);
    assert_eq!(rows[0].label, expected);
    assert!(rows[0].label_cut);
    assert_eq!(rows[1].label, "short");
    assert!(!rows[1].label_cut);
}

#[test]
fn past_the_label_work_bound_names_come_from_attributes_only() {
    let mut input = String::from("<span id=s>");
    input.push_str(&"x".repeat(65_536));
    input.push_str("</span>");
    input.push_str(&"<input title=T aria-labelledby=s>".repeat(20_000));
    let tree = parse(&input);
    let rows = actionable_elements(&tree);
    assert_eq!(rows.len(), 20_000);
    assert_eq!(LABEL_WORK_MAX, 16_777_216);
    assert!(rows.iter().all(|r| r.label.len() <= LABEL_BYTES_MAX));
    assert!(rows.iter().all(|r| r.label_cut));
    assert_eq!(rows[0].label, "x".repeat(1_024));
    let sum: usize = rows.iter().map(|r| r.label.len()).sum();
    assert!(sum <= LABEL_WORK_MAX, "the names hold {sum} bytes");
    assert_eq!(rows.last().unwrap().label, "T");
    let full = rows.iter().filter(|r| r.label.len() == 1_024).count();
    assert!(full > 10_000 && full < 20_000, "{full} names from the span");
    assert!(rows[full..].iter().all(|r| r.label == "T"));
}

#[test]
fn hidden_flags_cost_one_walk_for_many_references() {
    let bound = Duration::from_secs(10);

    // A `body` with `aria-hidden` true after 50,000 comments: the body element, whose
    // `aria-hidden` is not read.
    let mut input = "<!---->".repeat(50_000);
    input.push_str("<html><body aria-hidden=true><span id=t>T</span>");
    input.push_str(&"<input aria-labelledby=t>".repeat(50_000));
    let tree = parse(&input);
    let start = Instant::now();
    let rows = actionable_elements(&tree);
    let took = start.elapsed();
    assert_eq!(rows.len(), 50_000);
    assert!(rows.iter().all(|r| r.label == "T" && !r.hidden));
    assert!(took < bound, "{took:?} for the body after comments");

    // A `summary` after 50,000 children of a closed `details`.
    let mut input = String::from("<details>");
    input.push_str(&"<span></span>".repeat(50_000));
    input.push_str("<summary id=s>S</summary></details>");
    input.push_str(&"<input aria-labelledby=s>".repeat(50_000));
    let tree = parse(&input);
    let start = Instant::now();
    let rows = actionable_elements(&tree);
    let took = start.elapsed();
    assert_eq!(rows.len(), 50_000);
    assert!(rows.iter().all(|r| r.label == "S" && !r.hidden));
    assert!(took < bound, "{took:?} for the summary after children");
}

/// 100,000 nested `span` in a `button`, named by its subtree and, through `aria-labelledby`, by
/// a link before it. Not in an `a`: with an `a` in the list of active formatting elements each
/// `span` start tag reconstructs that list against the whole stack, and the parse alone runs
/// over 60 s until `story:html-parser-limits` bounds the depth.
#[test]
fn the_name_of_a_deep_subtree_is_computed_without_recursion() {
    let mut input = String::from("<a href=deep id=l aria-labelledby=b></a><button id=b>d");
    input.push_str(&"<span>".repeat(100_000));
    let tree = parse(&input);
    let rows = actionable_elements(&tree);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.label == "d" && !r.label_cut));
}

// Adversarial cases.

/// `LABEL_WORK_MAX` counts an element read as one unit, and the story states that the bounds
/// make the time linear in the nodes and the attribute bytes plus a fixed budget. 400 names each
/// read the 25 `span` of one `div`, each `span` with 1,024 attributes (the attribute limit of
/// `story:html-parser-limits`). The walk compares each attribute of the page a fixed number of
/// times (at most 16), whatever the number of names; names that searched the attributes of each
/// element they read would compare about 2 * 1,024 * 25 * 400 = 20,480,000. Counted, not timed:
/// a time comparison fails by chance on a loaded machine.
#[test]
fn the_time_of_the_names_does_not_grow_with_the_attributes_of_the_nodes_read() {
    fn page(attributes: usize) -> String {
        let mut span = String::from("<span");
        for i in 0..attributes {
            // As long as `aria-label`, so that a scan compares the bytes.
            span.push_str(&format!(" aria-l{i:04}"));
        }
        span.push_str("></span>");
        let mut input = String::from("<div id=d>");
        input.push_str(&span.repeat(25));
        input.push_str("</div>");
        input.push_str(&"<input aria-labelledby=d>".repeat(400));
        input
    }
    let tree = parse(&page(1_024));
    let attributes: usize = tree
        .descendants(tree.root())
        .into_iter()
        .map(|node| match tree.data(node) {
            NodeData::Element { attributes, .. } => attributes.len(),
            _ => 0,
        })
        .sum();
    assert_eq!(attributes, 25 * 1_024 + 1 + 400);
    let before = attribute_visits();
    let rows = actionable_elements(&tree);
    let visits = attribute_visits() - before;
    assert_eq!(rows.len(), 400);
    assert!(rows.iter().all(|r| r.label.is_empty() && !r.label_cut));
    assert!(
        visits <= 16 * attributes,
        "{visits} attribute comparisons for a page of {attributes} attributes"
    );
}

/// The unit's own documentation of 2E: an embedded Submit Button, Reset Button or Button gives
/// its `value`, or `Submit` or `Reset` without one; an Image Button its `alt`. A Submit Button
/// with `value=""` gives nothing, embedded or named itself.
#[test]
fn a_button_input_embedded_in_a_label_gives_its_value_or_its_default() {
    assert_eq!(
        names(
            "<label for=c><input type=checkbox id=c>A<input type=submit value=Go>B\
             <input type=reset>C<input type=submit value=\"\">D<input type=image alt=Pic>E\
             <input type=button>F</label>"
        ),
        [
            "c=\"AGoBResetCDPicEF\"",
            "-=\"Go\"",
            "-=\"Reset\"",
            "-=\"\"",
            "-=\"Pic\"",
            "-=\"\"",
        ]
    );
}

/// Accname 4.3.2 2C applies to a control embedded in the label "for another widget": a text
/// control that names itself through `aria-labelledby` does not give its own value.
#[test]
fn a_text_control_in_its_own_aria_labelledby_leaves_its_value_out() {
    assert_eq!(
        names("<span id=l>Label</span><input id=q value=typed aria-labelledby=\"l q\">"),
        ["q=\"Label\""]
    );
}
