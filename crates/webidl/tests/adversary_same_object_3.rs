//! Adversarial cases of the generator's `[SameObject]` (story `scripts-dom-reading`), third
//! review: made-up rows of every return form on a node, on the global and on a non-node
//! interface, with and without the mark. The rule of the unit: a `[SameObject]` attribute is
//! held (a collection of a node through the kept object, a node through its one wrapper) or
//! refused with an error that names the row; never a getter that makes a new object a read.

use surf_webidl::{Model, Selection, generate, read};

fn generated(idl: &str, selection: &str) -> Result<String, String> {
    let model = Model::new(read(idl, "small.idl").map_err(|e| format!("{e:?}"))?)
        .map_err(|e| format!("{e:?}"))?;
    let selection = Selection::read(selection, "s.txt").map_err(|e| format!("{e:?}"))?;
    generate(&model, &selection, &[]).map_err(|e| format!("{e:?}"))
}

fn function<'t>(text: &'t str, name: &str) -> Option<&'t str> {
    let start = text.find(&format!("\nfn {name}("))?;
    let rest = &text[start + 1..];
    let end = rest[1..].find("\nfn ").map_or(rest.len(), |at| at + 1);
    Some(&rest[..end])
}

/// The IDL with `member` on `host` (`Node`, `Window` or `NodeList`).
fn idl(host: &str, member: &str) -> String {
    let on = |name: &str| if name == host { member } else { "" };
    format!(
        "interface EventTarget {{}};\n\
         interface Node : EventTarget {{ {} }};\n\
         interface Window : EventTarget {{ {} }};\n\
         interface NodeList {{\n\
         \x20 getter Node? item(unsigned long index);\n\
         \x20 readonly attribute unsigned long length;\n\
         \x20 {}\n\
         }};\n\
         dictionary Bag {{ DOMString name; }};\n\
         interface DOMException {{ readonly attribute DOMString name; }};\n",
        on("Node"),
        on("Window"),
        on("NodeList"),
    )
}

fn selection(host: &str) -> String {
    format!(
        "definitions: EventTarget Node Window NodeList DOMException\n\
         global: Window\n\
         {host} kept reading\n\
         NodeList item reading\n\
         NodeList length reading\n"
    )
}

/// What the generator does with `[SameObject]` (or without it) on a row of each return form on
/// each kind of `this`: held (with the hook the getter calls) or refused (and why).
fn outcome(host: &str, mark: bool, ty: &str) -> (bool, String) {
    let member = format!(
        "{}readonly attribute {ty} kept;",
        if mark { "[SameObject] " } else { "" }
    );
    let getter = format!(
        "getter_{}_kept",
        host.to_ascii_lowercase().replace("nodelist", "node_list")
    );
    match generated(&idl(host, &member), &selection(host)) {
        Ok(text) => {
            let body = function(&text, &getter).unwrap_or("(no getter)");
            let how = if body.contains("same_object(") {
                "kept object"
            } else if body.contains(", node_value)") || body.contains("node_value(") {
                "wrapper of the node"
            } else if body.contains("collection_value(") {
                "NEW COLLECTION A READ"
            } else {
                "other"
            };
            (true, format!("generated ({how})"))
        }
        Err(error) => {
            let names_row = error.contains(&format!("{host} kept"));
            let same_object = error.contains("[SameObject]");
            (
                false,
                format!(
                    "refused (row named: {names_row}, for [SameObject]: {same_object}): {}",
                    error.chars().take(160).collect::<String>()
                ),
            )
        }
    }
}

/// Every made-up `[SameObject]` row is refused, or generated with a getter that answers one
/// object for one node: the kept collection of a node, or the wrapper of a node. The same rows
/// without the mark are printed beside them.
#[test]
fn same_object_rows_of_every_form_on_every_this() {
    let forms = [
        "Node",
        "Node?",
        "NodeList",
        "(Node or DOMString)",
        "sequence<Node>",
        "Bag",
        "DOMString",
        "Promise<Node>",
        "boolean",
    ];
    let mut seen = Vec::new();
    let mut wrong = Vec::new();
    for host in ["Node", "Window", "NodeList"] {
        for ty in forms {
            for mark in [true, false] {
                let (made, how) = outcome(host, mark, ty);
                let line = format!(
                    "{host}.kept {ty}{}: {how}",
                    if mark { " [SameObject]" } else { "" }
                );
                if mark
                    && made
                    && !(how.contains("kept object") || how.contains("wrapper of the node"))
                {
                    wrong.push(line.clone());
                }
                if mark && !made && !how.contains("row named: true") && !how.contains("ReadError") {
                    wrong.push(line.clone());
                }
                seen.push(line);
            }
        }
    }
    eprintln!("{}", seen.join("\n"));
    assert!(wrong.is_empty(), "{wrong:#?}");
}
