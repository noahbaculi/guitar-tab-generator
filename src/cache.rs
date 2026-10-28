use std::collections::VecDeque;

/// Fixed-capacity call cache that evicts the least recently used entry
///
/// Entries are scanned linearly, which beats hashing at the small capacities the
/// pipeline uses. Callers keep it in a `thread_local!` so lookups never contend.
pub(crate) struct LruCache<K, V> {
    capacity: usize,
    // Front is least recently used
    entries: VecDeque<(K, V)>,
}

impl<K: PartialEq, V: Clone> LruCache<K, V> {
    pub(crate) const fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: VecDeque::new(),
        }
    }

    /// Returns a clone of the cached value and marks it most recently used
    pub(crate) fn get(&mut self, key: &K) -> Option<V> {
        let index = self.entries.iter().position(|(k, _)| k == key)?;
        let entry = self.entries.remove(index)?;
        let value = entry.1.clone();
        self.entries.push_back(entry);
        Some(value)
    }

    /// Stores `value` as most recently used, evicting the oldest entry when full
    pub(crate) fn insert(&mut self, key: K, value: V) {
        if let Some(index) = self.entries.iter().position(|(k, _)| *k == key) {
            self.entries.remove(index);
        } else if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back((key, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_returns_stored_value() {
        let mut cache = LruCache::new(2);
        cache.insert(1, "a");
        assert_eq!(cache.get(&1), Some("a"));
        assert_eq!(cache.get(&2), None);
    }

    #[test]
    fn full_cache_evicts_least_recently_used() {
        let mut cache = LruCache::new(2);
        cache.insert(1, "a");
        cache.insert(2, "b");
        // Touch 1 so 2 becomes the eviction candidate
        cache.get(&1);
        cache.insert(3, "c");
        assert_eq!(cache.get(&1), Some("a"));
        assert_eq!(cache.get(&2), None);
        assert_eq!(cache.get(&3), Some("c"));
    }

    #[test]
    fn reinsert_replaces_without_evicting() {
        let mut cache = LruCache::new(2);
        cache.insert(1, "a");
        cache.insert(2, "b");
        cache.insert(1, "z");
        assert_eq!(cache.get(&1), Some("z"));
        assert_eq!(cache.get(&2), Some("b"));
    }
}
