//! The page's files, compiled in from `crates/view/web/` (item 5 of `docs/design/live-view.md`),
//! each with the content type of `docs/view-stream.md`.

use crate::http::PageFile;

const HTML: &str = "text/html; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const JAVASCRIPT: &str = "text/javascript; charset=utf-8";

/// Each file of `crates/view/web/` with its name and content type: `index.html` answers `/`,
/// every other file `/<name>`.
pub fn page_files() -> Vec<PageFile> {
    [
        ("index.html", HTML, include_str!("../web/index.html")),
        ("view.css", CSS, include_str!("../web/view.css")),
        ("view.js", JAVASCRIPT, include_str!("../web/view.js")),
        ("dom.js", JAVASCRIPT, include_str!("../web/dom.js")),
        ("stream.js", JAVASCRIPT, include_str!("../web/stream.js")),
        ("state.js", JAVASCRIPT, include_str!("../web/state.js")),
        (
            "sessions.js",
            JAVASCRIPT,
            include_str!("../web/sessions.js"),
        ),
        ("page.js", JAVASCRIPT, include_str!("../web/page.js")),
        (
            "timeline.js",
            JAVASCRIPT,
            include_str!("../web/timeline.js"),
        ),
    ]
    .into_iter()
    .map(|(name, content_type, text)| PageFile {
        name: name.to_string(),
        content_type: content_type.to_string(),
        bytes: text.as_bytes().to_vec(),
    })
    .collect()
}
