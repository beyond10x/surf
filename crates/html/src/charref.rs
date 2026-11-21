//! Character references: the lookup of the named character reference state (13.2.5.78) in the
//! table of 13.5 "Named character references", and the replacement table of the numeric
//! character reference end state (13.2.5.84). Written by `story:html-character-references`;
//! the states themselves are the tokenizer's ([`crate::tokenizer`]).
//!
//! The table is `charref_table.rs`, written by the binary `entities-table` of the crate
//! `surf-html-conformance` from the vendored `entities.json` and never edited by hand. It is a
//! portion of the HTML Standard incorporated into source code, so it carries the BSD 3-Clause
//! License notice of the WHATWG in its header (`docs/design/html-parser.md`, item 3).
//!
//! The module has no public item: the tokenizer is its only reader.

use crate::charref_table::NAMED_CHARACTER_REFERENCES;

/// The longest name of the table, in code points after the `&`, `;` included
/// (`&CounterClockwiseContourIntegral;`). The named character reference state looks at no more
/// code points than this.
pub(crate) const LONGEST_NAME: usize = 32;

/// The longest name of the table that `next`, the code points after the `&`, starts with: the
/// number of code points it takes and the one or two code points it stands for. Only the first
/// [`LONGEST_NAME`] code points of `next` are looked at.
pub(crate) fn longest_name(next: &[u32]) -> Option<(usize, &'static [u32])> {
    let mut name = Vec::with_capacity(LONGEST_NAME);
    for &c in next.iter().take(LONGEST_NAME) {
        match u8::try_from(c) {
            Ok(b) if b.is_ascii_alphanumeric() => name.push(b),
            Ok(b';') => {
                name.push(b';');
                break;
            }
            _ => break,
        }
    }
    (1..=name.len()).rev().find_map(|n| {
        NAMED_CHARACTER_REFERENCES
            .binary_search_by(|(entry, _)| entry.as_bytes().cmp(&name[..n]))
            .ok()
            .map(|i| (n, NAMED_CHARACTER_REFERENCES[i].1))
    })
}

/// The code point the numeric character reference end state (13.2.5.84) puts in place of a
/// character reference code of 0x80 to 0x9F, from its table; `None` for a code the table does
/// not list.
pub(crate) fn numeric_replacement(code: u32) -> Option<u32> {
    let replacement = match code {
        0x80 => 0x20AC,
        0x82 => 0x201A,
        0x83 => 0x0192,
        0x84 => 0x201E,
        0x85 => 0x2026,
        0x86 => 0x2020,
        0x87 => 0x2021,
        0x88 => 0x02C6,
        0x89 => 0x2030,
        0x8A => 0x0160,
        0x8B => 0x2039,
        0x8C => 0x0152,
        0x8E => 0x017D,
        0x91 => 0x2018,
        0x92 => 0x2019,
        0x93 => 0x201C,
        0x94 => 0x201D,
        0x95 => 0x2022,
        0x96 => 0x2013,
        0x97 => 0x2014,
        0x98 => 0x02DC,
        0x99 => 0x2122,
        0x9A => 0x0161,
        0x9B => 0x203A,
        0x9C => 0x0153,
        0x9E => 0x017E,
        0x9F => 0x0178,
        _ => return None,
    };
    Some(replacement)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code_points(s: &str) -> Vec<u32> {
        s.chars().map(u32::from).collect()
    }

    #[test]
    fn the_table_is_sorted_and_its_longest_name_is_the_bound() {
        assert!(
            NAMED_CHARACTER_REFERENCES
                .windows(2)
                .all(|pair| pair[0].0.as_bytes() < pair[1].0.as_bytes())
        );
        let longest = NAMED_CHARACTER_REFERENCES
            .iter()
            .map(|(name, _)| name.len())
            .max();
        assert_eq!(longest, Some(LONGEST_NAME));
    }

    #[test]
    fn the_longest_name_wins() {
        assert_eq!(
            longest_name(&code_points("notin;x")),
            Some((6, &[0x2209][..]))
        );
        assert_eq!(
            longest_name(&code_points("notit;")),
            Some((3, &[0x00AC][..]))
        );
        assert_eq!(
            longest_name(&code_points("NotEqualTilde;")),
            Some((14, &[0x2242, 0x0338][..]))
        );
        assert_eq!(longest_name(&code_points("xyz;")), None);
        assert_eq!(longest_name(&code_points("")), None);
        // A name is no longer than the bound, so a run of letters is not read to its end.
        let run = "a".repeat(1000);
        assert_eq!(longest_name(&code_points(&run)), None);
    }

    #[test]
    fn the_numeric_table_replaces_0x80_to_0x9f_but_five() {
        assert_eq!(numeric_replacement(0x80), Some(0x20AC));
        assert_eq!(numeric_replacement(0x9F), Some(0x0178));
        for code in [0x7F, 0x81, 0x8D, 0x8F, 0x90, 0x9D, 0xA0] {
            assert_eq!(numeric_replacement(code), None, "{code:#X}");
        }
    }
}
