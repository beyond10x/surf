//! `bindings [--selection <file>] [--without-html] [<out>]`: writes the DOM bindings of the
//! script engine (`story:webidl-bindings`) to `<out>`, by default
//! `crates/script/src/bindings.rs`, from the vendored files of `crates/webidl/data/whatwg/` and
//! the selection `crates/webidl/selection.txt` (or `<file>`). The header names the sha256 of
//! each source, computed with `sha256sum`, among them `rust-toolchain.toml`, which pins the
//! `rustfmt` that formats the text; after `rustfmt`, its last line gets the sha256 of the lines
//! after it (`surf_webidl::BODY_DIGEST_LINE`). The time of each phase goes to standard error.
//!
//! `--without-html` reads the DOM Standard and the Web IDL Standard only, for timing the
//! generator before the HTML Standard is added; a selection that names `Window` then fails.
//!
//! `bindings --cut <snapshot> <out>` writes the cut of a page of the HTML Standard to `<out>`
//! and nothing else: its blocks of Web IDL, byte for byte ([`cut_idl_blocks`]), the form in
//! which `crates/webidl/data/whatwg/html-idl.html` holds the snapshot `SOURCE` names
//! (`task webidl-html-cut`).

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

use surf_webidl::{
    Digest, Model, Selection, bindings_model, cut_idl_blocks, fill_body_digest, generate, read_page,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("bindings: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|error| format!("the repository root: {error}"))?;
    let mut selection_path = PathBuf::from("crates/webidl/selection.txt");
    let mut out = PathBuf::from("crates/script/src/bindings.rs");
    let mut with_html = true;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cut" => {
                let (Some(snapshot), Some(cut), None) = (args.next(), args.next(), args.next())
                else {
                    return Err("usage: bindings --cut <snapshot> <out>".to_string());
                };
                let page =
                    std::fs::read(&snapshot).map_err(|error| format!("{snapshot}: {error}"))?;
                let blocks = cut_idl_blocks(&page);
                std::fs::write(&cut, &blocks).map_err(|error| format!("{cut}: {error}"))?;
                eprintln!(
                    "bindings: cut {snapshot} into {cut} ({} bytes)",
                    blocks.len()
                );
                return Ok(());
            }
            "--selection" => {
                selection_path = PathBuf::from(args.next().ok_or("--selection needs a file")?);
            }
            "--without-html" => with_html = false,
            _ if arg.starts_with('-') => return Err(format!("unknown option `{arg}`")),
            _ => out = PathBuf::from(arg),
        }
    }
    let selection_path = absolute(&root, &selection_path);
    let out = absolute(&root, &out);
    let data = root.join("crates/webidl/data/whatwg");
    let read =
        |path: &Path| std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()));

    let started = Instant::now();
    let selection_text = String::from_utf8(read(&selection_path)?)
        .map_err(|_| format!("{}: not UTF-8", selection_path.display()))?;
    let label = relative(&root, &selection_path);
    let selection = Selection::read(&selection_text, &label).map_err(|error| error.to_string())?;
    let dom = read(&data.join("dom.html"))?;
    let webidl = read(&data.join("webidl.html"))?;
    let model = if with_html {
        let html = read(&data.join("html-idl.html"))?;
        let (model, extracted) =
            bindings_model(&dom, &webidl, &html, &selection).map_err(|error| error.to_string())?;
        eprintln!(
            "bindings: html-idl.html: {} blocks of Web IDL, {} taken, {} parts skipped",
            extracted.blocks,
            extracted.taken,
            extracted.skipped.len()
        );
        model
    } else {
        let mut definitions = read_page(&dom, "dom.html")
            .map_err(|error| error.to_string())?
            .definitions;
        definitions.extend(
            read_page(&webidl, "webidl.html")
                .map_err(|error| error.to_string())?
                .definitions,
        );
        Model::new(definitions).map_err(|error| error.to_string())?
    };
    let read_time = started.elapsed();

    let mut sources = vec![
        "crates/webidl/data/whatwg/dom.html".to_string(),
        "crates/webidl/data/whatwg/webidl.html".to_string(),
    ];
    if with_html {
        sources.push("crates/webidl/data/whatwg/html-idl.html".to_string());
    }
    sources.push(label);
    sources.push("crates/webidl/Cargo.toml".to_string());
    // It pins the toolchain, so the version of the `rustfmt` that formats the file.
    sources.push("rust-toolchain.toml".to_string());
    let mut code = Vec::new();
    rust_files(&root.join("crates/webidl/src"), &mut code)?;
    code.sort();
    sources.extend(code.iter().map(|path| relative(&root, path)));
    let digests = sha256(&root, &sources)?;

    let started_generate = Instant::now();
    let text = generate(&model, &selection, &digests).map_err(|error| error.to_string())?;
    let generate_time = started_generate.elapsed();

    let started_format = Instant::now();
    let formatted = rustfmt(&text)?;
    let format_time = started_format.elapsed();
    let formatted = fill_body_digest(&formatted, sha256_of)
        .ok_or("the generated text holds no line of the body's digest")??;
    std::fs::write(&out, &formatted).map_err(|error| format!("{}: {error}", out.display()))?;
    eprintln!(
        "bindings: wrote {} ({} lines, {} bytes); read {:.3} s, generate {:.3} s, rustfmt {:.3} s",
        relative(&root, &out),
        formatted.lines().count(),
        formatted.len(),
        read_time.as_secs_f64(),
        generate_time.as_secs_f64(),
        format_time.as_secs_f64()
    );
    Ok(())
}

