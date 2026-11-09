use std::collections::HashMap;
use std::fmt::Display;

/// A double linked list for LRU cache implementation
/// `id` is unique for each Node
/// `prev`, `next` are indexes within the Vec storing Node
#[derive(Debug)]
struct Node {
    pub id: u64,
    pub value: Vec<u8>,
    /// Slot id of prev node
    pub prev: Option<usize>,
    /// Slot id of next node
    pub next: Option<usize>,
}

impl Node {
    pub fn new(id: u64, value: Vec<u8>) -> Self {
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
    /// Mapping Node id to slot
    map: HashMap<u64, usize>,
    nodes: Vec<Node>,
    /// Slot id of head
    head: Option<usize>,
    /// Slot id of tail
    tail: Option<usize>,
    capacity: usize,
}

impl LinkedList {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            nodes: Vec::with_capacity(capacity),
            head: None,
            tail: None,
            capacity,
        }
    }

    /// k is the id of the Node
    pub fn node(&self, k: &u64) -> Option<&Node> {
        let slot_id = self.map.get(k)?;
        self.nodes.get(*slot_id)
    }

    pub fn node_mut(&mut self, k: &u64) -> Option<&mut Node> {
        let slot_id = self.map.get(k)?;
        self.nodes.get_mut(*slot_id)
    }

    /// Add node identified by u64 key to list as new head
    /// If the key already exists, move that node to head and update its value
    pub fn add(&mut self, k: u64, v: Vec<u8>) -> Option<u64> {
        // Existing key
        if let Some(&slot_id) = self.map.get(&k) {
            self.nodes[slot_id].value = v;
            self.touch(slot_id);
            return None;
        }

        // Entirely new key
        if self.map.len() >= self.capacity {
            // Slots full
            let evicted = self.pop_tail()?;
            self.map.insert(k, evicted);
            self.nodes[evicted] = Node::new(k, v);
            self.push_head(evicted);
        } else {
            // Slots not full
            let ptr = self.nodes.len();
            self.map.insert(k, ptr);
            self.nodes.push(Node::new(k, v));
            self.push_head(ptr);
        };
        
        Some(k)
    }

    /// Move an existing node to head, no-op if missing or already head
    /// Every private method assume the mapping of id - slot id is correct
    /// Here, `slot_id` must exist
    pub fn touch(&mut self, slot_id: usize) -> Option<&[u8]>{
        if self.head == Some(slot_id) {
            return Some(self.nodes.get(slot_id)?.value());
        };
        self.unlink(slot_id);
        self.push_head(slot_id);
        Some(self.nodes.get(slot_id)?.value())
    }

    /// Detach k from its neighbours; fixes head/tail if k was one of them
    /// k is node id, not slot index
    fn unlink(&mut self, slot_id: usize) {
        let Some(node) = self.nodes.get_mut(slot_id) else {
            return
        };

        let (prev_slot_id, next_slot_id) = (node.prev, node.next);
        match prev_slot_id {
            Some(id) => {
                if let Some(prev_node) = self.nodes.get_mut(id) {
                    prev_node.next = next_slot_id
                }
            }
            None => {
                self.tail = next_slot_id;
            },
        }
        match next_slot_id {
            Some(id) => {
                if let Some(next_node) = self.nodes.get_mut(id) {
                    next_node.prev = prev_slot_id
                }
            }
            None => self.head = prev_slot_id,
        }
    }

    /// Attach an inserted, detached node k as the new head
    fn push_head(&mut self, slot_id: usize) {
        let old_head = self.head;
        if let Some(node) = self.nodes.get_mut(slot_id) {
            node.prev = old_head;
            node.next = None;
        }
        match old_head {
            Some(head_id) => {
                if let Some(n) = self.nodes.get_mut(head_id) {
                    n.next = Some(slot_id)
                }
            }
            None => self.tail = Some(slot_id),
        }
        self.head = Some(slot_id);
    }

    /// Remove current tail from list and nodes, returns its slot index
    fn pop_tail(&mut self) -> Option<usize> {
        let t = self.tail?;
        self.unlink(t);
        
        // Set new tail
        let node_tail = self.nodes.get(t)?;
        self.tail = node_tail.next;
        self.map.remove(&node_tail.id);
        Some(t)
    }
}

struct LinkedListIter<'a> {
    linked_list: &'a LinkedList,
    current: Option<usize>,
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
                let node = self.linked_list.nodes.get(id)?;
                self.current = node.next;
                Some(node.id)
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
pub struct LRUCacheVec {
    linked_list: LinkedList,
}

impl LRUCacheVec {
    pub fn new(capacity: usize) -> Self {
        Self {
            linked_list: LinkedList::new(capacity),
        }
    }

    /// Return the cached page and mark it as most recently used
    /// A miss leaves the list untouched
    pub fn get(&mut self, k: &u64) -> Option<&[u8]> {
        let slot_id = self.linked_list.map.get(k)?;
        self.linked_list.touch(*slot_id)
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
        assert_eq!(iter.collect::<Vec<_>>(), value_order);
        // for (id, target) in iter.zip(value_order.iter()) {
        //     assert_eq!(id, *target)
        // }
    }

    #[test]
    fn lrucache_vec_insert() {
        let mut cache = LRUCacheVec::new(3);

        // Add 3 items to cache
        cache.set(1, vec![1]);
        assert_orders(&cache.linked_list, vec![1]);
        assert_eq!(cache.linked_list.head, cache.linked_list.tail);
        cache.set(2, vec![2]);
        cache.set(3, vec![3]);

        // Assert ordering
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
