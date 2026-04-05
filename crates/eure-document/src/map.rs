use indexmap::IndexMap;

use crate::{prelude_internal::*, value::PartialObjectKey};

#[derive(Debug, Clone, Plural)]
#[plural(len, is_empty, iter, into_iter, into_iter_ref, new)]
pub struct Map<K, V>(IndexMap<K, V>);

impl<K: Eq + std::hash::Hash, V: Eq> Eq for Map<K, V> {}
impl<K: Eq + std::hash::Hash, V: PartialEq> PartialEq for Map<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K: Eq + std::hash::Hash, V> FromIterator<(K, V)> for Map<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(IndexMap::from_iter(iter))
    }
}

impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Self(IndexMap::new())
    }
}

impl<K: std::hash::Hash + Eq, V> Map<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.0.insert(key, value)
    }

    /// O(1) removal, may reorder remaining keys.
    /// Use when order doesn't matter (e.g., batch processing).
    pub fn remove_fast(&mut self, key: &K) -> Option<V> {
        self.0.swap_remove(key)
    }

    /// O(n) removal, preserves document order.
    /// Use when order must be maintained.
    pub fn remove_ordered(&mut self, key: &K) -> Option<V> {
        self.0.shift_remove(key)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.0.contains_key(key)
    }
}

impl Map<ObjectKey, NodeId> {
    pub fn add(&mut self, key: ObjectKey, value: NodeId) -> Result<(), InsertErrorKind> {
        match self.0.entry(key) {
            indexmap::map::Entry::Occupied(e) => Err(InsertErrorKind::AlreadyAssigned {
                key: e.key().clone(),
            }),
            indexmap::map::Entry::Vacant(e) => {
                e.insert(value);
                Ok(())
            }
        }
    }

    pub fn get_node_id(&self, key: &ObjectKey) -> Option<NodeId> {
        self.0.get(key).copied()
    }
}

/// A map whose keys may contain holes (unresolved placeholders).
///
/// Backed by a `Vec` rather than `IndexMap` because hole keys are not
/// unconditionally deduplicated: anonymous holes (`!`) always create new
/// entries. Labeled holes (`!label`) and resolved keys are deduplicated
/// by value on lookup, matching the behavior of regular `Map`.
///
/// Use [`PartialMap::find`] to look up an existing entry before inserting.
#[derive(Debug, Clone, Default)]
pub struct PartialMap<V>(Vec<(PartialObjectKey, V)>);

pub type PartialNodeMap = PartialMap<NodeId>;

impl<V: PartialEq> PartialEq for PartialMap<V> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<V> PartialMap<V> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Append a (key, value) pair unconditionally.
    /// Callers should call [`Self::find`] first to avoid duplicate labeled entries.
    pub fn push(&mut self, key: PartialObjectKey, value: V) {
        self.0.push((key, value));
    }

    /// Find the first entry matching `key`.
    ///
    /// Keys containing anonymous holes never match — they are always unique.
    pub fn find(&self, key: &PartialObjectKey) -> Option<&V> {
        if key.contains_anonymous_hole() {
            return None;
        }
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PartialObjectKey, &V)> {
        self.0.iter().map(|(k, v)| (k, v))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
