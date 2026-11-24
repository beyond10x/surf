//! The vendored pages and their `SOURCE`: the digests hold, and each file is named with where it
//! came from.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The vendored files, each with the line of its title (`h1 id="title"`), which names the commit.
const FILES: [(&str, usize); 2] = [("dom.html", 410), ("webidl.html", 531)];

/// The keys `SOURCE` holds for each file.
const KEYS: [&str; 8] = [
    "file",
    "address",
    "commit",
    "last-updated",
    "fetched",
    "licence",
    "copyright",
    "index-lines",
];

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data/whatwg")
}

fn source_text() -> String {
    std::fs::read_to_string(data_dir().join("SOURCE")).expect("read SOURCE")
}

/// The lines of `SOURCE` in the form `sha256sum` writes: 64 hex digits, two spaces, a name.
fn digest_lines(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let Some(digest) = line.get(..64) else {
            continue;
        };
        if digest.bytes().all(|b| b.is_ascii_hexdigit()) && line[64..].starts_with("  ") {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Runs `sha256sum --check --strict` in `dir` over `lines` on its standard input: whether it
/// passed, and what it printed.
fn sha256_check(dir: &Path, lines: &str) -> (bool, String) {
    let mut child = Command::new("sha256sum")
        .args(["--check", "--strict", "-"])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start sha256sum");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(lines.as_bytes())
        .expect("write the digest lines");
    let out = child.wait_with_output().expect("wait for sha256sum");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

/// The blocks of `SOURCE` that start with a `file:` line, each as its key and value pairs, up to
/// the first line that is not `key: value`.
fn file_blocks(source: &str) -> Vec<Vec<(String, String)>> {
    let mut blocks: Vec<Vec<(String, String)>> = Vec::new();
    let mut open = false;
    for line in source.lines() {
        if line.starts_with("file:") {
            blocks.push(Vec::new());
            open = true;
        }
        if !open {
            continue;
        }
        match line.split_once(':') {
            Some((key, value)) if !key.contains(' ') && !key.is_empty() => {
                if let Some(block) = blocks.last_mut() {
                    block.push((key.to_string(), value.trim().to_string()));
                }
            }
            _ => open = false,
        }
    }
    blocks
}

/// The commit a vendored page names in its title line: `Commit <40 hex digits>`.
fn title_commit(file: &str, line: usize) -> String {
    let bytes = std::fs::read(data_dir().join(file)).expect("read the page");
    let text = String::from_utf8(bytes).expect("the page is UTF-8");
    let title = text.lines().nth(line - 1).expect("the title line");
    assert!(
        title.contains("id=\"title\""),
        "{file}:{line} is not the title: {title}"
    );
    let at = title.find("Commit ").expect("the title names a commit") + "Commit ".len();
    let commit: String = title[at..]
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    assert_eq!(commit.len(), 40, "{file}:{line}: {title}");
    commit
}

#[test]
fn sources_match_their_digests() {
    let lines = digest_lines(&source_text());
    assert_eq!(lines.lines().count(), FILES.len(), "{lines}");

    let (passed, text) = sha256_check(&data_dir(), &lines);
    assert!(passed, "{text}");

    let copy = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("webidl-source-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&copy);
    std::fs::create_dir_all(&copy).expect("create the copy");
    for (file, _) in FILES {
        std::fs::copy(data_dir().join(file), copy.join(file)).expect("copy a page");
    }
    let mut dom = std::fs::read(copy.join("dom.html")).expect("read the copy");
    let middle = dom.len() / 2;
    dom[middle] ^= 0x01;
    std::fs::write(copy.join("dom.html"), dom).expect("write the changed copy");

    let (passed, text) = sha256_check(&copy, &lines);
    std::fs::remove_dir_all(&copy).expect("remove the copy");
    assert!(!passed, "{text}");
    assert!(text.contains("dom.html: FAILED"), "{text}");
    assert!(text.contains("webidl.html: OK"), "{text}");
}

#[test]
fn source_names_each_file() {
    let blocks = file_blocks(&source_text());
    let names: Vec<&str> = blocks.iter().map(|block| block[0].1.as_str()).collect();
    assert_eq!(names, ["dom.html", "webidl.html"]);
    for ((file, title_line), block) in FILES.iter().zip(&blocks) {
        let keys: Vec<&str> = block.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(keys, KEYS, "{file}");
        for (key, value) in block {
            assert!(!value.is_empty(), "{file}: {key} is empty");
        }
        let commit = &block[2].1;
        assert_eq!(*commit, title_commit(file, *title_line), "{file}");
    }
}