fn absolute(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|dir| dir.join(path))
            .ok()
            .filter(|joined| joined.exists() || joined.parent().is_some_and(Path::exists))
            .unwrap_or_else(|| root.join(path))
    }
}

/// `path` from the repository root, `/`-separated, or as given when it is outside it.
fn relative(root: &Path, path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    canonical
        .strip_prefix(root)
        .map(|inner| {
            inner
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_else(|_| path.display().to_string())
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("{}: {error}", dir.display()))?
            .path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// The sha256 of each of `paths`, from `root`, by `sha256sum`.
fn sha256(root: &Path, paths: &[String]) -> Result<Vec<Digest>, String> {
    let output = Command::new("sha256sum")
        .arg("--")
        .args(paths)
        .current_dir(root)
        .output()
        .map_err(|error| format!("sha256sum: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "sha256sum: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8(output.stdout).map_err(|_| "sha256sum: not UTF-8")?;
    let digests: Vec<Digest> = text
        .lines()
        .filter_map(|line| line.split_once("  "))
        .map(|(sha256, path)| Digest {
            sha256: sha256.to_string(),
            path: path.to_string(),
        })
        .collect();
    if digests.len() != paths.len() {
        return Err(format!(
            "sha256sum: {} digests for {} files",
            digests.len(),
            paths.len()
        ));
    }
    Ok(digests)
}

/// The sha256 of `bytes`, by `sha256sum` on its standard input: 64 lowercase hex digits.
fn sha256_of(bytes: &[u8]) -> Result<String, String> {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("sha256sum: {error}"))?;
    let mut stdin = child.stdin.take().ok_or("sha256sum: no stdin")?;
    let input = bytes.to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let output = child
        .wait_with_output()
        .map_err(|error| format!("sha256sum: {error}"))?;
    writer
        .join()
        .map_err(|_| "sha256sum: the writer panicked")?
        .map_err(|error| format!("sha256sum: {error}"))?;
    let digest: String = String::from_utf8_lossy(&output.stdout)
        .chars()
        .take(64)
        .collect();
    if !output.status.success()
        || digest.len() != 64
        || !digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(format!(
            "sha256sum: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(digest)
}

/// `text` formatted by `rustfmt --edition 2024`.
fn rustfmt(text: &str) -> Result<String, String> {
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("rustfmt: {error}"))?;
    let mut stdin = child.stdin.take().ok_or("rustfmt: no stdin")?;
    let input = text.to_string();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let output = child
        .wait_with_output()
        .map_err(|error| format!("rustfmt: {error}"))?;
    writer
        .join()
        .map_err(|_| "rustfmt: the writer panicked")?
        .map_err(|error| format!("rustfmt: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "rustfmt: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|_| "rustfmt: not UTF-8".to_string())
}
