//! The files of the cases: per case `<case>/chain.pem` (the leaf first), `<case>/root.pem` and
//! `<case>/leaf.pk8`, and `INDEX`, one line per case.

use std::fs;
use std::path::Path;

use crate::cases::Case;
use crate::{Error, VALIDATION_TIME, pem};

/// The line of `INDEX` for a case: name, host, validation time, outcome.
pub fn index_line(case: &Case) -> String {
    format!(
        "{} {} {VALIDATION_TIME} {}\n",
        case.spec.name,
        case.spec.host,
        case.spec.outcome.index_text()
    )
}

/// Every file of `cases`, by path relative to the output directory, in the order written.
pub fn files(cases: &[Case]) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut index = String::new();
    for case in cases {
        let name = case.spec.name;
        let chain: String = case.chain.iter().map(|der| pem::certificate(der)).collect();
        out.push((format!("{name}/chain.pem"), chain.into_bytes()));
        out.push((
            format!("{name}/root.pem"),
            pem::certificate(&case.root).into_bytes(),
        ));
        out.push((format!("{name}/leaf.pk8"), case.leaf_key.clone()));
        index.push_str(&index_line(case));
    }
    out.push(("INDEX".into(), index.into_bytes()));
    out
}

/// Writes `files` under `dir`, making the directories.
pub fn write(dir: &Path, files: &[(String, Vec<u8>)]) -> Result<(), Error> {
    for (path, bytes) in files {
        let path = dir.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error(format!("{}: {e}", parent.display())))?;
        }
        fs::write(&path, bytes).map_err(|e| Error(format!("{}: {e}", path.display())))?;
    }
    Ok(())
}
