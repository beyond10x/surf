//! The conformance harness of `surf-html`: it reads the parser test data vendored under
//! `data/`, html5lib-tests (`tokenizer/*.test`, `encoding/`) and web-platform-tests
//! (`html/syntax/parsing/resources/*.dat`), each pinned and recorded in its `SOURCE`, runs every
//! case against `surf-html`, and counts per data file and per tag the cases that exist, run and
//! pass. What a case is, what is compared and how cases are tagged is
//! `docs/design/html-parser.md` (items 3 and 8); the files and their counts are its
//! `html-inventory` block, which [`inventory`] reads.
//!
//! Standard library and `surf-html` only: the JSON reader ([`json`]), the `.dat` reader
//! ([`dat`]) and the `| ` tree format ([`dump`]) are written here.
//!
//! The binary prints one line per data file, `file <path> <pass>/<run>/<exist>` followed by
//! `not-run:<reason>=<n>` for each reason cases of that file are not run, one line per tag,
//! `tag <name> <pass>/<run>/<exist>`, in the order of the `tag` lines of the inventory, and the
//! total, `html-conformance: <exist> cases, <run> run, <pass> passed`. With
//! `--list-failing <tag>` it prints instead `<path> <case number>` for each case of that tag
//! that runs and does not pass.
//!
//! The expected counts are held in `expected/<group>.txt`, one file per group of the
//! inventory (`tokenizer`, `tree-construction`, `encoding`): the `file` lines of that group's
//! data files as the binary prints them, one `tag <name> <pass>/<run>/<exist>` line per tag
//! that has a case in that group, counted over that group's cases, and one `section <name>
//! complete` or `section <name> incomplete` line per section of the note's `html-sections`
//! block whose data files are in that group. `--check --base <revision>` ([`check`]) fails
//! when a count drops below them, when one of them is below its value at `<revision>`, or when
//! a `complete` mark is not allowed; `--root <dir>` reads another repository.
//!
//! `data/whatwg/entities.json`, the table of 13.5 "Named character references" (CC BY 4.0, see
//! its `SOURCE`), holds no case. The second binary, `entities-table`, writes from it the named
//! reference table of `surf-html`, `crates/html/src/charref_table.rs` ([`entities_table`]);
//! `cargo run -p surf-html-conformance` runs the harness (`default-run`).

#![forbid(unsafe_code)]

pub mod cases;
pub mod check;
pub mod dat;
pub mod dump;
pub mod entities_table;
pub mod inventory;
pub mod json;
pub mod runner;
pub mod tags;

/// Where the harness finds the data and the note.
pub mod paths {
    use std::path::{Path, PathBuf};

    use crate::inventory::Inventory;

    /// The crate's directory.
    pub fn crate_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// The repository root, two directories above the crate's.
    pub fn root() -> PathBuf {
        crate_dir().join("../..")
    }

    /// `crates/html-conformance/data/`, the root every data path is relative to.
    pub fn data() -> PathBuf {
        crate_dir().join("data")
    }

    /// `docs/design/html-parser.md`, which holds the `html-inventory` block.
    pub fn note() -> PathBuf {
        crate_dir().join("../../docs/design/html-parser.md")
    }

    /// The files under `data/<source>/` for each `source` line of the inventory, as paths
    /// relative to `data/`: every file but the source's own `LICENSE`, `LICENSE.md` and
    /// `SOURCE`.
    pub fn data_files(data: &Path, inventory: &Inventory) -> std::io::Result<Vec<String>> {
        let mut out = Vec::new();
        for source in &inventory.sources {
            walk(data, &data.join(&source.name), &mut out)?;
        }
        out.retain(|path| {
            let rest = path.split_once('/').map_or("", |(_, rest)| rest);
            !matches!(rest, "LICENSE" | "LICENSE.md" | "SOURCE")
        });
        Ok(out)
    }

    fn walk(data: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(data, &path, out)?;
            } else {
                let relative = path.strip_prefix(data).unwrap_or(&path);
                let parts: Vec<String> = relative
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                out.push(parts.join("/"));
            }
        }
        Ok(())
    }
}

/// Undoes the second escaping of a `doubleEscaped` tokenizer test: each `\uXXXX` becomes the
/// code point, a high and a low surrogate escape in a row become one code point, and a lone
/// surrogate escape stays a surrogate. A backslash that does not start `\u` and four hex
/// digits is an error.
pub fn unescape(input: &[u32]) -> Result<Vec<u32>, String> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] != u32::from('\\') {
            out.push(input[i]);
            i += 1;
            continue;
        }
        let hex = input
            .get(i + 1..i + 6)
            .filter(|s| s[0] == u32::from('u'))
            .and_then(|s| {
                s[1..].iter().try_fold(0u32, |acc, &c| {
                    char::from_u32(c)?.to_digit(16).map(|d| acc * 16 + d)
                })
            })
            .ok_or_else(|| "a backslash that does not start \\uXXXX".to_string())?;
        out.push(hex);
        json::join_surrogate_pair(&mut out);
        i += 6;
    }
    Ok(out)
}
