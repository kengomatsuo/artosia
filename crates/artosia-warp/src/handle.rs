//! The shared setting another thread writes and the compositor reads each frame.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use artosia_slides::frame::Warp;
use rune::Waker;

struct Inner {
    warp: Mutex<Warp>,
    version: AtomicU64,
    wakers: Mutex<Vec<Waker>>,
}

/// A cloneable handle to the current `Warp`. `set` from any thread; the compositor picks the
/// change up on its next frame. Nothing is rebuilt.
#[derive(Clone)]
pub struct WarpHandle(Arc<Inner>);

impl Default for WarpHandle {
    fn default() -> Self {
        WarpHandle::new(Warp::default())
    }
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // A panic elsewhere cannot leave a `Warp` half written, so a poisoned lock is still good.
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl WarpHandle {
    pub fn new(warp: Warp) -> Self {
        WarpHandle(Arc::new(Inner { warp: Mutex::new(warp), version: AtomicU64::new(1), wakers: Mutex::new(Vec::new()) }))
    }

    /// Stores `warp`; when it differs from the current one, bumps `version` and wakes every
    /// window registered with `add_waker`. Returns whether anything changed.
    pub fn set(&self, warp: Warp) -> bool {
        {
            let mut current = locked(&self.0.warp);
            if *current == warp {
                return false;
            }
            *current = warp;
        }
        self.0.version.fetch_add(1, Ordering::AcqRel);
        for wake in locked(&self.0.wakers).iter() {
            wake();
        }
        true
    }

    pub fn get(&self) -> Warp {
        *locked(&self.0.warp)
    }

    /// Bumped on every change; a window's `poll` returns true when this moved.
    pub fn version(&self) -> u64 {
        self.0.version.load(Ordering::Acquire)
    }

    /// Registers a window's waker (the one `AppSpec::setup` receives) so `set` redraws it.
    /// A redraw also needs `poll` to return true: compare `version`.
    pub fn add_waker(&self, waker: Waker) {
        locked(&self.0.wakers).push(waker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn set_reports_change_and_wakes_once() {
        let handle = WarpHandle::default();
        let woken = Arc::new(AtomicUsize::new(0));
        let w = woken.clone();
        handle.add_waker(Arc::new(move || {
            w.fetch_add(1, Ordering::SeqCst);
        }));
        let v = handle.version();
        assert!(!handle.set(Warp::default()));
        assert_eq!(handle.version(), v);
        let tinted = Warp { brightness: 0.1, ..Warp::default() };
        assert!(handle.set(tinted));
        assert_eq!(handle.get(), tinted);
        assert_eq!(handle.version(), v + 1);
        assert_eq!(woken.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn clones_share_one_setting() {
        let a = WarpHandle::default();
        let b = a.clone();
        b.set(Warp { contrast: 1.3, ..Warp::default() });
        assert_eq!(a.get().contrast, 1.3);
    }
}
