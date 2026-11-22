//! One in-memory table of snapshots, keyed by each snapshot's identity.

use std::collections::HashMap;

/// A snapshot that carries its own identity.
pub(crate) trait Keyed {
    /// The identity type.
    type Key: PartialEq;

    /// This snapshot's identity.
    fn key(&self) -> &Self::Key;

    /// The text of an identity, the UUID every identity type wraps: the key of the table's
    /// index.
    fn text(key: &Self::Key) -> &str;
}

/// Rows in insertion order. A `put` under an identity already held replaces that row where it
/// stands.
///
/// An index from each identity's text to its row makes `get` and `put` cost one hash lookup, not
/// a scan of the table: the parser step adds one element row per actionable element of a page,
/// so a scan per `put` made a page of n elements cost n² comparisons
/// (`story:html-document-port`, fix round). `delete` rebuilds the index.
#[derive(Debug)]
pub(crate) struct Rows<S> {
    rows: Vec<S>,
    index: HashMap<String, usize>,
}

impl<S> Default for Rows<S> {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            index: HashMap::new(),
        }
    }
}

impl<S: Keyed + Clone> Rows<S> {
    pub(crate) fn get(&self, identity: &S::Key) -> Option<S> {
        let at = *self.index.get(S::text(identity))?;
        self.rows.get(at).cloned()
    }

    pub(crate) fn put(&mut self, snapshot: S) {
        let text = S::text(snapshot.key());
        match self.index.get(text) {
            Some(&at) => self.rows[at] = snapshot,
            None => {
                self.index.insert(text.to_owned(), self.rows.len());
                self.rows.push(snapshot);
            }
        }
    }

    pub(crate) fn delete(&mut self, identity: &S::Key) {
        if self.index.remove(S::text(identity)).is_none() {
            return;
        }
        self.rows.retain(|row| row.key() != identity);
        self.index = self
            .rows
            .iter()
            .enumerate()
            .map(|(at, row)| (S::text(row.key()).to_owned(), at))
            .collect();
    }

    pub(crate) fn list(&self) -> Vec<S> {
        self.rows.clone()
    }
}
