//! `entities-table`: writes `crates/html/src/charref_table.rs`, the named character reference
//! table of `surf-html`, from the vendored `crates/html-conformance/data/whatwg/entities.json`
//! ([`entities_table::table_text`]). Run it with `cargo run -p surf-html-conformance --bin
//! entities-table` after the JSON changes; the file is never edited by hand.
//!
//! Exit status: 0 when the file is written, 1 when the JSON cannot be read or turned into the
//! table, or the file cannot be written.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use surf_html_conformance::{entities_table, paths};

fn main() -> ExitCode {
    let json_path = paths::data().join(entities_table::JSON);
    let table_path = paths::root().join(entities_table::TABLE);
    let text = match std::fs::read_to_string(&json_path)
        .map_err(|e| format!("{}: {e}", json_path.display()))
        .and_then(|json| entities_table::table_text(&json))
    {
        Ok(text) => text,
        Err(e) => {
            eprintln!("entities-table: {e}");
            return ExitCode::from(1);
        }
    };
    if let Err(e) = std::fs::write(&table_path, text) {
        eprintln!("entities-table: {}: {e}", table_path.display());
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
