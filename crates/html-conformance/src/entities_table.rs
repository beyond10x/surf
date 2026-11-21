//! The named character reference table of `surf-html`, `crates/html/src/charref_table.rs`,
//! written from the vendored `data/whatwg/entities.json` (13.5 "Named character references").
//!
//! [`table_text`] turns the JSON into the file's text: a header that is the BSD 3-Clause
//! License notice for the WHATWG portion (the WHATWG IPR policy licenses the parts of a Living
//! Standard incorporated into source code under it), then every name without its `&`, sorted
//! by its bytes, with its code points. The binary `entities-table` writes it; the test
//! `tests/entities_table.rs` holds the committed file to it byte for byte. The file is never
//! edited by hand.

use crate::json::{self, Value};

/// The vendored JSON, relative to `data/`.
pub const JSON: &str = "whatwg/entities.json";

/// The generated table, relative to the repository root.
pub const TABLE: &str = "crates/html/src/charref_table.rs";

/// The copyright line of the HTML Standard's repository licence, the first line of the table.
pub const COPYRIGHT: &str = "Copyright © WHATWG (Apple, Google, Mozilla, Microsoft).";

/// The BSD 3-Clause License text after the copyright line: the permission, the three
/// conditions and the disclaimer.
pub const BSD_3_CLAUSE: &str = "\
Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS \"AS IS\"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
";

/// One name of the table: the name without its `&`, and its one or two code points.
struct Entry {
    name: String,
    code_points: Vec<u32>,
}

/// The text of `crates/html/src/charref_table.rs` for the JSON `input`. Fails when the JSON is
/// not an object of names, each `&`, ASCII alphanumerics and an optional final `;`, whose
/// `codepoints` are one or two Unicode scalar values, or when a name repeats.
pub fn table_text(input: &str) -> Result<String, String> {
    let Value::Object(members) = json::parse(input).map_err(|e| e.to_string())? else {
        return Err("entities.json: not an object".to_string());
    };
    let mut entries = Vec::with_capacity(members.len());
    for (key, value) in &members {
        let key: String = key
            .iter()
            .map(|&c| char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect();
        entries.push(entry(&key, value).map_err(|e| format!("entities.json: {key:?}: {e}"))?);
    }
    entries.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    if let Some(pair) = entries.windows(2).find(|pair| pair[0].name == pair[1].name) {
        return Err(format!("entities.json: {:?} twice", pair[0].name));
    }

    let mut out = String::new();
    out.push_str("// ");
    out.push_str(COPYRIGHT);
    out.push_str("\n//\n");
    for line in BSD_3_CLAUSE.lines() {
        if line.is_empty() {
            out.push_str("//\n");
        } else {
            out.push_str("// ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str(
        "\n\
         //! The named character references of 13.5 \"Named character references\" of the HTML\n\
         //! Standard: each name without its `&`, sorted by its bytes, with its one or two code\n\
         //! points. A portion of the standard incorporated into source code, under the BSD 3-Clause\n\
         //! License above (WHATWG IPR policy, <https://whatwg.org/ipr-policy>).\n\
         //!\n\
         //! Written by `cargo run -p surf-html-conformance --bin entities-table` from\n\
         //! `crates/html-conformance/data/whatwg/entities.json`\n\
         //! (<https://html.spec.whatwg.org/entities.json>). Never edited by hand.\n\
         \n",
    );
    out.push_str(&format!(
        "/// The {} names of the table, sorted by their bytes.\n",
        entries.len()
    ));
    out.push_str("pub(crate) static NAMED_CHARACTER_REFERENCES: &[(&str, &[u32])] = &[\n");
    for entry in &entries {
        let code_points: Vec<String> = entry
            .code_points
            .iter()
            .map(|c| format!("0x{c:04X}"))
            .collect();
        out.push_str(&format!(
            "    (\"{}\", &[{}]),\n",
            entry.name,
            code_points.join(", ")
        ));
    }
    out.push_str("];\n");
    Ok(out)
}

fn entry(key: &str, value: &Value) -> Result<Entry, String> {
    let name = key.strip_prefix('&').ok_or("does not start with `&`")?;
    let letters = name.strip_suffix(';').unwrap_or(name);
    if letters.is_empty() || !letters.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err("not ASCII alphanumerics and an optional final `;`".to_string());
    }
    let Some(Value::Array(numbers)) = value.get("codepoints") else {
        return Err("no `codepoints` array".to_string());
    };
    if numbers.is_empty() || numbers.len() > 2 {
        return Err(format!("{} code points", numbers.len()));
    }
    let mut code_points = Vec::with_capacity(numbers.len());
    for number in numbers {
        let Value::Number(digits) = number else {
            return Err(format!("a code point that is {}", number.kind()));
        };
        let c: u32 = digits
            .parse()
            .map_err(|_| format!("the code point {digits} is not a whole number"))?;
        if char::from_u32(c).is_none() {
            return Err(format!(
                "the code point {digits} is no Unicode scalar value"
            ));
        }
        code_points.push(c);
    }
    Ok(Entry {
        name: name.to_string(),
        code_points,
    })
}
