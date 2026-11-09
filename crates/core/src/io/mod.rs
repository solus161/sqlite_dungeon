pub mod file_source;
mod lru_cache;
pub mod mem_source;
pub mod page_source;

/// A trait for page caching
pub trait PageCache {
    /// Return the cached page and mark it as most recently used
    /// A miss leaves the list untouched
    fn get(&mut self, k: &u64) -> Option<&[u8]>; //{
        // if !self.linked_list.nodes.contains_key(k) {
        //     return None;
        // };
        // self.linked_list.touch(*k)
    // }

    /// Add a new key - value to cache, or update an existing one
    /// Drops the evicted page's bytes when full
    fn set(&mut self, k: u64, v: Vec<u8>);
}
