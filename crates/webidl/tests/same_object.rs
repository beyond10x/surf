//! `[SameObject]` (story `scripts-dom-reading`): the getter of an attribute marked
//! `[SameObject]` that returns a `NodeList` or an `HTMLCollection` asks the wrapper hook for the
//! object it kept for the node and the member, and keeps the one it makes; a member without the
//! mark, such as an operation that returns a collection, makes a new object at each call. The
//! generator names each such member in the enum `SameObject`.

use surf_webidl::{Model, Selection, generate, read};

const IDL: &str = "interface EventTarget {};\n\
                   interface Node : EventTarget {\n\
                   \x20 [SameObject] readonly attribute NodeList childNodes;\n\
                   \x20 NodeList listNodes();\n\
                   };\n\
                   interface NodeList {\n\
                   \x20 getter Node? item(unsigned long index);\n\
                   \x20 readonly attribute unsigned long length;\n\
                   };\n\
                   interface Window : EventTarget {};\n\
                   interface DOMException { readonly attribute DOMString name; };\n";

const SELECTION: &str = "definitions: EventTarget Node NodeList Window DOMException\n\
                         global: Window\n\
                         Node childNodes reading\n\
                         Node listNodes reading\n\
                         NodeList item reading\n\
                         NodeList length reading\n";

/// The text of the generated function `name`, up to the next item.
fn function<'t>(text: &'t str, name: &str) -> &'t str {
    let start = text
        .find(&format!("\nfn {name}("))
        .unwrap_or_else(|| panic!("no function {name}"));
    let rest = &text[start + 1..];
    let end = rest[1..].find("\nfn ").map_or(rest.len(), |at| at + 1);
    &rest[..end]
}

#[test]
fn the_getters_of_the_four_rows_ask_wrap_for_the_kept_object() {
    let model = Model::new(read(IDL, "small.idl").expect("read")).expect("model");
    let selection = Selection::read(SELECTION, "s.txt").expect("the selection");
    let text = generate(&model, &selection, &[]).expect("generates");

    let kept = function(&text, "getter_node_child_nodes");
    assert!(kept.contains("SameObject::NodeChildNodes"), "{kept}");
    assert!(kept.contains(".same_object("), "{kept}");
    assert!(kept.contains(".keep_same_object("), "{kept}");

    let made = function(&text, "operation_node_list_nodes");
    assert!(!made.contains("same_object"), "{made}");
    assert!(made.contains("collection_value("), "{made}");

    let start = text
        .find("pub enum SameObject {")
        .expect("the enum SameObject");
    let body = &text[start..start + text[start..].find('}').expect("its end")];
    let variants: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|line| line.ends_with(',') && !line.starts_with("///"))
        .collect();
    assert_eq!(variants, ["NodeChildNodes,"], "{body}");
}
