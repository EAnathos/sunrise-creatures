//! The series' locks on its shared state (docs/CONVENTIONS.md, "Shared state"): every mutex of the library and the mods
//! is taken through `Locked::locked`, so a poisoned one is handled in one way everywhere.
//!
//! A mutex is poisoned when code panics while holding it. A hook's panic stops the game (its crash report names the
//! hook), but a Part's (a renderer, an item's use) is contained by the game, which logs it and goes on: the next hook to
//! take that mutex would then panic too, and stop the game naming the wrong place. So a poisoned lock is taken anyway,
//! its data as the panic left it (each entry is one creature's, rebuilt as it ticks), and counted: the count is a line of
//! any later crash report (`Report`), beside the panic the game logged where it happened. Nothing is hidden, and one
//! fault doesn't turn every later tick into a crash.
//!
//! The rules the code keeps with them: take a lock for as short as possible; never call the game (a host), a mod's
//! callback or another module's function while holding one, but copy what is needed out first and write back after;
//! take two at once only when they're always taken in the same order.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use mod_api::CrashSection;

/// How many poisoned locks were taken since the game started.
static RECOVERED: AtomicUsize = AtomicUsize::new(0);

/// A lock taken the series' way.
pub trait Locked<T> {
    /// Locks it; a poisoned lock is cleared, counted, and taken as it is.
    fn locked(&self) -> MutexGuard<'_, T>;
}

impl<T> Locked<T> for Mutex<T> {
    fn locked(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| {
            RECOVERED.fetch_add(1, Ordering::Relaxed);
            self.clear_poison();
            poisoned.into_inner()
        })
    }
}

/// How many poisoned locks were taken since the game started.
pub fn recovered() -> usize {
    RECOVERED.load(Ordering::Relaxed)
}

/// The crash report's line, when a lock was ever found poisoned.
pub struct Report;

impl CrashSection for Report {
    fn value(&self) -> Option<String> {
        let n = recovered();
        (n > 0).then(|| format!("{n} poisoned lock(s) taken after a panic (see the panic logged before)"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poisoned_lock_is_taken_with_its_data_and_counted() {
        static M: Mutex<Vec<u32>> = Mutex::new(Vec::new());
        let before = recovered();
        let _ = std::thread::spawn(|| {
            let mut v = M.locked();
            v.push(1);
            panic!("while holding it");
        })
        .join();
        assert!(M.is_poisoned());
        assert_eq!(*M.locked(), vec![1], "the data as the panic left it");
        assert!(!M.is_poisoned(), "cleared");
        M.locked().push(2);
        // Other tests may poison their own locks meanwhile: this one is counted, at least.
        let after = recovered();
        assert!(after > before);
        assert!(Report.value().is_some());
        assert_eq!(*M.locked(), vec![1, 2]);
    }
}
