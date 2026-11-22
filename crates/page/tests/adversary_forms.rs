//! Adversary cases on `story:agent-form-port` in `surf-page`: the form owner at the edges of
//! the `form` attribute, `handles` beside forms, and `forms` and `form_handles` on a page with
//! many forms, options and controls (`PROBE` each; counts asserted, no wall time).

use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_page::forms::forms;
use surf_page::{form_handles, handles};

fn parse(html: &str) -> Tree {
    parse_document(html, &ParseOptions::default())
}

fn id_of(tree: &Tree, node: NodeId) -> String {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes
            .iter()
            .find(|a| a.namespace.is_none() && a.local_name == "id")
            .map_or_else(|| "?".to_owned(), |a| a.value.clone()),
        _ => "?".to_owned(),
    }
}

/// "form owner" at the edges: an ID first held by a `p` and then by a form names no form; two
/// forms with one ID, the first; a form later in the document; markup that nests a form start
/// tag (the parser drops the inner one).
#[test]
fn adversary_forms_probe_form_attribute_edges() {
    let tree = parse(
        "<!doctype html><p id=dup></p><form id=dup></form>\
         <input name=to-dup form=dup>\
         <form id=twin></form><form id=twin><input name=in-second-twin form=twin></form>\
         <input name=later form=late>\
         <form id=outer><form id=inner><input name=nested></form></form>\
         <form id=late></form>",
    );
    let found = forms(&tree);
    let owners: Vec<String> = found
        .controls
        .iter()
        .map(|c| {
            format!(
                "{} -> {}",
                c.name.as_deref().unwrap_or(""),
                c.owner
                    .map_or_else(|| "-".to_owned(), |at| id_of(&tree, found.forms[at].node))
            )
        })
        .collect();
    assert_eq!(
        owners,
        [
            "to-dup -> -",
            "in-second-twin -> twin",
            "later -> late",
            "nested -> outer"
        ]
    );
    let twin_first = found
        .controls
        .iter()
        .find(|c| c.name.as_deref() == Some("in-second-twin"))
        .and_then(|c| c.owner);
    assert_eq!(
        twin_first,
        Some(1),
        "the first form with the ID, not the ancestor"
    );
}

/// `handles` of a page with forms, links, buttons and fields: the letters `l`, `b`, `f`
/// counted over the actionable elements as `crates/page/src/lib.rs` at `3538efd` counts them;
/// a hidden input has no handle and a form none.
#[test]
fn adversary_forms_probe_handles_unchanged() {
    let tree = parse(
        "<!doctype html><a href=/a>A</a><form action=/s><input name=q>\
         <input type=hidden name=h value=1><select name=s><option>o</select>\
         <button>Go</button><input type=submit></form><a href=/b>B</a>\
         <form><textarea name=t></textarea><input type=image alt=I></form>",
    );
    let plain: Vec<String> = handles(&tree).into_iter().map(|(_, h)| h).collect();
    assert_eq!(plain, ["l1", "f1", "f2", "b1", "b2", "l2", "f3", "b3"]);
    let named: Vec<String> = form_handles(&tree).into_iter().map(|(_, h)| h).collect();
    assert_eq!(named, ["F1", "F2"]);
}

/// The size of the probe below: at 100,000 it ran 7 s to 14 s; at this size it runs under 3 s
/// alone. It asserts counts, never wall time.
const PROBE: usize = 20_000;

/// `PROBE` forms, a select of `PROBE` options and a form of `PROBE` controls: `forms` and
/// `form_handles` find each of them.
#[test]
fn adversary_forms_probe_many() {
    let mut html = String::from("<!doctype html>");
    for _ in 0..PROBE {
        html.push_str("<form></form>");
    }
    html.push_str("<form id=big><select name=s>");
    for n in 0..PROBE {
        html.push_str(&format!("<option>o{n}"));
    }
    html.push_str("</select>");
    for _ in 0..PROBE {
        html.push_str("<input name=c>");
    }
    html.push_str("</form>");
    let tree = parse(&html);
    let found = forms(&tree);
    let numbered = form_handles(&tree);
    assert_eq!(found.forms.len(), PROBE + 1);
    assert_eq!(found.controls.len(), PROBE + 1);
    assert_eq!(found.controls[0].options.len(), PROBE);
    assert!(found.controls[0].options[0].selected);
    assert!(found.controls.iter().all(|c| c.owner == Some(PROBE)));
    assert_eq!(
        numbered.last().map(|(_, h)| h.as_str()),
        Some(format!("F{}", PROBE + 1).as_str())
    );
}
