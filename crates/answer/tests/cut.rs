//! Not one of the story's counted cases: a field with a length limit is cut before escaping on a
//! character boundary, ends in `...`, and reads back to its kept part and the mark that it was
//! cut. No block of step 1 has such a field; the label of the `now` block is the first.

use surf_answer::{Cut, UnquoteError, quote_cut, unquote_cut};

#[test]
fn cut_field_reads_back_kept_part_and_mark() {
    let long = format!("{}\u{2028}tail", "é".repeat(59));
    let field = quote_cut(&long, 60);
    assert_eq!(
        field,
        format!("\"{}\\xe2\\x80\\xa8\"...", "\\xc3\\xa9".repeat(59))
    );
    assert_eq!(
        unquote_cut(&field, 60),
        Ok(Cut {
            kept: format!("{}\u{2028}", "é".repeat(59)),
            cut: true,
        })
    );

    let dots = "ends in dots...";
    assert_eq!(quote_cut(dots, 60), "\"ends in dots...\"");
    assert_eq!(
        unquote_cut(&quote_cut(dots, 60), 60),
        Ok(Cut {
            kept: dots.to_string(),
            cut: false,
        })
    );

    assert_eq!(unquote_cut("\"short\"...", 60), Err(UnquoteError::Length));
    assert_eq!(
        unquote_cut(&format!("\"{}\"", "a".repeat(61)), 60),
        Err(UnquoteError::Length)
    );
}
