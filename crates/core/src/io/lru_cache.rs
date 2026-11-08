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

/// Measuring performance of LRUCache implemented using smart pointers
/// vs no smart pointer
/// Run with cargo test -p dungeon_core --features perf_measure --release perf_measure -- --no-capture
#[cfg(all(test, feature = "perf_measure"))]
mod perf_measure {
    use super::*;
    use std::rc::Rc;
    use std::cell::{Ref, RefCell};
    use std::hint::black_box;
    use std::time::Instant;

    /// An awkward implementation using Box<Rc<RefCell<_>>>
    /// Kept only as an example of the pain, never used by `LRUCache`:
    /// - `Box` around `Rc` is a redundant extra indirection (`Rc` is already a pointer)
    /// - every access goes through `.borrow()` / `.borrow_mut()`, checked at runtime (can panic)
    /// - `get` cannot hand out `&[u8]`, only a `Ref` guard tied to the `RefCell`
    /// - `prev` and `next` are both strong `Rc`s, so neighbours form reference cycles
    ///   and leak unless links are cut by hand (see `Drop`); the usual fix is `Weak`
    ///   for one direction, which adds `.upgrade()` noise on top
    type NodeRc = Rc<RefCell<Node>>;

    #[allow(dead_code)]
    struct Node { id: u64,
        value: Vec<u8>,
        prev: Option<Box<NodeRc>>,
        next: Option<Box<NodeRc>>,
    }

    #[allow(dead_code)]
    impl Node {
        fn new(id: u64, value: Vec<u8>) -> NodeRc {
            Rc::new(RefCell::new(Self {
                id,
                value,
                prev: None,
                next: None,
            }))
        }
    }

    /// Same ordering as `LinkedList`: tail = least recently used, `next` points towards head.
    /// `head` and `tail` are sentinel nodes, real nodes live between them:
    /// tail <-> n1 <-> n2 <-> ... <-> head
    #[allow(dead_code)]
    struct LRUAwkward {
        nodes: HashMap<u64, NodeRc>,
        head: NodeRc,
        tail: NodeRc,
        capacity: usize,
    }

    #[allow(dead_code)]
    impl LRUAwkward {
        fn new(capacity: usize) -> Self {
            let head = Node::new(u64::MAX, vec![]);
            let tail = Node::new(u64::MAX, vec![]);
            tail.borrow_mut().next = Some(Box::new(NodeRc::clone(&head)));
            head.borrow_mut().prev = Some(Box::new(NodeRc::clone(&tail)));
            Self {
                nodes: HashMap::new(),
                head,
                tail,
                capacity,
            }
        }

        /// Takes `&self`: all mutation hides behind RefCell (interior mutability)
        fn get(&self, k: &u64) -> Option<Ref<'_, [u8]>> {
            let node = self.nodes.get(k)?;
            Self::unlink(node);
            self.push_head(node);
            Some(Ref::map(node.borrow(), |n| n.value.as_slice()))
        }

        fn set(&mut self, k: u64, v: Vec<u8>) {
            if self.capacity == 0 {
                return;
            }
            if let Some(node) = self.nodes.get(&k) {
                node.borrow_mut().value = v;
                Self::unlink(node);
                self.push_head(node);
                return;
            }
            if self.nodes.len() >= self.capacity {
                self.pop_tail();
            }
            let node = Node::new(k, v);
            self.push_head(&node);
            self.nodes.insert(k, node);
        }

        /// Detach node from its neighbours; sentinels guarantee both exist
        fn unlink(node: &NodeRc) {
            // take() both links first: holding node's borrow_mut while
            // borrowing a neighbour is fine here, but easy to get wrong
            let (prev, next) = {
                let mut n = node.borrow_mut();
                (n.prev.take(), n.next.take())
            };
            let (Some(prev), Some(next)) = (prev, next) else {
                return;
            };
            prev.borrow_mut().next = Some(next.clone());
            next.borrow_mut().prev = Some(prev);
        }

