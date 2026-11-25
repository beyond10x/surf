//! Adversarial cases of the generator's `[SameObject]` (story `scripts-dom-reading`), second
//! review: the `[SameObject]` attributes of the vendored IDL that a later slice selects, each
//! as a small selection. The rule of the unit: the generator holds a `[SameObject]` row or
//! refuses it with an error that names the row; it never writes a getter that makes a new
//! object at each read without a word.

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

const BASE: &str = "interface EventTarget {};\n\
                    interface Node : EventTarget {};\n\
                    interface Window : EventTarget {};\n\
                    interface NodeList {\n\
                    \x20 getter Node? item(unsigned long index);\n\
                    \x20 readonly attribute unsigned long length;\n\
                    };\n\
                    interface HTMLCollection {\n\
                    \x20 getter Element? item(unsigned long index);\n\
                    \x20 readonly attribute unsigned long length;\n\
                    };\n\
                    interface DOMException { readonly attribute DOMString name; };\n";

const ROWS: &str = "global: Window\nNodeList item reading\nNodeList length reading\n\
                    HTMLCollection item reading\nHTMLCollection length reading\n";

/// The rows of the vendored IDL marked `[SameObject]` that a later slice of the DOM and HTML
/// selects: `Element.attributes`, `Element.classList`, `Document.implementation`,
/// `MutationRecord.addedNodes`, `HTMLFormElement.elements`, `Document.forms`. What the generator
/// does with each, printed; a getter that makes a new object at each read is a defect.
#[test]
fn the_same_object_rows_a_later_slice_selects() {
    let cases = [
        (
            "Element.attributes, a NamedNodeMap",
            "interface Element : Node { [SameObject] readonly attribute NamedNodeMap attributes; };\n\
             interface NamedNodeMap { readonly attribute unsigned long length; };\n",
            "definitions: EventTarget Node Window NodeList HTMLCollection DOMException Element NamedNodeMap\n\
             Element attributes attributes\nNamedNodeMap length attributes\n",
            "getter_element_attributes",
        ),
        (
            "Element.classList, a DOMTokenList",
            "interface Element : Node { [SameObject, PutForwards=value] readonly attribute DOMTokenList classList; };\n\
             interface DOMTokenList { stringifier attribute DOMString value; };\n",
            "definitions: EventTarget Node Window NodeList HTMLCollection DOMException Element DOMTokenList\n\
             Element classList attributes\nDOMTokenList value attributes\n",
            "getter_element_class_list",
        ),
        (
            "Document.implementation, a DOMImplementation",
            "interface Element : Node {};\n\
             interface Document : Node { [SameObject] readonly attribute DOMImplementation implementation; };\n\
             interface DOMImplementation { boolean hasFeature(); };\n",
            "definitions: EventTarget Node Window NodeList HTMLCollection DOMException Element Document DOMImplementation\n\
             Document implementation reading\nDOMImplementation hasFeature reading\n",
            "getter_document_implementation",
        ),
        (
            "MutationRecord.addedNodes, a NodeList of a non-node interface",
            "interface Element : Node {};\n\
             interface MutationRecord { [SameObject] readonly attribute NodeList addedNodes; };\n",
            "definitions: EventTarget Node Window NodeList HTMLCollection DOMException Element MutationRecord\n\
             MutationRecord addedNodes mutation\n",
            "getter_mutation_record_added_nodes",
        ),
        (
            "HTMLFormElement.elements, an HTMLFormControlsCollection",
            "interface Element : Node {};\n\
             interface HTMLFormControlsCollection : HTMLCollection {};\n\
             interface HTMLFormElement : Element { [SameObject] readonly attribute HTMLFormControlsCollection elements; };\n",
            "definitions: EventTarget Node Window NodeList HTMLCollection DOMException Element HTMLFormControlsCollection HTMLFormElement\n\
             HTMLFormElement elements reading\n",
            "getter_html_form_element_elements",
        ),
        (
            "Document.forms, an HTMLCollection",
            "interface Element : Node {};\n\
             interface Document : Node { [SameObject] readonly attribute HTMLCollection forms; };\n",
            "definitions: EventTarget Node Window NodeList HTMLCollection DOMException Element Document\n\
             Document forms reading\n",
            "getter_document_forms",
        ),
    ];
    let mut seen = Vec::new();
    let mut silent = Vec::new();
    for (name, extra, rows, getter) in cases {
        let idl = format!("{BASE}{extra}");
        let mut lines = rows.lines();
        let definitions = lines.next().unwrap_or_default();
        let rest: Vec<&str> = lines.collect();
        let selection = format!("{definitions}\n{ROWS}{}\n", rest.join("\n"));
        match generated(&idl, &selection) {
            Err(refusal) if refusal.contains("SameObject") => {
                seen.push(format!("{name}: refused naming [SameObject]: {refusal}"));
            }
            Err(refusal) => seen.push(format!("{name}: refused for another reason: {refusal}")),
            Ok(text) => match function(&text, getter) {
                Some(body) if body.contains("same_object(") => {
                    seen.push(format!("{name}: held"));
                }
                Some(_) => {
                    seen.push(format!("{name}: a plain getter, a new object per read"));
                    silent.push(name);
                }
                None => seen.push(format!("{name}: no function {getter}")),
            },
        }
    }
    eprintln!("{}", seen.join("\n"));
    assert!(silent.is_empty(), "{seen:#?}");
}
