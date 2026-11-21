//! The `application/x-www-form-urlencoded` serializer (URL Standard,
//! "application/x-www-form-urlencoded serializing"), one case per code point the story names.

use surf_url::form_urlencoded::serialize;

#[test]
fn space_becomes_plus() {
    assert_eq!(serialize(&[("a b", "c d")]), "a+b=c+d");
}

#[test]
fn ampersand_is_percent_encoded() {
    assert_eq!(serialize(&[("a&b", "&")]), "a%26b=%26");
}

#[test]
fn equals_sign_is_percent_encoded() {
    assert_eq!(serialize(&[("a=b", "=")]), "a%3Db=%3D");
}

#[test]
fn plus_is_percent_encoded() {
    assert_eq!(serialize(&[("a+b", "+")]), "a%2Bb=%2B");
}

#[test]
fn asterisk_stays() {
    assert_eq!(serialize(&[("a*b", "*")]), "a*b=*");
}

#[test]
fn bytes_outside_ascii_are_utf8_percent_encoded() {
    assert_eq!(
        serialize(&[("é", "≡"), ("\u{1F4A9}", "")]),
        "%C3%A9=%E2%89%A1&%F0%9F%92%A9="
    );
}

#[test]
fn pairs_are_joined_with_ampersand_in_order() {
    assert_eq!(serialize(&[("b", "2"), ("a", "1"), ("", "")]), "b=2&a=1&=");
}