        /// Insert a detached node right before the head sentinel
        fn push_head(&self, node: &NodeRc) {
            let old = self
                .head
                .borrow_mut()
                .prev
                .take()
                .expect("head sentinel always has prev");
            old.borrow_mut().next = Some(Box::new(NodeRc::clone(node)));
            {
                let mut n = node.borrow_mut();
                n.prev = Some(old);
                n.next = Some(Box::new(NodeRc::clone(&self.head)));
            }
            self.head.borrow_mut().prev = Some(Box::new(NodeRc::clone(node)));
        }

        /// Remove the node right after the tail sentinel, returns its key
        fn pop_tail(&mut self) -> Option<u64> {
            let lru = self.tail.borrow().next.as_ref().map(|n| NodeRc::clone(n))?;
            if Rc::ptr_eq(&lru, &self.head) {
                return None; // empty: tail sentinel points straight at head sentinel
            }
            let id = lru.borrow().id;
            Self::unlink(&lru);
            self.nodes.remove(&id);
            Some(id)
        }
    }

    impl Drop for LRUAwkward {
        /// Strong Rc in both directions = cycles; without cutting them every node leaks
        fn drop(&mut self) {
            for node in self.nodes.values().chain([&self.head, &self.tail]) {
                let mut n = node.borrow_mut();
                n.prev = None;
                n.next = None;
            }
        }
    }

    /// Walk LRUAwkward from tail (LRU) to head (MRU), skipping sentinels
    fn awkward_order(lru: &LRUAwkward) -> Vec<u64> {
        let mut ids = vec![];
        let mut current = lru.tail.borrow().next.as_ref().map(|n| NodeRc::clone(n));
        while let Some(node) = current {
            if Rc::ptr_eq(&node, &lru.head) {
                break;
            }
            ids.push(node.borrow().id);
            current = node.borrow().next.as_ref().map(|n| NodeRc::clone(n));
        }
        ids
    }

    #[test]
    fn lru_awkward_insert() {
        let mut cache = LRUAwkward::new(3);

        // Add 3 items to cache
        cache.set(1, vec![1]);
        assert_eq!(awkward_order(&cache), vec![1]);
        cache.set(2, vec![2]);
        cache.set(3, vec![3]);
        assert_eq!(awkward_order(&cache), vec![1, 2, 3]);

        // Item 1 as tail called, moved to head, value still readable
        assert_eq!(cache.get(&1).as_deref(), Some(&[1u8][..]));
        assert_eq!(awkward_order(&cache), vec![2, 3, 1]);

        // Item 2 as tail called
        let _ = cache.get(&2);
        assert_eq!(awkward_order(&cache), vec![3, 1, 2]);

        // New item evicts tail (3)
        cache.set(4, vec![4]);
        assert_eq!(awkward_order(&cache), vec![1, 2, 4]);
        assert!(cache.get(&3).is_none());
    }

    #[test]
    fn measure_lru_and_awkward() {
        const N: u64 = 1_000_000;
        let mut cache = LRUCache::new(1024);
        let mut cache_awkward = LRUAwkward::new(1024);

        let start = Instant::now();
        for i in 0..N {
            cache.set(i, vec![0u8;64])
        };
        let set_lru_elapsed = start.elapsed();

        let start = Instant::now();
        for i in 0..N {
            black_box(cache.get(&(N - 2048 + i % 2048))); // ~50% hit rate: cache holds keys N-1024..N
        };
        let get_lru_elapsed = start.elapsed();

        let start = Instant::now();
        for i in 0..N {
            cache_awkward.set(i, vec![0u8;64])
        };
        let set_awkward_elapsed = start.elapsed();

        let start = Instant::now();
        for i in 0..N {
            black_box(cache_awkward.get(&(N - 2048 + i % 2048))); // ~50% hit rate: cache holds keys N-1024..N
        };
        let get_awkward_elapsed = start.elapsed();

        println!("LRUCache set: {:?} total, {:?}/op", set_lru_elapsed, set_lru_elapsed/N as u32);
        println!("LRUCache get: {:?} total, {:?}/op", get_lru_elapsed, get_lru_elapsed/N as u32);
        println!("LRUAwkward set: {:?} total, {:?}/op", set_awkward_elapsed, set_awkward_elapsed/N as u32);
        println!("LRUAwkward get: {:?} total, {:?}/op", get_awkward_elapsed, get_awkward_elapsed/N as u32);
    }

}
