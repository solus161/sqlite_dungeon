//! An awkward implementation using Box<Rc<RefCell<_>>>
//! Kept only as an example of the pain, compared against `dungeon_core`'s `LRUCache`:
//! - `Box` around `Rc` is a redundant extra indirection (`Rc` is already a pointer)
//! - every access goes through `.borrow()` / `.borrow_mut()`, checked at runtime (can panic)
//! - `get` cannot hand out `&[u8]`, only a `Ref` guard tied to the `RefCell`
//! - `prev` and `next` are both strong `Rc`s, so neighbours form reference cycles
//!   and leak unless links are cut by hand (see `Drop`); the usual fix is `Weak`
//!   for one direction, which adds `.upgrade()` noise on top

use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

type NodeRc = Rc<RefCell<Node>>;

struct Node {
    id: u64,
    value: Vec<u8>,
    prev: Option<Box<NodeRc>>,
    next: Option<Box<NodeRc>>,
}

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

/// Same ordering as the core `LinkedList`: tail = least recently used, `next` points towards head.
/// `head` and `tail` are sentinel nodes, real nodes live between them:
/// tail <-> n1 <-> n2 <-> ... <-> head
struct LinkedList {
    nodes: HashMap<u64, NodeRc>,
    head: NodeRc,
    tail: NodeRc,
    capacity: usize,
}

impl LinkedList {
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

    /// Add node identified by u64 key to list as new head
    /// If the key already exists, update its value and move it to head
    /// Returns the evicted key, if any
    fn add(&mut self, k: u64, v: Vec<u8>) -> Option<u64> {
        if let Some(node) = self.nodes.get(&k) {
            node.borrow_mut().value = v;
            Self::unlink(node);
            self.push_head(node);
            return None;
        }
        let evicted = if self.nodes.len() >= self.capacity {
            self.pop_tail()
        } else {
            None
        };
        let node = Node::new(k, v);
        self.push_head(&node);
        self.nodes.insert(k, node);
        evicted
    }

    /// Move an existing node to head and return its value, None if missing
    /// Takes `&self`: all mutation hides behind RefCell (interior mutability)
    fn touch(&self, k: u64) -> Option<Ref<'_, [u8]>> {
        let node = self.nodes.get(&k)?;
        Self::unlink(node);
        self.push_head(node);
        Some(Ref::map(node.borrow(), |n| n.value.as_slice()))
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

impl Drop for LinkedList {
    /// Strong Rc in both directions = cycles; without cutting them every node leaks
    fn drop(&mut self) {
        for node in self.nodes.values().chain([&self.head, &self.tail]) {
            let mut n = node.borrow_mut();
            n.prev = None;
            n.next = None;
        }
    }
}

/// A cache, for byte page, least recently used
pub struct LRUCacheRc {
    linked_list: LinkedList,
}

impl LRUCacheRc {
    pub fn new(capacity: usize) -> Self {
        Self {
            linked_list: LinkedList::new(capacity),
        }
    }

    /// Return the cached page and mark it as most recently used
    /// A miss leaves the list untouched
    pub fn get(&self, k: &u64) -> Option<Ref<'_, [u8]>> {
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

    /// Walk the list from tail (LRU) to head (MRU), skipping sentinels
    fn list_order(linked_list: &LinkedList) -> Vec<u64> {
        let mut ids = vec![];
        let mut current = linked_list
            .tail
            .borrow()
            .next
            .as_ref()
            .map(|n| NodeRc::clone(n));
        while let Some(node) = current {
            if Rc::ptr_eq(&node, &linked_list.head) {
                break;
            }
            ids.push(node.borrow().id);
            current = node.borrow().next.as_ref().map(|n| NodeRc::clone(n));
        }
        ids
    }

    #[test]
    fn lrucache_rc_insert() {
        let mut cache = LRUCacheRc::new(3);

        // Add 3 items to cache
        cache.set(1, vec![1]);
        assert_eq!(list_order(&cache.linked_list), vec![1]);
        cache.set(2, vec![2]);
        cache.set(3, vec![3]);
        assert_eq!(list_order(&cache.linked_list), vec![1, 2, 3]);

        // Item 1 as tail called, moved to head, value still readable
        assert_eq!(cache.get(&1).as_deref(), Some(&[1u8][..]));
        assert_eq!(list_order(&cache.linked_list), vec![2, 3, 1]);

        // Item 2 as tail called
        let _ = cache.get(&2);
        assert_eq!(list_order(&cache.linked_list), vec![3, 1, 2]);

        // Head called: still a hit, order unchanged
        assert_eq!(cache.get(&2).as_deref(), Some(&[2u8][..]));
        assert_eq!(list_order(&cache.linked_list), vec![3, 1, 2]);

        // Existing key updated: new value, moved to head
        cache.set(1, vec![9]);
        assert_eq!(list_order(&cache.linked_list), vec![3, 2, 1]);
        assert_eq!(cache.get(&1).as_deref(), Some(&[9u8][..]));

        // New item evicts tail (3)
        cache.set(4, vec![4]);
        assert_eq!(list_order(&cache.linked_list), vec![2, 1, 4]);
        assert!(cache.get(&3).is_none());
    }
}
