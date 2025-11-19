//! Root node abstraction for thread-safe access
//!
//! This module provides type aliases and helper methods that abstract over
//! single-threaded (Rc<RefCell<>>) and multi-threaded (Arc<RwLock<>>) implementations.

use crate::core::node::StructureNode;

#[cfg(not(feature = "mutli-thread"))]
mod impl_ {
    use super::StructureNode;
    use std::cell::{Ref, RefCell, RefMut};
    use std::rc::Rc;

    /// Single-threaded root node type (zero-cost abstraction)
    pub type RootNode = Rc<RefCell<StructureNode>>;

    /// Create a new root node
    pub fn new_root_node(node: StructureNode) -> RootNode {
        Rc::new(RefCell::new(node))
    }

    /// Read guard for root node
    pub type RootReadGuard<'a> = Ref<'a, StructureNode>;

    /// Write guard for root node
    pub type RootWriteGuard<'a> = RefMut<'a, StructureNode>;

    /// Read access to the root node (shared borrow)
    pub fn root_read(root: &RootNode) -> RootReadGuard<'_> {
        root.borrow()
    }

    /// Write access to the root node (mutable borrow)
    pub fn root_write(root: &RootNode) -> RootWriteGuard<'_> {
        root.borrow_mut()
    }

    /// Execute a closure with read access to the root node
    /// Always succeeds in single-threaded mode
    pub fn root_read_with<F, R>(root: &RootNode, f: F) -> R
    where
        F: FnOnce(&StructureNode) -> R,
    {
        let guard = root_read(root);
        f(&guard)
    }
}

#[cfg(feature = "mutli-thread")]
mod impl_ {
    use super::StructureNode;
    use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

    /// Multi-threaded root node type (thread-safe)
    pub type RootNode = Arc<RwLock<StructureNode>>;

    /// Create a new root node
    pub fn new_root_node(node: StructureNode) -> RootNode {
        Arc::new(RwLock::new(node))
    }

    /// Read guard for root node
    pub type RootReadGuard<'a> = RwLockReadGuard<'a, StructureNode>;

    /// Write guard for root node
    pub type RootWriteGuard<'a> = RwLockWriteGuard<'a, StructureNode>;

    /// Read access to the root node (shared lock)
    pub fn root_read(
        root: &RootNode,
    ) -> Result<RootReadGuard<'_>, std::sync::PoisonError<RwLockReadGuard<'_, StructureNode>>> {
        root.read()
    }

    /// Write access to the root node (exclusive lock)
    pub fn root_write(
        root: &RootNode,
    ) -> Result<RootWriteGuard<'_>, std::sync::PoisonError<RwLockWriteGuard<'_, StructureNode>>>
    {
        root.write()
    }

    /// Execute a closure with read access to the root node
    /// Returns default value if lock acquisition fails
    pub fn root_read_with<F, R>(root: &RootNode, f: F) -> R
    where
        F: FnOnce(&StructureNode) -> R,
        R: Default,
    {
        match root_read(root) {
            Ok(guard) => f(&guard),
            Err(_) => R::default(),
        }
    }
}

pub use impl_::{new_root_node, root_read, root_read_with, root_write, RootNode};
