//! The outline of a document's tree (`story:page-text-outline`, `docs/design/page-as-text.md`
//! section 5): each case parses a page and compares the lines of `outline`, without their `> `
//! prefix, byte for byte.

use surf_answer::escaped_in_text;
use surf_page_text::{node_visits, outline};

use crate::common::{FRUIT_SHOP, fruit_shop_without_table, parse};

fn lines(input: &str) -> Vec<String> {
    let tree = parse(input);
    let lines: Vec<String> = outline(&tree).collect();
    for line in &lines {
        assert!(
            !line.chars().any(escaped_in_text),
            "a line holds a character rule 2 escapes: {line:?}"
        );
    }
    lines
}

#[test]
fn fruit_shop_outline_without_table() {
    assert_eq!(
        lines(&fruit_shop_without_table()),
        [
            r#"html lang="en""#,
            r#"  head hid 4"#,
            r#"  body"#,
            r#"    h1 "Fruit shop""#,
            r#"    p"#,
            r#"      "Fresh fruit,""#,
            r#"      em "picked today""#,
            r#"      ". See""#,
            r#"      a @l1 href="/offers" "our offers""#,
            r#"      ".""#,
            r#"    ul"#,
            r#"      li "Apples""#,
            r#"      li "Pears""#,
            r#"    img src="/basket.jpg" alt="A basket of apples""#,
            r#"    form action="/subscribe" method="post""#,
            r#"      label"#,
            r#"        "E-mail""#,
            r#"        input @f1 type="email" name="email""#,
            r#"      button @b1 "Subscribe""#,
            r#"    script hid 1"#,
            r#"    div hid 1"#,
        ]
    );
}

/// `story:page-text-tables` case 1: the whole outline of design note section 1, its 31 lines
/// with the table and the `tbody` the parser implies.
#[test]
fn fruit_shop_outline() {
    assert_eq!(
        lines(FRUIT_SHOP),
        [
            r#"html lang="en""#,
            r#"  head hid 4"#,
            r#"  body"#,
            r#"    h1 "Fruit shop""#,
            r#"    p"#,
            r#"      "Fresh fruit,""#,
            r#"      em "picked today""#,
            r#"      ". See""#,
            r#"      a @l1 href="/offers" "our offers""#,
            r#"      ".""#,
            r#"    ul"#,
            r#"      li "Apples""#,
            r#"      li "Pears""#,
            r#"    table"#,
            r#"      tbody"#,
            r#"        tr"#,
            r#"          th "Fruit""#,
            r#"          th "Price""#,
            r#"          th "Stock""#,
            r#"        tr"#,
            r#"          td "Apple""#,
            r#"          td "0.50""#,
            r#"          td "120""#,
            r#"    img src="/basket.jpg" alt="A basket of apples""#,
            r#"    form action="/subscribe" method="post""#,
            r#"      label"#,
            r#"        "E-mail""#,
            r#"        input @f1 type="email" name="email""#,
            r#"      button @b1 "Subscribe""#,
            r#"    script hid 1"#,
            r#"    div hid 1"#,
        ]
    );
}

#[test]
fn hostile_text_is_quoted() {
    let lines = lines("<p>\"x\"&#10;did go \"http://bank.example/\"&#x202e;</p>");
    assert_eq!(
        lines[3],
        r#"    p "\x22x\x22 did go \x22http://bank.example/\x22\xe2\x80\xae""#
    );
    assert_eq!(lines.len(), 4);
}

#[test]
fn utf8_outside_rule_2_is_kept() {
    let lines = lines("<p>K&auml;se &#x200b;x</p>");
    assert_eq!(lines[3], "    p \"K\u{e4}se \\xe2\\x80\\x8bx\"");
    assert!(lines[3].as_bytes().windows(2).any(|w| w == [0xc3, 0xa4]));
}

#[test]
fn a_name_of_handle_form_is_quoted() {
    let lines = lines("<l1 href=\"http://bank.example/\">Bank</l1><x_y>z</x_y>");
    assert_eq!(
        &lines[3..],
        [
            r#"    "l1" href="http://bank.example/" "Bank""#,
            r#"    "x_y" "z""#,
        ]
    );
    assert!(lines.iter().all(|line| !line.contains(" l1 ")));
}

