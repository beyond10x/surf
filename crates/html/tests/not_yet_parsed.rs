//! `NOT_YET_PARSED` names parts of tree construction in a form an answer can print. The test
//! holds the form only, not the number or the names of the entries, so that each tree story that
//! removes its entry keeps it passing.

use surf_html::NOT_YET_PARSED;

#[test]
fn every_not_yet_parsed_entry_matches_a_z_and_hyphen() {
    for entry in NOT_YET_PARSED {
        assert!(!entry.is_empty(), "an empty entry");
        assert!(
            entry.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "{entry:?} is not of `a` to `z` and `-`"
        );
    }
}
