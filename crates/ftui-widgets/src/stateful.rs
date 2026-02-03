//! Opt-in trait for widgets with persistable state.
//!
//! The [`Stateful`] trait defines a contract for widgets that can save and
//! restore their state across sessions or configuration changes. It is
//! orthogonal to [`StatefulWidget`](super::StatefulWidget) — a widget can
//! implement both (render-time state mutation + persistence) or just one.
//!
//! # Design Invariants
//!
//! 1. **Round-trip fidelity**: `restore_state(save_state())` must produce an
//!    equivalent observable state. Fields that are purely derived (e.g., cached
//!    layout) may differ, but user-facing state (scroll position, selection,
//!    expanded nodes) must survive the round trip.
//!
//! 2. **Graceful version mismatch**: When [`VersionedState`] detects a version
//!    mismatch (`stored.version != T::state_version()`), the caller should fall
//!    back to `T::State::default()` rather than panic. Migration logic belongs
//!    in the downstream state migration system (bd-30g1.5).
//!
//! 3. **Key uniqueness**: Two distinct widget instances must produce distinct
//!    [`StateKey`] values. The `(widget_type, instance_id)` pair is the primary
//!    uniqueness invariant.
//!
//! 4. **No side effects**: `save_state` must be a pure read; `restore_state`
//!    must only mutate `self` (no I/O, no global state).
//!
//! # Failure Modes
//!
//! | Failure | Cause | Fallback |
//! |---------|-------|----------|
//! | Deserialization error | Schema drift, corrupt data | Use `Default::default()` |
//! | Version mismatch | Widget upgraded | Use `Default::default()` |
//! | Missing state | First run, key changed | Use `Default::default()` |
//! | Duplicate key | Bug in `state_key()` impl | Last-write-wins (logged) |
//!
//! # Feature Gate
//!
//! This module is always available, but the serde-based [`VersionedState`]
//! wrapper requires the `state-persistence` feature for serialization support.

use core::fmt;
use core::hash::{Hash, Hasher};

/// Unique identifier for a widget's persisted state.
///
/// A `StateKey` is the `(widget_type, instance_id)` pair that maps a widget
/// instance to its stored state blob. Widget type is a `&'static str` (cheap
/// to copy, no allocation) while instance id is an owned `String` to support
/// dynamic widget trees.
///
/// # Construction
///
/// ```
/// # use ftui_widgets::stateful::StateKey;
/// // Explicit
/// let key = StateKey::new("ScrollView", "main-content");
///
/// // From a widget-tree path
/// let key = StateKey::from_path(&["app", "sidebar", "tree"]);
/// assert_eq!(key.instance_id, "app/sidebar/tree");
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateKey {
    /// The widget type name (e.g., `"ScrollView"`, `"TreeView"`).
    pub widget_type: &'static str,
    /// Instance-unique identifier within a widget tree.
    pub instance_id: String,
}

impl StateKey {
    /// Create a new state key from a widget type and instance id.
    #[must_use]
    pub fn new(widget_type: &'static str, id: impl Into<String>) -> Self {
        Self {
            widget_type,
            instance_id: id.into(),
        }
    }

    /// Build a state key from a path of widget-tree segments.
    ///
    /// Segments are joined with `/` to form the instance id.
    /// The widget type is derived from the last segment.
    ///
    /// # Panics
    ///
    /// Panics if `path` is empty.
    #[must_use]
    pub fn from_path(path: &[&str]) -> Self {
        assert!(
            !path.is_empty(),
            "StateKey::from_path requires a non-empty path"
        );
        let widget_type_str = path.last().expect("checked non-empty");
        // We need a &'static str for widget_type. Since the caller passes &str
        // slices that may or may not be 'static, we leak a copy. This is fine
        // because state keys are created once and live for the program lifetime.
        let widget_type: &'static str = Box::leak((*widget_type_str).to_owned().into_boxed_str());
        Self {
            widget_type,
            instance_id: path.join("/"),
        }
    }

    /// Canonical string representation: `"widget_type::instance_id"`.
    #[must_use]
    pub fn canonical(&self) -> String {
        format!("{}::{}", self.widget_type, self.instance_id)
    }
}

impl Hash for StateKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.widget_type.hash(state);
        self.instance_id.hash(state);
    }
}

impl fmt::Display for StateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.widget_type, self.instance_id)
    }
}

