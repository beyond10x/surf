//! The `| ` tree format of the tree-construction data, read into nodes and written back.

use surf_html_conformance::cases::{self, Expected};
use surf_html_conformance::dump::{self, DumpNode};
use surf_html_conformance::inventory::{self, Group};
use surf_html_conformance::paths;

fn holds_processing_instruction(nodes: &[DumpNode]) -> bool {
    nodes.iter().any(|node| match node {
        DumpNode::ProcessingInstruction { .. } => true,
        DumpNode::Element {
            content, children, ..
        } => {
            holds_processing_instruction(children)
                || content.as_deref().is_some_and(holds_processing_instruction)
        }
        _ => false,
    })
}

#[test]
fn the_tree_format_round_trips_the_expected_trees() {
    let inventory = inventory::load(&paths::note()).unwrap_or_else(|e| panic!("{e}"));
    let mut documents = 0;
    let mut with_processing_instruction = 0;
    let mut failures = Vec::new();
    for file in inventory
        .files
        .iter()
        .filter(|f| f.group == Group::TreeConstruction)
    {
        let read =
            cases::read(&paths::data(), file.group, &file.path).unwrap_or_else(|e| panic!("{e}"));
        for case in read {
            let Expected::Tree { document, .. } = &case.expected else {
                panic!("{} {}: not a tree case", file.path, case.number);
            };
            documents += 1;
            match dump::read(document) {
                Ok(nodes) => {
                    if holds_processing_instruction(&nodes) {
                        with_processing_instruction += 1;
                    }
                    let written = dump::write(&nodes);
                    if written != *document {
                        failures.push(format!(
                            "{} {}: written back differs\n--- data\n{document}\n--- written\n{written}",
                            file.path, case.number
                        ));
                    }
                }
                Err(e) => failures.push(format!("{} {}: {e}", file.path, case.number)),
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let tree_cases: usize = inventory
        .files
        .iter()
        .filter(|f| f.group == Group::TreeConstruction)
        .map(|f| f.cases)
        .sum();
    assert_eq!(
        documents, tree_cases,
        "every tree-construction case was read"
    );
    assert_eq!(
        with_processing_instruction, 81,
        "the cases whose expected tree holds a processing instruction"
    );
}

#[test]
fn the_writer_sorts_attributes_and_writes_template_contents() {
    let nodes = vec![DumpNode::Element {
        name: "template".into(),
        attributes: vec![
            ("xlink href".into(), "b".into()),
            ("a".into(), "x\ny".into()),
        ],
        content: Some(vec![DumpNode::Text("t".into())]),
        children: vec![],
    }];
    assert_eq!(
        dump::write(&nodes),
        "| <template>\n|   a=\"x\ny\"\n|   xlink href=\"b\"\n|   content\n|     \"t\""
    );
}
