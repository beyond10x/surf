//! The runner's values and the generated types' values, both ways.
//!
//! A suite carries a value as an ESS [`Node`]: a `Uuid` and every newtype over one as its text, a
//! `String` as text, an `Integer` as a number, a `Boolean` as a boolean, `Bytes` as padded
//! standard base64 text (RFC 4648, section 4), and an enum as the name of its variant.

use std::collections::BTreeMap;

use ess_conformance::target::TargetError;
use ess_primitives::facts::Number;
use ess_primitives::node::Node;
use surf_types::primitives::Uuid;

/// The value of an `Integer`.
pub fn integer(value: i64) -> Node {
    Node::Number(Number::from(value))
}

/// The value of a `Boolean`.
pub fn boolean(value: bool) -> Node {
    Node::Bool(value)
}

/// The value of a `String`, a `Uuid` or an enum variant.
pub fn text(value: &str) -> Node {
    Node::Text(value.to_owned())
}

/// The value of a `Uuid` or a newtype over one.
pub fn uuid(value: &Uuid) -> Node {
    text(&value.0)
}

/// The value of `Bytes`: padded standard base64.
pub fn bytes(value: &[u8]) -> Node {
    Node::Text(encode(value))
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` as padded standard base64.
pub fn encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let group = (u32::from(chunk[0]) << 16)
            | (u32::from(chunk.get(1).copied().unwrap_or(0)) << 8)
            | u32::from(chunk.get(2).copied().unwrap_or(0));
        for position in 0..4 {
            if position <= chunk.len() {
                let index = (group >> (18 - 6 * position)) & 0x3f;
                text.push(char::from(ALPHABET[index as usize]));
            } else {
                text.push('=');
            }
        }
    }
    text
}

/// Padded standard base64 as bytes; `None` for any other text, including padding bits that are
/// not zero.
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let input = text.as_bytes();
    if !input.len().is_multiple_of(4) {
        return None;
    }
    let mut bytes = Vec::with_capacity(input.len() / 4 * 3);
    let groups = input.len() / 4;
    for (number, group) in input.chunks(4).enumerate() {
        let padding = group.iter().rev().take_while(|&&byte| byte == b'=').count();
        if padding > 2 || (padding > 0 && number + 1 != groups) {
            return None;
        }
        let mut value = 0u32;
        for &byte in &group[..4 - padding] {
            let index = ALPHABET.iter().position(|&letter| letter == byte)?;
            value = (value << 6) | index as u32;
        }
        value <<= 6 * padding as u32;
        let decoded = [(value >> 16) as u8, (value >> 8) as u8, value as u8];
        let kept = 3 - padding;
        if decoded[kept..].iter().any(|&byte| byte != 0) {
            return None;
        }
        bytes.extend_from_slice(&decoded[..kept]);
    }
    Some(bytes)
}

/// The input of one command, read field by field. A field it lacks, a value of another type and a
/// field the command does not declare are each an adapter failure naming the field.
pub struct Input<'a> {
    command: &'a str,
    fields: &'a BTreeMap<String, Node>,
    read: Vec<&'static str>,
}

impl<'a> Input<'a> {
    /// The input `fields` of `command`.
    pub fn new(command: &'a str, fields: &'a BTreeMap<String, Node>) -> Self {
        Self {
            command,
            fields,
            read: Vec::new(),
        }
    }

    fn field(&mut self, name: &'static str, kind: &str) -> Result<&'a Node, TargetError> {
        self.read.push(name);
        self.fields.get(name).ok_or_else(|| {
            TargetError::unavailable(
                format!("reading the input of `{}`", self.command),
                format!("`{name}` ({kind}) is missing"),
            )
        })
    }

    fn wrong(&self, name: &str, kind: &str, value: &Node) -> TargetError {
        TargetError::unavailable(
            format!("reading the input of `{}`", self.command),
            format!(
                "`{name}` is not {kind}: {}",
                ess_conformance::report::quote(value)
            ),
        )
    }

    /// A `String`, `Uuid` or enum field.
    pub fn text(&mut self, name: &'static str) -> Result<String, TargetError> {
        match self.field(name, "text")? {
            Node::Text(value) => Ok(value.clone()),
            other => Err(self.wrong(name, "text", other)),
        }
    }

    /// A `Uuid` field.
    pub fn uuid(&mut self, name: &'static str) -> Result<Uuid, TargetError> {
        self.text(name).map(Uuid)
    }

    /// An `Integer` field.
    pub fn integer(&mut self, name: &'static str) -> Result<i64, TargetError> {
        let value = self.field(name, "an integer")?;
        match value {
            Node::Number(number) => number
                .as_i64()
                .ok_or_else(|| self.wrong(name, "an integer", value)),
            other => Err(self.wrong(name, "an integer", other)),
        }
    }

    /// A `Boolean` field.
    pub fn boolean(&mut self, name: &'static str) -> Result<bool, TargetError> {
        match self.field(name, "a boolean")? {
            Node::Bool(value) => Ok(*value),
            other => Err(self.wrong(name, "a boolean", other)),
        }
    }

    /// A `Bytes` field.
    pub fn bytes(&mut self, name: &'static str) -> Result<Vec<u8>, TargetError> {
        let value = self.field(name, "padded base64 text")?;
        match value {
            Node::Text(text) => {
                decode(text).ok_or_else(|| self.wrong(name, "padded base64 text", value))
            }
            other => Err(self.wrong(name, "padded base64 text", other)),
        }
    }

    /// An enum field, read by `variant` from the name of its variant.
    pub fn variant<T>(
        &mut self,
        name: &'static str,
        variant: fn(&str) -> Option<T>,
    ) -> Result<T, TargetError> {
        let value = self.field(name, "a variant name")?;
        match value {
            Node::Text(text) => variant(text).ok_or_else(|| self.wrong(name, "a variant", value)),
            other => Err(self.wrong(name, "a variant name", other)),
        }
    }

    /// Fails naming every field the command does not declare.
    pub fn finish(self) -> Result<(), TargetError> {
        let undeclared: Vec<&str> = self
            .fields
            .keys()
            .map(String::as_str)
            .filter(|field| !self.read.contains(field))
            .collect();
        if undeclared.is_empty() {
            Ok(())
        } else {
            Err(TargetError::unavailable(
                format!("reading the input of `{}`", self.command),
                format!("undeclared field(s): {}", undeclared.join(", ")),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    #[test]
    fn base64_round_trips_every_length_and_matches_rfc_4648_vectors() {
        for (plain, coded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(plain.as_bytes()), coded);
            assert_eq!(decode(coded), Some(plain.as_bytes().to_vec()));
        }
        assert_eq!(decode("AA=="), Some(vec![0]));
        assert_eq!(
            decode("PGh0bWw+PC9odG1sPg=="),
            Some(b"<html></html>".to_vec())
        );
        let every: Vec<u8> = (0..=255).collect();
        assert_eq!(decode(&encode(&every)), Some(every));
    }

    #[test]
    fn base64_refuses_what_is_not_padded_standard_base64() {
        for text in [
            "A", "AA", "AA=", "A===", "AA==AA==", "AB==", "Zm9-", "Zm9_", "Zg= =",
        ] {
            assert_eq!(decode(text), None, "{text:?}");
        }
    }
}
