use std::collections::HashMap;
use std::fmt::Display;

/// A double linked list for LRU cache implementation
#[derive(Debug)]
struct Node {
    pub id: u64,
    pub value: Vec<u8>,
    pub prev: Option<u64>,
    pub next: Option<u64>,
}

impl Node {
    pub fn new(id: u64, value: Vec<u8>) -> Self {
        // let value = array::from_fn(|_| )
        Self {
            id,
            value,
            prev: None,
            next: None,
        }
    }

    pub fn value(&self) -> &[u8] {
        self.value.as_ref()
    }
}

struct LinkedList {
    nodes: HashMap<u64, Node>,
    head: Option<u64>,
    tail: Option<u64>,
    capacity: usize,
}

impl LinkedList {
    pub fn new(capacity: usize) -> Self {
        Self {
            nodes: HashMap::new(),
            head: None,
            tail: None,
            capacity,
        }
    }

    pub fn node(&self, k: &u64) -> Option<&Node> {
        self.nodes.get(k)
    }

    pub fn node_mut(&mut self, k: &u64) -> Option<&mut Node> {
        self.nodes.get_mut(k)
    }

    /// Add node identified by u64 key to list as new head
    /// If the key already exists, move that node to head
    /// Returns the evicted key, if any, so the caller can drop its value
    pub fn add(&mut self, k: u64, v: Vec<u8>) -> Option<u64> {
        if self.nodes.contains_key(&k) {
            self.touch(k);
            return None;
        }
        let evicted = if self.nodes.len() >= self.capacity {
            self.pop_tail()
        } else {
            None
        };
        self.nodes.insert(k, Node::new(k, v));
        self.push_head(k);
        evicted
    }

    /// Move an existing node to head, no-op if missing or already head
    pub fn touch(&mut self, k: u64) -> Option<&[u8]> {
        if self.head == Some(k) || !self.nodes.contains_key(&k) {
            return None;
        };
        self.unlink(k);
        self.push_head(k);
        Some(self.nodes.get(&k)?.value())
    }

    /// Detach k from its neighbours; fixes head/tail if k was one of them
    fn unlink(&mut self, k: u64) {
        let Some(node) = self.nodes.get(&k) else {
            return;
        };
        let (prev, next) = (node.prev, node.next);
        match prev {
            Some(p) => {
                if let Some(n) = self.nodes.get_mut(&p) {
                    n.next = next
                }
            }
            None => self.tail = next,
        }
        match next {
            Some(n) => {
                if let Some(n) = self.nodes.get_mut(&n) {
                    n.prev = prev
                }
            }
            None => self.head = prev,
        }
    }

    /// Attach an inserted, detached node k as the new head
    fn push_head(&mut self, k: u64) {
        let old_head = self.head;
        if let Some(node) = self.nodes.get_mut(&k) {
            node.prev = old_head;
            node.next = None;
        }
        match old_head {
            Some(h) => {
                if let Some(n) = self.nodes.get_mut(&h) {
                    n.next = Some(k)
                }
            }
            None => self.tail = Some(k),
        }
        self.head = Some(k);
    }

    /// Remove current tail from list and nodes, returns its key
    fn pop_tail(&mut self) -> Option<u64> {
        let t = self.tail?;
        self.unlink(t);
        self.nodes.remove(&t);
        Some(t)
    }
}

struct LinkedListIter<'a> {
    linked_list: &'a LinkedList,
    current: Option<u64>,
}

impl<'a> LinkedListIter<'a> {
    pub fn new(linked_list: &'a LinkedList) -> Self {
        Self {
            linked_list,
            current: linked_list.tail,
        }
    }
}

impl<'a> Iterator for LinkedListIter<'a> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            None => None,
            Some(id) => {
                let output = id;
                let next = {
                    let node = self.linked_list.node(&id)?;
                    node.next
                };
                self.current = next;
                Some(output)
            }
        }
    }
}

impl Display for LinkedList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = write!(f, "Nodes: [");
        let iter = LinkedListIter::new(self);
        for id in iter {
            let _ = write!(f, "{}, ", id);
        }
        write!(f, "]")
    }
}

/// A cache, for byte page, least recently used
pub struct LRUCache {
    linked_list: LinkedList,
}

impl LRUCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            linked_list: LinkedList::new(capacity),
        }
    }

    /// Return the cached page and mark it as most recently used
    /// A miss leaves the list untouched
    pub fn get(&mut self, k: &u64) -> Option<&[u8]> {
        if !self.linked_list.nodes.contains_key(k) {
            return None;
        };
        self.linked_list.touch(*k)
    }

    /// Add a new key - value to cache, or update an existing one
    /// Drops the evicted page's bytes when full
    pub fn set(&mut self, k: u64, v: Vec<u8>) {
        if self.linked_list.capacity == 0 {
            return;
        };
        self.linked_list.add(k, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// To assert whether the linked list is in correct order
    /// indicated by value at index 0 of the inner value
    fn assert_orders(linked_list: &LinkedList, value_order: Vec<u64>) {
        let iter = LinkedListIter::new(linked_list);
        for (id, target) in iter.zip(value_order.iter()) {
            assert_eq!(id, *target)
        }
    }

    #[test]
    fn lru_cache_insert() {
        let mut cache = LRUCache::new(3);

        // Add 3 items to cache
        cache.set(1, vec![1]);
        assert_orders(&cache.linked_list, vec![1]);
        assert_eq!(cache.linked_list.head, cache.linked_list.tail);
        cache.set(2, vec![2]);
        cache.set(3, vec![3]);

        // Assert ordering
        // for i in 1..4 {
        //     println!("Node {i}: {:?}", cache.linked_list.node(&i))
        // };
        assert_orders(&cache.linked_list, vec![1, 2, 3]);

        // Item 1 as tail called, ordering changed
        let _ = cache.get(&1);
        assert_orders(&cache.linked_list, vec![2, 3, 1]);

        // Item 2 as tail called
        let _ = cache.get(&2);
        assert_orders(&cache.linked_list, vec![3, 1, 2]);

        // New item enter cache, ordering changed
        cache.set(4, vec![4]);
        assert_orders(&cache.linked_list, vec![1, 2, 4]);
    }
}
