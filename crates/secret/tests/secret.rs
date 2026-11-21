//! The counted cases of `story:agent-secret-primitives`, one test per case, named after it.

#![forbid(unsafe_code)]

use std::path::Path;

use surf_secret::{Token, equal, fill_random, fill_random_from};

/// The comparison's source compiled into this test crate, where `cfg(test)` holds, so the count
/// of bytes read that the library carries in tests only is visible here. Cases 10 and 11 read the
/// count from this copy and check that the library's own `equal` gives the same answer.
#[path = "../src/compare.rs"]
mod compare;

fn is_lower_hex(character: char) -> bool {
    matches!(character, '0'..='9' | 'a'..='f')
}

#[test]
fn case_01_two_draws_of_32_bytes_differ() {
    let mut first = [0u8; 32];
    let mut second = [0u8; 32];
    fill_random(&mut first).expect("the random source");
    fill_random(&mut second).expect("the random source");
    assert_ne!(first, second);

    let first = Token::draw().expect("the random source");
    let second = Token::draw().expect("the random source");
    assert_ne!(first.to_hex(), second.to_hex());
    assert!(!first.matches(&second));
}

#[test]
fn case_02_fill_random_from_a_path_that_does_not_exist_is_an_error() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("surf-secret-no-such-device");
    assert!(!path.exists(), "{} exists", path.display());
    let mut buffer = [0u8; 32];
    let result = fill_random_from(&path, &mut buffer);
    assert!(result.is_err(), "{result:?}");
    assert_eq!(buffer, [0u8; 32]);
}

#[test]
fn case_03_to_hex_is_64_lower_case_hexadecimal_digits() {
    let text = Token::draw().expect("the random source").to_hex();
    assert_eq!(text.len(), 64, "{text}");
    assert_eq!(text.chars().count(), 64, "{text}");
    assert!(text.chars().all(is_lower_hex), "{text}");
}

#[test]
fn case_04_from_hex_of_to_hex_matches_the_token() {
    let token = Token::draw().expect("the random source");
    let back = Token::from_hex(&token.to_hex()).expect("its own form");
    assert!(back.matches(&token));
    assert!(token.matches(&back));
    assert_eq!(back.to_hex(), token.to_hex());

    let other = Token::draw().expect("the random source");
    assert!(!back.matches(&other));
}

#[test]
fn case_05_63_digits_refused() {
    assert!(Token::from_hex(&"0".repeat(64)).is_some());
    assert!(Token::from_hex(&"a".repeat(63)).is_none());
}

#[test]
fn case_06_65_digits_refused() {
    assert!(Token::from_hex(&"a".repeat(65)).is_none());
}

#[test]
fn case_07_upper_case_digits_refused() {
    let lower = "0123456789abcdef".repeat(4);
    assert!(Token::from_hex(&lower).is_some());
    assert!(Token::from_hex(&lower.to_uppercase()).is_none());
    let one_upper = format!("{}A", "a".repeat(63));
    assert!(Token::from_hex(&one_upper).is_none());
}

#[test]
fn case_08_a_g_refused() {
    let text = format!("{}g", "0".repeat(63));
    assert!(Token::from_hex(&text).is_none());
    let text = format!("g{}", "0".repeat(63));
    assert!(Token::from_hex(&text).is_none());
}

#[test]
fn case_09_equal_on_two_equal_32_byte_slices_is_true() {
    let a = [0x5au8; 32];
    let b = [0x5au8; 32];
    assert!(equal(&a, &b));
    compare::take_bytes_read();
    assert!(compare::equal(&a, &b));
    assert_eq!(compare::take_bytes_read(), 32);
}

#[test]
fn case_10_slices_differing_only_in_the_first_byte_are_unequal_and_32_bytes_read() {
    let a = [7u8; 32];
    let mut b = a;
    b[0] ^= 1;
    assert!(!equal(&a, &b));
    compare::take_bytes_read();
    assert!(!compare::equal(&a, &b));
    let read = compare::take_bytes_read();
    println!("case 10: {read} bytes read");
    assert_eq!(read, 32);
}

#[test]
fn case_11_slices_differing_only_in_the_last_byte_are_unequal_and_32_bytes_read() {
    let a = [7u8; 32];
    let mut b = a;
    b[31] ^= 1;
    assert!(!equal(&a, &b));
    compare::take_bytes_read();
    assert!(!compare::equal(&a, &b));
    let read = compare::take_bytes_read();
    println!("case 11: {read} bytes read");
    assert_eq!(read, 32);
}

#[test]
fn case_12_slices_of_31_and_32_bytes_are_unequal() {
    let long = [9u8; 32];
    let short = [9u8; 31];
    assert!(!equal(&short, &long));
    assert!(!equal(&long, &short));
    assert!(!equal(&long[..31], &long));
}

#[test]
fn case_13_debug_of_a_token_holds_none_of_its_digits() {
    let token = Token::draw().expect("the random source");
    let hex = token.to_hex();
    for debug in [format!("{token:?}"), format!("{token:#?}")] {
        assert_eq!(debug, "Token(..)");
        assert!(!debug.contains(&hex), "{debug}");
        assert!(!debug.chars().any(|c| c.is_ascii_digit()), "{debug}");
    }
}