/// Opt-in trait for widgets with persistable state.
///
/// Implementing this trait signals that a widget's user-facing state can be
/// serialized, stored, and later restored. This is used by the state registry
/// (bd-30g1.2) to persist widget state across sessions.
///
/// # Relationship to `StatefulWidget`
///
/// - [`StatefulWidget`](super::StatefulWidget): render-time mutable state (scroll clamping, layout cache).
/// - [`Stateful`]: persistence contract (save/restore across sessions).
///
/// A widget can implement both when its render-time state is also worth persisting.
///
/// # Example
///
/// ```ignore
/// use serde::{Serialize, Deserialize};
/// use ftui_widgets::stateful::{Stateful, StateKey};
///
/// #[derive(Serialize, Deserialize, Default)]
/// struct ScrollViewPersist {
///     scroll_offset: u16,
/// }
///
/// impl Stateful for ScrollView {
///     type State = ScrollViewPersist;
///
///     fn state_key(&self) -> StateKey {
///         StateKey::new("ScrollView", &self.id)
///     }
///
///     fn save_state(&self) -> Self::State {
///         ScrollViewPersist { scroll_offset: self.offset }
///     }
///
///     fn restore_state(&mut self, state: Self::State) {
///         self.offset = state.scroll_offset.min(self.max_offset());
///     }
/// }
/// ```
pub trait Stateful: Sized {
    /// The state type that gets persisted.
    ///
    /// Must implement `Default` so missing/corrupt state degrades gracefully.
    type State: Default;

    /// Unique key identifying this widget instance.
    ///
    /// Two distinct widget instances **must** return distinct keys.
    fn state_key(&self) -> StateKey;

    /// Extract current state for persistence.
    ///
    /// This must be a pure read — no side effects, no I/O.
    fn save_state(&self) -> Self::State;

    /// Restore state from persistence.
    ///
    /// Implementations should clamp restored values to valid ranges
    /// (e.g., scroll offset ≤ max offset) rather than trusting stored data.
    fn restore_state(&mut self, state: Self::State);

    /// State schema version for forward-compatible migrations.
    ///
    /// Bump this when the `State` type's serialized form changes in a
    /// backwards-incompatible way. The state registry will discard stored
    /// state with a mismatched version and fall back to `Default`.
    fn state_version() -> u32 {
        1
    }
}

/// Version-tagged wrapper for serialized widget state.
///
/// When persisting state, the registry wraps the raw state in this envelope
/// so it can detect schema version mismatches on restore.
///
/// # Serialization
///
/// With the `state-persistence` feature enabled, `VersionedState` derives
/// `Serialize` and `Deserialize`. Without the feature, it is a plain struct
/// usable for in-memory versioning.
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "state-persistence",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct VersionedState<S> {
    /// Schema version (from `Stateful::state_version()`).
    pub version: u32,
    /// The actual state payload.
    pub data: S,
}

impl<S> VersionedState<S> {
    /// Wrap state with its current version tag.
    #[must_use]
    pub fn new(version: u32, data: S) -> Self {
        Self { version, data }
    }

    /// Pack a widget's state into a versioned envelope.
    pub fn pack<W: Stateful<State = S>>(widget: &W) -> Self {
        Self {
            version: W::state_version(),
            data: widget.save_state(),
        }
    }

    /// Attempt to unpack, returning `None` if the version does not match
    /// the widget's current `state_version()`.
    pub fn unpack<W: Stateful<State = S>>(self) -> Option<S> {
        if self.version == W::state_version() {
            Some(self.data)
        } else {
            None
        }
    }

    /// Unpack with fallback: returns the stored data if versions match,
    /// otherwise returns `S::default()`.
    pub fn unpack_or_default<W: Stateful<State = S>>(self) -> S
    where
        S: Default,
    {
        if self.version == W::state_version() {
            self.data
        } else {
            S::default()
        }
    }
}

