//! Adversary cases of `story:values-and-labels-before-fill`, change 2: no path of the name
//! computation gives a password input's value to a label.

use surf_html::{ParseOptions, parse_document};
use surf_page::actionable_elements;

const MARKER: &str = "pw-marker";

/// The labels of every row of `input`.
fn labels(input: &str) -> Vec<String> {
    let tree = parse_document(input, &ParseOptions::default());
    actionable_elements(&tree)
        .into_iter()
        .map(|row| row.label)
        .collect()
}

/// Each way a password input reaches another element's name: embedded in a button, a link and
/// a `label`, nested labels, referenced by `aria-labelledby` (also hidden, and itself carrying
/// `aria-labelledby`, `aria-label`, `title`, `placeholder`), the `type` in upper and mixed case,
/// and the password named by a reference to itself.
#[test]
fn no_password_value_reaches_any_label() {
    let pages = [
        "<button>Code <input type=password value=pw-marker></button>",
        "<button>Code <input type=PASSWORD value=pw-marker></button>",
        "<button>Code <input type=PassWord value=pw-marker></button>",
        "<a href=/x>Code <input type=password value=pw-marker></a>",
        "<button>Code <input type=password value=pw-marker title=Hint \
         placeholder=Hint aria-label=Hint></button>",
        "<label>User <input type=password value=pw-marker> <input type=text></label>",
        "<label for=t>Outer <label>Inner <input type=password value=pw-marker></label></label>\
         <input id=t>",
        "<input id=p type=password value=pw-marker><button aria-labelledby=p>Go</button>",
        "<input id=p type=password value=pw-marker hidden><button aria-labelledby=p>Go</button>",
        "<span id=q>Q</span><input id=p type=password value=pw-marker aria-labelledby=q>\
         <button aria-labelledby=p>Go</button>",
        "<input id=p type=password value=pw-marker aria-labelledby=p>",
        "<div id=d>Code <input type=password value=pw-marker></div>\
         <input type=text aria-labelledby=d>",
    ];
    for page in pages {
        let labels = labels(page);
        assert!(!labels.is_empty(), "{page}: no row");
        assert!(
            !labels.iter().any(|label| label.contains(MARKER)),
            "{page}: {labels:?}"
        );
    }
}
