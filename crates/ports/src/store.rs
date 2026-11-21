//! One in-memory table of snapshots, keyed by each snapshot's identity.

/// A snapshot that carries its own identity.
pub(crate) trait Keyed {
    /// The identity type.
    type Key: PartialEq;

    /// This snapshot's identity.
    fn key(&self) -> &Self::Key;
}

/// Rows in insertion order. A `put` under an identity already held replaces that row where it
/// stands.
#[derive(Debug)]
pub(crate) struct Rows<S> {
    rows: Vec<S>,
}

impl<S> Default for Rows<S> {
    fn default() -> Self {
        Self { rows: Vec::new() }
    }
}

impl<S: Keyed + Clone> Rows<S> {
    pub(crate) fn get(&self, identity: &S::Key) -> Option<S> {
        self.rows.iter().find(|row| row.key() == identity).cloned()
    }

    pub(crate) fn put(&mut self, snapshot: S) {
        match self.rows.iter_mut().find(|row| row.key() == snapshot.key()) {
            Some(held) => *held = snapshot,
            None => self.rows.push(snapshot),
        }
    }

    pub(crate) fn delete(&mut self, identity: &S::Key) {
        self.rows.retain(|row| row.key() != identity);
    }

    pub(crate) fn list(&self) -> Vec<S> {
        self.rows.clone()
    }
}
