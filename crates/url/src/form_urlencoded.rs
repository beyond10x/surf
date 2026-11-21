//! URL Standard, "`application/x-www-form-urlencoded` serializing", with UTF-8 as the encoding.
//! The parser of the format is not written.

use crate::percent::{self, EncodeSet};

/// The `application/x-www-form-urlencoded` serializer: each name and value UTF-8
/// percent-encoded with the `application/x-www-form-urlencoded` percent-encode set (a space
/// becomes `+`), joined as `name=value` pairs with `&`.
pub fn serialize<N: AsRef<str>, V: AsRef<str>>(tuples: &[(N, V)]) -> String {
    let mut output = String::new();
    for (name, value) in tuples {
        if !output.is_empty() {
            output.push('&');
        }
        percent::utf8_percent_encode_into(&mut output, name.as_ref(), EncodeSet::FormUrlencoded);
        output.push('=');
        percent::utf8_percent_encode_into(&mut output, value.as_ref(), EncodeSet::FormUrlencoded);
    }
    output
}
