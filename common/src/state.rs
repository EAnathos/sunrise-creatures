//! What the series keeps about each creature between hooks (docs/CONVENTIONS.md, "Shared state"): a map or a set by
//! entity id, as a static (`EntityMap`, `EntitySet`), locked as every lock of the series (`.locked()`, `crate::sync`).
//!
//! Each one joins a list the first time it is locked, and the library's `entity_leave_level` drops the entity's entry
//! from all of them (`forget`): an entity that dies, despawns, is unloaded or changes dimension (a new entity) leaves
//! nothing behind, with no cleanup of its own for each module to remember. What a creature does at its death (a
//! module's `living_death`) stays the module's.
//!
//! Single player: the server's hooks and the client's renderers share these statics by entity id.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::sync::Locked;

/// A map or a set keyed by entity id, emptied of an entity when it leaves the level.
pub struct ByEntity<C> {
    inner: Mutex<C>,
    joined: AtomicBool,
}

/// What each entity has, by its id.
pub type EntityMap<T> = ByEntity<BTreeMap<u32, T>>;
/// The entities that are something (busy, struck...).
pub type EntitySet = ByEntity<BTreeSet<u32>>;

impl<T> ByEntity<BTreeMap<u32, T>> {
    pub const fn new() -> Self {
        ByEntity {
            inner: Mutex::new(BTreeMap::new()),
            joined: AtomicBool::new(false),
        }
    }
}

impl ByEntity<BTreeSet<u32>> {
    pub const fn new() -> Self {
        ByEntity {
            inner: Mutex::new(BTreeSet::new()),
            joined: AtomicBool::new(false),
        }
    }
}

impl<T> Default for ByEntity<BTreeMap<u32, T>> {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ByEntity<BTreeSet<u32>> {
    fn default() -> Self {
        Self::new()
    }
}

/// A collection an entity's entry can be dropped from.
pub trait Keyed: Send {
    fn drop_entity(&mut self, id: u32);
}

impl<T: Send> Keyed for BTreeMap<u32, T> {
    fn drop_entity(&mut self, id: u32) {
        self.remove(&id);
    }
}

impl Keyed for BTreeSet<u32> {
    fn drop_entity(&mut self, id: u32) {
        self.remove(&id);
    }
}

impl<C> ByEntity<C> {
    /// Whether a panic left its lock poisoned (`crate::sync` takes it anyway).
    pub fn is_poisoned(&self) -> bool {
        self.inner.is_poisoned()
    }
}

impl<C: Keyed + 'static> ByEntity<C> {
    /// Locks it (`crate::sync::Locked`), joining the list `forget` empties the first time.
    pub fn locked(&'static self) -> MutexGuard<'static, C> {
        if !self.joined.swap(true, Ordering::AcqRel) {
            ALL.locked().push(self);
        }
        self.inner.locked()
    }
}

trait Forget: Sync {
    fn forget(&self, id: u32);
}

impl<C: Keyed> Forget for ByEntity<C> {
    fn forget(&self, id: u32) {
        self.inner.locked().drop_entity(id);
    }
}

/// Every map and set locked so far.
static ALL: Mutex<Vec<&'static dyn Forget>> = Mutex::new(Vec::new());

/// From the library's `entity_leave_level`: entity `id` is gone, its entries dropped everywhere. The list is copied
/// out first: each entry is dropped under its own lock only.
pub fn forget(id: u32) {
    let all: Vec<&'static dyn Forget> = ALL.locked().clone();
    for f in all {
        f.forget(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static SEEN: EntityMap<i32> = EntityMap::new();
    static BUSY: EntitySet = EntitySet::new();

    #[test]
    fn an_entity_that_leaves_is_dropped_from_every_map_and_set() {
        SEEN.locked().insert(5_000_000, 1);
        SEEN.locked().insert(5_000_001, 2);
        BUSY.locked().insert(5_000_000);
        forget(5_000_000);
        assert!(!SEEN.locked().contains_key(&5_000_000));
        assert_eq!(SEEN.locked().get(&5_000_001), Some(&2), "the others stay");
        assert!(!BUSY.locked().contains(&5_000_000));
        // Joined once, however often locked.
        drop(SEEN.locked());
        let me = &SEEN as *const EntityMap<i32>;
        let joined = ALL
            .locked()
            .iter()
            .filter(|f| std::ptr::addr_eq(**f as *const dyn Forget, me))
            .count();
        assert_eq!(joined, 1);
    }
}
