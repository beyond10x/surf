//! The `selectedcontent` element (the fix round after the parser unit's adversary pass): the
//! parser fills each `selectedcontent` of a `select` with a copy of the selected option's
//! content; that copy is a mirror, hidden (`hid`) and left out of both text forms. A
//! `selectedcontent` the parser fills nothing into keeps its content.

use surf_page_text::markdown::markdown;
use surf_page_text::outline;

use crate::common::parse;

fn both(input: &str) -> (Vec<String>, Vec<String>) {
    let tree = parse(input);
    (outline(&tree).collect(), markdown(&tree).collect())
}

/// The parser adversary's page: the link of the option is one line and one handle.
#[test]
fn the_copy_of_the_selected_option_is_left_out() {
    let (outline, markdown) = both(
        "<!DOCTYPE html><select><button><selectedcontent></selectedcontent></button>\
         <option><a href=x>x</a></option></select>",
    );
    assert_eq!(
        &outline[3..],
        [
            "    select @f1",
            "      button @b1",
            "        selectedcontent hid 2",
            "      option",
            "        a @l1 href=\"x\" \"x\""
        ]
    );
    assert_eq!(markdown, ["[@f1 select]"]);
}

#[test]
fn two_selectedcontent_elements_are_both_left_out() {
    let (outline, markdown) = both(
        "<select><button><selectedcontent></selectedcontent><selectedcontent></selectedcontent>\
         </button><option>a<option selected><b>b</b></select>",
    );
    assert_eq!(
        &outline[3..],
        [
            "    select @f1",
            "      button @b1",
            "        selectedcontent hid 2",
            "        selectedcontent hid 2",
            "      option \"a\"",
            "      option",
            "        b \"b\""
        ]
    );
    assert_eq!(markdown, ["[@f1 select]"]);
}

/// Outside a `select`, and below a `select` with `multiple`, the parser copies nothing into a
/// `selectedcontent`: its own content is shown as any other.
#[test]
fn a_selectedcontent_the_parser_does_not_fill_is_shown() {
    let (outline, markdown) =
        both("<div><selectedcontent><a href=y>y</a> z</selectedcontent></div>");
    assert_eq!(
        &outline[3..],
        [
            "    div",
            "      selectedcontent",
            "        a @l1 href=\"y\" \"y\"",
            "        \"z\""
        ]
    );
    assert_eq!(markdown, ["[y](<y>) [@l1] z"]);
    let (outline, _) = both(
        "<select multiple><button><selectedcontent></selectedcontent></button><option>m</select>",
    );
    assert_eq!(outline[5], "        selectedcontent");
}
