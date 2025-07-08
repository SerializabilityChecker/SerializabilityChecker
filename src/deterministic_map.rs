use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap as StdHashMap, HashSet as StdHashSet};
use std::hash::BuildHasherDefault;

pub type DeterministicHasher = BuildHasherDefault<DefaultHasher>;
pub type HashMap<K, V> = StdHashMap<K, V, DeterministicHasher>;
pub type HashSet<T> = StdHashSet<T, DeterministicHasher>;

// Helper functions to create new instances
pub fn hashmap_new<K, V>() -> HashMap<K, V> {
    HashMap::default()
}

pub fn hashset_new<T>() -> HashSet<T> {
    HashSet::default()
}
