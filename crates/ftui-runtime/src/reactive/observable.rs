#![forbid(unsafe_code)]

//! Observable value wrapper with change notification and version tracking.
//!
//! # Design
//!
//! [`Observable<T>`] wraps a value of type `T` in shared, reference-counted
//! storage (`Rc<RefCell<..>>`). When the value changes (determined by
//! `PartialEq`), all live subscribers are notified in registration order.
//!
//! # Performance
//!
//! | Operation    | Complexity               |
//! |-------------|--------------------------|
//! | `get()`     | O(1)                     |
//! | `set()`     | O(S) where S = subscribers |
//! | `subscribe()` | O(1) amortized          |
//! | Memory      | ~48 bytes + sizeof(T)    |
//!
//! # Failure Modes
//!
//! - **Re-entrant set**: Calling `set()` from within a subscriber callback
//!   will panic (RefCell borrow rules). This is intentional: re-entrant
//!   mutations indicate a design bug in the subscriber graph.
//! - **Subscriber leak**: If `Subscription` guards are stored indefinitely
//!   without being dropped, callbacks accumulate. Dead weak references are
//!   cleaned lazily during `notify()`.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A subscriber callback stored as a strong `Rc` internally, handed out
/// as `Weak` to the observable.
type CallbackRc<T> = Rc<dyn Fn(&T)>;
type CallbackWeak<T> = Weak<dyn Fn(&T)>;

/// Shared interior for [`Observable<T>`].
struct ObservableInner<T> {
    value: T,
    version: u64,
    /// Subscribers stored as weak references. Dead entries are pruned on notify.
    subscribers: Vec<CallbackWeak<T>>,
}

/// A shared, version-tracked value with change notification.
///
/// Cloning an `Observable` creates a new handle to the **same** inner state —
/// both handles see the same value and share subscribers.
///
/// # Invariants
///
/// 1. `version` increments by exactly 1 on each value-changing mutation.
/// 2. `set(v)` where `v == current` is a no-op.
/// 3. Subscribers are notified in registration order.
/// 4. Dead subscribers (dropped [`Subscription`] guards) are pruned lazily.
pub struct Observable<T> {
    inner: Rc<RefCell<ObservableInner<T>>>,
}

// Manual Clone: shares the same Rc.
impl<T> Clone for Observable<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Observable<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.borrow();
        f.debug_struct("Observable")
            .field("value", &inner.value)
            .field("version", &inner.version)
            .field("subscriber_count", &inner.subscribers.len())
            .finish()
    }
}

impl<T: Clone + PartialEq + 'static> Observable<T> {
    /// Create a new observable with the given initial value.
    ///
    /// The initial version is 0 and no subscribers are registered.
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ObservableInner {
                value,
                version: 0,
                subscribers: Vec::new(),
            })),
        }
    }

    /// Get a clone of the current value.
    #[must_use]
    pub fn get(&self) -> T {
        self.inner.borrow().value.clone()
    }

    /// Access the current value by reference without cloning.
    ///
    /// The closure `f` receives an immutable reference to the value.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.inner.borrow().value)
    }

    /// Set a new value. If the new value differs from the current value
    /// (by `PartialEq`), the version is incremented and all live subscribers
    /// are notified.
    ///
    /// # Panics
    ///
    /// Panics if called re-entrantly from within a subscriber callback.
    pub fn set(&self, value: T) {
        let changed = {
            let mut inner = self.inner.borrow_mut();
            if inner.value == value {
                return;
            }
            inner.value = value;
            inner.version += 1;
            true
        };
        if changed {
            self.notify();
        }
    }

    /// Modify the value in place via a closure. If the value changes
    /// (compared by `PartialEq` against a snapshot), the version is
    /// incremented and subscribers are notified.
    ///
    /// # Panics
    ///
    /// Panics if called re-entrantly from within a subscriber callback.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        let changed = {
            let mut inner = self.inner.borrow_mut();
            let old = inner.value.clone();
            f(&mut inner.value);
            if inner.value != old {
                inner.version += 1;
                true
            } else {
                false
            }
        };
        if changed {
            self.notify();
        }
    }

    /// Subscribe to value changes. The callback is invoked with a reference
    /// to the new value each time it changes.
    ///
    /// Returns a [`Subscription`] guard. Dropping the guard unsubscribes
    /// the callback (it will not be called after drop, though it may still
    /// be in the subscriber list until the next `notify()` prunes it).
    pub fn subscribe(&self, callback: impl Fn(&T) + 'static) -> Subscription {
        let strong: CallbackRc<T> = Rc::new(callback);
        let weak = Rc::downgrade(&strong);
        self.inner.borrow_mut().subscribers.push(weak);
        // Wrap in a holder struct that can be type-erased as `dyn Any`,
        // since `Rc<dyn Fn(&T)>` itself cannot directly coerce to `Rc<dyn Any>`.
        Subscription {
            _guard: Box::new(strong),
        }
    }

    /// Current version number. Increments by 1 on each value-changing
    /// mutation. Useful for dirty-checking in render loops.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.inner.borrow().version
    }

    /// Number of currently registered subscribers (including dead ones
    /// not yet pruned).
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.inner.borrow().subscribers.len()
    }

    /// Notify live subscribers and prune dead ones.
    ///
    /// If a batch scope is active (see [`super::batch::BatchScope`]),
    /// notifications are deferred until the batch exits.
    fn notify(&self) {
        // Collect live callbacks first (to avoid holding the borrow during calls).
        let callbacks: Vec<CallbackRc<T>> = {
            let mut inner = self.inner.borrow_mut();
            // Prune dead weak refs and collect live ones.
            inner.subscribers.retain(|w| w.strong_count() > 0);
            inner
                .subscribers
                .iter()
                .filter_map(|w| w.upgrade())
                .collect()
        };

        if callbacks.is_empty() {
            return;
        }

        // Clone the value once for all callbacks.
        let value = self.inner.borrow().value.clone();

        if super::batch::is_batching() {
            // Defer each callback to the batch queue.
            for cb in callbacks {
                let v = value.clone();
                super::batch::defer_or_run(move || cb(&v));
            }
        } else {
            // Fire immediately.
            for cb in &callbacks {
                cb(&value);
            }
        }
    }
}