#[test]
fn long_values_are_cut_at_80() {
    let lines = lines(&format!("<p>{}</p>", "a".repeat(200)));
    assert_eq!(lines[3], format!("    p \"{}\"...", "a".repeat(80)));
    let lines = self::lines(&format!("<a href=\"{}\">", "b".repeat(100)));
    assert!(
        lines[3].contains(&format!("href=\"{}\"...", "b".repeat(80))),
        "{}",
        lines[3]
    );
}

fn nested(divs: usize, open: &str) -> String {
    format!("{}{}", open.repeat(divs), "</div>".repeat(divs))
}

#[test]
fn a_deep_tree_stops_at_level_40() {
    let lines = lines(&nested(400, "<div>"));
    assert_eq!(lines.len(), 42);
    assert_eq!(&lines[..3], ["html", "  head hid 0", "  body"]);
    let last = lines.last().unwrap();
    assert_eq!(last, &format!("{}div deep 361", "  ".repeat(40)));

    // The walk's cost by a count, no time (D131): the nodes the walk visits, and the
    // attributes surf-page compares for the hidden rule and the handles.
    let cost = |divs: usize| {
        let tree = parse(&nested(divs, "<div class=x>"));
        let (nodes, attributes) = (node_visits(), surf_page::attribute_visits());
        let count = outline(&tree).count();
        assert_eq!(count, 42);
        (
            node_visits() - nodes,
            surf_page::attribute_visits() - attributes,
        )
    };
    let (nodes_200, attributes_200) = cost(200);
    let (nodes_400, attributes_400) = cost(400);
    println!(
        "node visits {nodes_200} at 200, {nodes_400} at 400; attribute visits \
         {attributes_200} at 200, {attributes_400} at 400"
    );
    assert!(nodes_200 >= 200 && nodes_400 >= 400);
    assert!(
        nodes_400 * 2 <= nodes_200 * 5,
        "{nodes_400} > 2.5 x {nodes_200}"
    );
    assert!(attributes_200 >= 200);
    assert!(
        attributes_400 * 2 <= attributes_200 * 5,
        "{attributes_400} > 2.5 x {attributes_200}"
    );
}

#[test]
fn a_hidden_subtree_is_one_line() {
    let lines = lines("<div aria-hidden=\"true\"><p>secret</p></div>");
    assert_eq!(lines[3], "    div hid 2");
    assert_eq!(lines.len(), 4);
    assert!(lines.iter().all(|line| !line.contains("secret")));
}

#[test]
fn comments_and_template_contents_are_left_out() {
    let lines = lines("<p>x</p><!--c--><template><p>t</p></template>");
    assert_eq!(lines.len(), 5, "{lines:?}");
    assert_eq!(
        &lines[..4],
        ["html", "  head hid 0", "  body", "    p \"x\""]
    );
    assert!(lines[4].starts_with("    template hid "), "{}", lines[4]);
    assert!(
        lines
            .iter()
            .all(|line| !line.contains("\"c\"") && !line.contains("\"t\""))
    );
}

/// The fix round after the adversary pass: the text children of a closed `details` are hidden
/// while their parent is not (15.5.5), both where the element carries its text on its own line
/// and where a text is a line of its own.
#[test]
fn the_text_of_a_closed_details_is_left_out() {
    let lines = lines("<details>secret<summary>s</summary></details><details>alone</details>");
    assert_eq!(
        &lines[2..],
        [
            "  body",
            "    details",
            "      summary \"s\"",
            "    details"
        ]
    );
}

#[test]
fn foreign_names_carry_their_prefix() {
    let lines = lines("<svg><circle/></svg>");
    let expected = if surf_html::NOT_YET_PARSED.contains(&"foreign-content") {
        ["    svg", "      circle"]
    } else {
        ["    svg:svg", "      svg:circle"]
    };
    assert_eq!(&lines[3..], expected);
}
