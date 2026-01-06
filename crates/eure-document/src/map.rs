use indexmap::IndexMap;

use crate::prelude_internal::*;

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