/// RAII guard for a subscriber callback.
///
/// Dropping the `Subscription` causes the associated callback to become
/// unreachable (the strong `Rc` is dropped, so the `Weak` in the
/// observable's subscriber list will fail to upgrade on the next
/// notification cycle).
pub struct Subscription {
    /// Type-erased strong reference keeping the callback `Rc` alive.
    /// When this `Box<dyn Any>` is dropped, the inner `Rc<dyn Fn(&T)>`
    /// is dropped, and the corresponding `Weak` in the subscriber list
    /// loses its referent.
    _guard: Box<dyn std::any::Any>,
}

impl std::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn get_set_basic() {
        let obs = Observable::new(42);
        assert_eq!(obs.get(), 42);
        assert_eq!(obs.version(), 0);

        obs.set(99);
        assert_eq!(obs.get(), 99);
        assert_eq!(obs.version(), 1);
    }

    #[test]
    fn no_change_no_version_bump() {
        let obs = Observable::new(42);
        obs.set(42); // Same value.
        assert_eq!(obs.version(), 0);
    }

    #[test]
    fn with_access() {
        let obs = Observable::new(vec![1, 2, 3]);
        let sum = obs.with(|v| v.iter().sum::<i32>());
        assert_eq!(sum, 6);
    }

    #[test]
    fn update_mutates_in_place() {
        let obs = Observable::new(vec![1, 2, 3]);
        obs.update(|v| v.push(4));
        assert_eq!(obs.get(), vec![1, 2, 3, 4]);
        assert_eq!(obs.version(), 1);
    }

    #[test]
    fn update_no_change_no_bump() {
        let obs = Observable::new(10);
        obs.update(|v| {
            *v = 10; // Same value.
        });
        assert_eq!(obs.version(), 0);
    }

    #[test]
    fn change_notification() {
        let obs = Observable::new(0);
        let count = Rc::new(Cell::new(0u32));
        let count_clone = Rc::clone(&count);

        let _sub = obs.subscribe(move |_val| {
            count_clone.set(count_clone.get() + 1);
        });

        obs.set(1);
        assert_eq!(count.get(), 1);

        obs.set(2);
        assert_eq!(count.get(), 2);

        // Same value — no notification.
        obs.set(2);
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn subscriber_receives_new_value() {
        let obs = Observable::new(0);
        let last_seen = Rc::new(Cell::new(0));
        let last_clone = Rc::clone(&last_seen);

        let _sub = obs.subscribe(move |val| {
            last_clone.set(*val);
        });

        obs.set(42);
        assert_eq!(last_seen.get(), 42);

        obs.set(99);
        assert_eq!(last_seen.get(), 99);
    }

    #[test]
    fn subscription_drop_unsubscribes() {
        let obs = Observable::new(0);
        let count = Rc::new(Cell::new(0u32));
        let count_clone = Rc::clone(&count);

        let sub = obs.subscribe(move |_val| {
            count_clone.set(count_clone.get() + 1);
        });

        obs.set(1);
        assert_eq!(count.get(), 1);

        drop(sub);

        obs.set(2);
        // Callback should NOT have been called.
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn multiple_subscribers() {
        let obs = Observable::new(0);
        let a = Rc::new(Cell::new(0u32));
        let b = Rc::new(Cell::new(0u32));
        let a_clone = Rc::clone(&a);
        let b_clone = Rc::clone(&b);

        let _sub_a = obs.subscribe(move |_| a_clone.set(a_clone.get() + 1));
        let _sub_b = obs.subscribe(move |_| b_clone.set(b_clone.get() + 1));

        obs.set(1);
        assert_eq!(a.get(), 1);
        assert_eq!(b.get(), 1);

        obs.set(2);
        assert_eq!(a.get(), 2);
        assert_eq!(b.get(), 2);
    }

    #[test]
    fn version_increment() {
        let obs = Observable::new("hello".to_string());
        assert_eq!(obs.version(), 0);

        obs.set("world".to_string());
        assert_eq!(obs.version(), 1);

        obs.set("!".to_string());
        assert_eq!(obs.version(), 2);

        // Same value, no increment.
        obs.set("!".to_string());
        assert_eq!(obs.version(), 2);
    }

    #[test]
    fn clone_shares_state() {
        let obs1 = Observable::new(0);
        let obs2 = obs1.clone();

        obs1.set(42);
        assert_eq!(obs2.get(), 42);
        assert_eq!(obs2.version(), 1);

        obs2.set(99);
        assert_eq!(obs1.get(), 99);
        assert_eq!(obs1.version(), 2);
    }

    #[test]
    fn clone_shares_subscribers() {
        let obs1 = Observable::new(0);
        let count = Rc::new(Cell::new(0u32));
        let count_clone = Rc::clone(&count);

        let _sub = obs1.subscribe(move |_| count_clone.set(count_clone.get() + 1));

        let obs2 = obs1.clone();
        obs2.set(1);
        assert_eq!(count.get(), 1); // Subscriber sees change via clone.
    }

    #[test]
    fn subscriber_count() {
        let obs = Observable::new(0);
        assert_eq!(obs.subscriber_count(), 0);

        let _s1 = obs.subscribe(|_| {});
        assert_eq!(obs.subscriber_count(), 1);

        let s2 = obs.subscribe(|_| {});
        assert_eq!(obs.subscriber_count(), 2);

        drop(s2);
        // Dead subscriber not yet pruned.
        assert_eq!(obs.subscriber_count(), 2);

        // Trigger notify to prune dead.
        obs.set(1);
        assert_eq!(obs.subscriber_count(), 1);
    }

    #[test]
    fn debug_format() {
        let obs = Observable::new(42);
        let dbg = format!("{:?}", obs);
        assert!(dbg.contains("Observable"));
        assert!(dbg.contains("42"));
        assert!(dbg.contains("version"));
    }

    #[test]
    fn notification_order_is_registration_order() {
        let obs = Observable::new(0);
        let log = Rc::new(RefCell::new(Vec::new()));

        let log1 = Rc::clone(&log);
        let _s1 = obs.subscribe(move |_| log1.borrow_mut().push('A'));

        let log2 = Rc::clone(&log);
        let _s2 = obs.subscribe(move |_| log2.borrow_mut().push('B'));

        let log3 = Rc::clone(&log);
        let _s3 = obs.subscribe(move |_| log3.borrow_mut().push('C'));

        obs.set(1);
        assert_eq!(*log.borrow(), vec!['A', 'B', 'C']);
    }

    #[test]
    fn update_with_subscriber() {
        let obs = Observable::new(vec![1, 2, 3]);
        let last_len = Rc::new(Cell::new(0usize));
        let last_clone = Rc::clone(&last_len);

        let _sub = obs.subscribe(move |v: &Vec<i32>| {
            last_clone.set(v.len());
        });

        obs.update(|v| v.push(4));
        assert_eq!(last_len.get(), 4);
    }

    #[test]
    fn many_set_calls_version_monotonic() {
        let obs = Observable::new(0);
        for i in 1..=100 {
            obs.set(i);
        }
        assert_eq!(obs.version(), 100);
        assert_eq!(obs.get(), 100);
    }

    #[test]
    fn partial_subscriber_drop() {
        let obs = Observable::new(0);
        let a = Rc::new(Cell::new(0u32));
        let b = Rc::new(Cell::new(0u32));
        let a_clone = Rc::clone(&a);
        let b_clone = Rc::clone(&b);

        let sub_a = obs.subscribe(move |_| a_clone.set(a_clone.get() + 1));
        let _sub_b = obs.subscribe(move |_| b_clone.set(b_clone.get() + 1));

        obs.set(1);
        assert_eq!(a.get(), 1);
        assert_eq!(b.get(), 1);

        drop(sub_a);

        obs.set(2);
        assert_eq!(a.get(), 1); // A was unsubscribed.
        assert_eq!(b.get(), 2); // B still active.
    }

    #[test]
    fn string_observable() {
        let obs = Observable::new(String::new());
        let changes = Rc::new(Cell::new(0u32));
        let changes_clone = Rc::clone(&changes);

        let _sub = obs.subscribe(move |_| changes_clone.set(changes_clone.get() + 1));

        obs.set("hello".to_string());
        obs.set("hello".to_string()); // Same, no notify.
        obs.set("world".to_string());

        assert_eq!(changes.get(), 2);
        assert_eq!(obs.version(), 2);
    }
}