impl<S: Default> Default for VersionedState<S> {
    fn default() -> Self {
        Self {
            version: 1,
            data: S::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test widget ─────────────────────────────────────────────────

    #[derive(Default)]
    struct TestScrollView {
        id: String,
        offset: u16,
        max: u16,
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    struct ScrollState {
        scroll_offset: u16,
    }

    impl Stateful for TestScrollView {
        type State = ScrollState;

        fn state_key(&self) -> StateKey {
            StateKey::new("ScrollView", &self.id)
        }

        fn save_state(&self) -> ScrollState {
            ScrollState {
                scroll_offset: self.offset,
            }
        }

        fn restore_state(&mut self, state: ScrollState) {
            self.offset = state.scroll_offset.min(self.max);
        }
    }

    // ── Another test widget with version 2 ──────────────────────────

    #[derive(Default)]
    struct TestTreeView {
        id: String,
        expanded: Vec<u32>,
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    struct TreeState {
        expanded_nodes: Vec<u32>,
        collapse_all_on_blur: bool, // added in v2
    }

    impl Stateful for TestTreeView {
        type State = TreeState;

        fn state_key(&self) -> StateKey {
            StateKey::new("TreeView", &self.id)
        }

        fn save_state(&self) -> TreeState {
            TreeState {
                expanded_nodes: self.expanded.clone(),
                collapse_all_on_blur: false,
            }
        }

        fn restore_state(&mut self, state: TreeState) {
            self.expanded = state.expanded_nodes;
        }

        fn state_version() -> u32 {
            2
        }
    }

    // ── StateKey tests ──────────────────────────────────────────────

    #[test]
    fn state_key_new() {
        let key = StateKey::new("ScrollView", "main");
        assert_eq!(key.widget_type, "ScrollView");
        assert_eq!(key.instance_id, "main");
    }

    #[test]
    fn state_key_from_path() {
        let key = StateKey::from_path(&["app", "sidebar", "tree"]);
        assert_eq!(key.instance_id, "app/sidebar/tree");
        assert_eq!(key.widget_type, "tree");
    }

    #[test]
    #[should_panic(expected = "non-empty path")]
    fn state_key_from_empty_path_panics() {
        let _ = StateKey::from_path(&[]);
    }

    #[test]
    fn state_key_uniqueness() {
        let a = StateKey::new("ScrollView", "main");
        let b = StateKey::new("ScrollView", "sidebar");
        let c = StateKey::new("TreeView", "main");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);
    }

    #[test]
    fn state_key_equality() {
        let a = StateKey::new("ScrollView", "main");
        let b = StateKey::new("ScrollView", "main");
        assert_eq!(a, b);
    }

    #[test]
    fn state_key_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;

        let a = StateKey::new("ScrollView", "main");
        let b = StateKey::new("ScrollView", "main");

        let hash = |key: &StateKey| {
            let mut h = DefaultHasher::new();
            key.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(&a), hash(&b));
    }

    #[test]
    fn state_key_display() {
        let key = StateKey::new("ScrollView", "main");
        assert_eq!(key.to_string(), "ScrollView::main");
    }

    #[test]
    fn state_key_canonical() {
        let key = StateKey::new("ScrollView", "main");
        assert_eq!(key.canonical(), "ScrollView::main");
    }

    // ── Save/restore round-trip tests ───────────────────────────────

    #[test]
    fn save_restore_round_trip() {
        let mut widget = TestScrollView {
            id: "content".into(),
            offset: 42,
            max: 100,
        };

        let saved = widget.save_state();
        assert_eq!(saved.scroll_offset, 42);

        widget.offset = 0; // reset
        widget.restore_state(saved);
        assert_eq!(widget.offset, 42);
    }

    #[test]
    fn restore_clamps_to_valid_range() {
        let mut widget = TestScrollView {
            id: "content".into(),
            offset: 0,
            max: 10,
        };

        // Stored state exceeds current max
        widget.restore_state(ScrollState { scroll_offset: 999 });
        assert_eq!(widget.offset, 10);
    }

    #[test]
    fn default_state_on_missing() {
        let mut widget = TestScrollView {
            id: "new".into(),
            offset: 5,
            max: 100,
        };

        widget.restore_state(ScrollState::default());
        assert_eq!(widget.offset, 0);
    }

    // ── Version tests ───────────────────────────────────────────────

    #[test]
    fn default_state_version_is_one() {
        assert_eq!(TestScrollView::state_version(), 1);
    }

    #[test]
    fn custom_state_version() {
        assert_eq!(TestTreeView::state_version(), 2);
    }

    // ── VersionedState tests ────────────────────────────────────────

    #[test]
    fn versioned_state_pack_unpack() {
        let widget = TestScrollView {
            id: "main".into(),
            offset: 77,
            max: 100,
        };

        let packed = VersionedState::pack(&widget);
        assert_eq!(packed.version, 1);
        assert_eq!(packed.data.scroll_offset, 77);

        let unpacked = packed.unpack::<TestScrollView>();
        assert!(unpacked.is_some());
        assert_eq!(unpacked.unwrap().scroll_offset, 77);
    }

    #[test]
    fn versioned_state_version_mismatch_returns_none() {
        // Simulate stored state from version 1, but widget expects version 2
        let stored = VersionedState::<TreeState> {
            version: 1,
            data: TreeState::default(),
        };

        let result = stored.unpack::<TestTreeView>();
        assert!(result.is_none());
    }

    #[test]
    fn versioned_state_unpack_or_default_on_mismatch() {
        let stored = VersionedState::<TreeState> {
            version: 1,
            data: TreeState {
                expanded_nodes: vec![1, 2, 3],
                collapse_all_on_blur: true,
            },
        };

        let result = stored.unpack_or_default::<TestTreeView>();
        // Should return default because version 1 != expected 2
        assert_eq!(result, TreeState::default());
    }

    #[test]
    fn versioned_state_unpack_or_default_on_match() {
        let stored = VersionedState::<ScrollState> {
            version: 1,
            data: ScrollState { scroll_offset: 55 },
        };

        let result = stored.unpack_or_default::<TestScrollView>();
        assert_eq!(result.scroll_offset, 55);
    }

    #[test]
    fn versioned_state_default() {
        let vs = VersionedState::<ScrollState>::default();
        assert_eq!(vs.version, 1);
        assert_eq!(vs.data, ScrollState::default());
    }
}
