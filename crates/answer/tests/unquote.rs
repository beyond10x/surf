//! `unquote` reads one quoted field back to its exact value and refuses anything the quoting
//! function cannot have written.

use surf_answer::{UnquoteError, unquote};

#[test]
fn unquote_escapes_give_exact_value() {
    assert_eq!(
        unquote(r#""a\x22b\x5cc\xe2\x80\xa8""#),
        Ok("a\"b\\c\u{2028}".to_string())
    );
}

#[test]
fn unquote_refuses_no_closing_quote() {
    assert_eq!(unquote(r#""abc"#), Err(UnquoteError::NoClosingQuote));
}

#[test]
fn unquote_refuses_escape_with_one_digit() {
    assert_eq!(unquote(r#""a\x2""#), Err(UnquoteError::ShortEscape));
}

#[test]
fn unquote_refuses_upper_case_hex_digits() {
    assert_eq!(unquote(r#""a\x5Cb""#), Err(UnquoteError::UpperCaseHex));
}

#[test]
fn unquote_refuses_byte_outside_printable() {
    assert_eq!(unquote("\"a\nb\""), Err(UnquoteError::Unprintable));
}

#[test]
fn unquote_refuses_escape_not_utf8() {
    assert_eq!(unquote(r#""\xe2\x80""#), Err(UnquoteError::NotUtf8));
}
