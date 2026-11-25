//! Adversarial cases of the generator's `[SameObject]` (story `scripts-dom-reading`, decision
//! 10): a row marked `[SameObject]` that the generated getter cannot hold, on an interface
//! whose `this` is no node or on an operation. The generator either refuses the selection or
//! writes a getter that holds the mark; it must not write a getter that silently makes a new
//! object at each read.

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
                    interface NodeList {\n\
                    \x20 getter Node? item(unsigned long index);\n\
                    \x20 readonly attribute unsigned long length;\n\
                    };\n\
                    interface DOMException { readonly attribute DOMString name; };\n";

/// Each case: extra IDL, the selection's rows, and the generated function of the member.
#[test]
fn a_same_object_row_the_getter_cannot_hold_is_refused_or_held() {
    let cases = [
        (
            "a [SameObject] NodeList attribute of the global",
            "interface Window : EventTarget { [SameObject] readonly attribute NodeList kept; };\n",
            "definitions: EventTarget Node NodeList Window DOMException\nglobal: Window\n\
             Window kept reading\nNodeList item reading\nNodeList length reading\n",
            "getter_window_kept",
        ),
        (
            "a [SameObject] NodeList attribute of a non-node interface",
            "interface Window : EventTarget {};\n\
             interface Holder { [SameObject] readonly attribute NodeList kept; };\n",
            "definitions: EventTarget Node NodeList Window DOMException Holder\nglobal: Window\n\
             Holder kept reading\nNodeList item reading\nNodeList length reading\n",
            "getter_holder_kept",
        ),
    ];
    // The generator refuses such a row, naming it; holding it is not built.
    let mut silent = Vec::new();
    let mut seen = Vec::new();
    for (name, extra, selection, getter) in cases {
        let idl = format!("{BASE}{extra}");
        let words: Vec<&str> = selection
            .lines()
            .nth(2)
            .unwrap_or_default()
            .split(' ')
            .collect();
        let row = format!("`{} {}`", words[0], words[1]);
        match generated(&idl, selection) {
            Err(refusal) if refusal.contains("[SameObject]") && refusal.contains(&row) => {
                seen.push(format!("{name}: refused: {refusal}"));
            }
            Err(refusal) => {
                seen.push(format!("{name}: refused for another reason: {refusal}"));
                silent.push(name);
            }
            Ok(text) => match function(&text, getter) {
                Some(body) if body.contains("same_object(") => {
                    seen.push(format!("{name}: held, not refused"));
                    silent.push(name);
                }
                Some(_) => {
                    seen.push(format!("{name}: a getter that makes a new object per read"));
                    silent.push(name);
                }
                None => {
                    seen.push(format!("{name}: no function {getter}"));
                    silent.push(name);
                }
            },
        }
    }
    eprintln!("{}", seen.join("\n"));
    assert!(silent.is_empty(), "{seen:#?}");
}
