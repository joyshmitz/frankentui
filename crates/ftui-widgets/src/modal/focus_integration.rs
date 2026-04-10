#![forbid(unsafe_code)]

//! Focus-aware modal integration for automatic focus trap management.
//!
//! This module provides `FocusAwareModalStack`, which combines [`ModalStack`]
//! with [`FocusManager`] integration for automatic focus trapping when modals
//! are opened and focus restoration when they close.
//!
//! # Invariants
//!
//! 1. **Auto-focus**: When a modal opens with a focus group, focus moves to the
//!    first focusable element in that group.
//! 2. **Focus trap**: Tab navigation is constrained to the modal's focus group.
//! 3. **Focus restoration**: When a modal closes, focus returns to where it was
//!    before the modal opened.
//! 4. **LIFO ordering**: Focus traps follow modal stack ordering (nested modals
//!    restore to the correct previous state).
//!
//! # Failure Modes
//!
//! - If the focus group has no focusable members, focus remains unchanged.
//! - If the original focus target is removed during modal display, focus moves
//!   to the first available focusable element.
//! - Focus trap with an empty group allows focus to escape (graceful degradation).
//!
//! # Example
//!
//! ```ignore
//! use ftui_widgets::focus::FocusManager;
//! use ftui_widgets::modal::{ModalStack, WidgetModalEntry};
//! use ftui_widgets::modal::focus_integration::FocusAwareModalStack;
//!
//! let mut modals = FocusAwareModalStack::new();
//!
//! // Push modal with focus group members
//! let focus_ids = vec![ok_button_id, cancel_button_id];
//! let modal_id = modals.push_with_trap(
//!     Box::new(WidgetModalEntry::new(dialog)),
//!     focus_ids,
//! );
//!
//! // Handle event (focus trap active, Escape closes and restores focus)
//! if let Some(result) = modals.handle_event(&event, None) {
//!     // Modal closed, focus already restored
//! }
//! ```

use std::sync::atomic::{AtomicU32, Ordering};

use ftui_core::event::Event;
use ftui_core::geometry::Rect;
use ftui_render::frame::{Frame, HitData, HitId, HitRegion};

use crate::focus::{FocusId, FocusManager};
use crate::modal::{ModalId, ModalResult, ModalStack, StackModal};

/// Global counter for unique focus group IDs.
static FOCUS_GROUP_COUNTER: AtomicU32 = AtomicU32::new(1_000_000);

/// Generate a unique focus group ID.
fn next_focus_group_id(focus_manager: &FocusManager) -> u32 {
    loop {
        let group_id = FOCUS_GROUP_COUNTER.fetch_add(1, Ordering::Relaxed);
        if !focus_manager.has_group(group_id) {
            return group_id;
        }
    }
}

/// Modal stack with integrated focus management.
///
/// This wrapper provides automatic focus trapping when modals open and
/// focus restoration when they close. It manages both the modal stack
/// and focus manager in a coordinated way.
///
/// # Invariants
///
/// - Focus trap stack depth equals the number of modals with focus groups.
/// - Each modal's focus group ID is unique and not reused.
/// - Pop operations always call `pop_trap` for modals with focus groups.
pub struct FocusAwareModalStack {
    stack: ModalStack,
    focus_manager: FocusManager,
    base_focus: Option<FocusId>,
}

impl Default for FocusAwareModalStack {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusAwareModalStack {
    /// Create a new focus-aware modal stack.
    pub fn new() -> Self {
        Self {
            stack: ModalStack::new(),
            focus_manager: FocusManager::new(),
            base_focus: None,
        }
    }

    /// Create from existing stack and focus manager.
    ///
    /// Use this when you already have a `FocusManager` in your application
    /// and want to integrate modal focus trapping.
    pub fn with_focus_manager(focus_manager: FocusManager) -> Self {
        Self {
            stack: ModalStack::new(),
            focus_manager,
            base_focus: None,
        }
    }

    // --- Modal Stack Delegation ---

    /// Push a modal without focus trapping.
    ///
    /// The modal will be rendered and receive events, but focus is not managed.
    pub fn push(&mut self, modal: Box<dyn StackModal>) -> ModalId {
        self.stack.push(modal)
    }

    /// Push a modal with automatic focus trapping.
    ///
    /// # Parameters
    /// - `modal`: The modal content
    /// - `focusable_ids`: The focus IDs of elements inside the modal
    ///
    /// # Behavior
    /// 1. Creates a focus group with the provided IDs
    /// 2. Pushes a focus trap (saving current focus)
    /// 3. Moves focus to the first element in the group
    pub fn push_with_trap(
        &mut self,
        modal: Box<dyn StackModal>,
        focusable_ids: Vec<FocusId>,
    ) -> ModalId {
        let group_id = next_focus_group_id(&self.focus_manager);
        let base_focus = self.focus_manager.current();
        let was_trapped = self.focus_manager.is_trapped();

        // Create focus group and push trap.
        // If the group ends up empty (no focusable members), push_trap
        // returns false and we record no focus group for this modal so
        // that pop() won't try to pop a trap that was never pushed.
        self.focus_manager.create_group(group_id, focusable_ids);
        let trapped = self.focus_manager.push_trap(group_id);
        if !trapped {
            self.focus_manager.remove_group(group_id);
        }
        if trapped && !was_trapped {
            self.base_focus = base_focus;
        }

        // Push modal with focus group tracking
        let focus_group = if trapped { Some(group_id) } else { None };
        self.stack.push_with_focus(modal, focus_group)
    }

    /// Pop the top modal.
    ///
    /// If the modal had a focus group, the focus trap is popped and
    /// focus is restored to where it was before the modal opened.
    pub fn pop(&mut self) -> Option<ModalResult> {
        let result = self.stack.pop()?;
        if let Some(group_id) = result.focus_group_id {
            self.close_focus_group(group_id);
        }
        Some(result)
    }

    /// Pop a specific modal by ID.
    ///
    pub fn pop_id(&mut self, id: ModalId) -> Option<ModalResult> {
        let result = self.stack.pop_id(id)?;
        if let Some(group_id) = result.focus_group_id {
            let closing_members = self.focus_manager.group_members(group_id);
            self.focus_manager.remove_group(group_id);
            self.rebuild_focus_traps();
            self.focus_manager
                .repair_focus_after_excluding_ids(&closing_members);
        }

        Some(result)
    }

    /// Pop all modals, restoring focus to the original state.
    pub fn pop_all(&mut self) -> Vec<ModalResult> {
        let results = self.stack.pop_all();
        let mut removed_group = false;
        let mut removed_members = Vec::new();
        for result in &results {
            if let Some(group_id) = result.focus_group_id {
                removed_members.extend(self.focus_manager.group_members(group_id));
                self.focus_manager.remove_group(group_id);
                removed_group = true;
            }
        }
        if removed_group {
            self.rebuild_focus_traps();
            self.focus_manager
                .repair_focus_after_excluding_ids(&removed_members);
        }
        results
    }

    /// Handle an event, routing to the top modal.
    ///
    /// If the modal closes (via Escape, backdrop click, etc.), the focus
    /// trap is automatically popped and focus is restored.
    pub fn handle_event(
        &mut self,
        event: &Event,
        hit: Option<(HitId, HitRegion, HitData)>,
    ) -> Option<ModalResult> {
        if let Event::Focus(focused) = event {
            self.focus_manager.apply_host_focus(*focused);
        }
        let result = self.stack.handle_event(event, hit)?;
        if let Some(group_id) = result.focus_group_id {
            self.close_focus_group(group_id);
        }
        Some(result)
    }

    /// Render all modals.
    pub fn render(&self, frame: &mut Frame, screen: Rect) {
        self.stack.render(frame, screen);
    }

    // --- State Queries ---

    /// Check if the modal stack is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Get the number of open modals.
    #[inline]
    pub fn depth(&self) -> usize {
        self.stack.depth()
    }

    /// Check if focus is currently trapped in a modal.
    #[inline]
    pub fn is_focus_trapped(&self) -> bool {
        self.focus_manager.is_trapped()
    }

    /// Get a reference to the underlying modal stack.
    pub fn stack(&self) -> &ModalStack {
        &self.stack
    }

    /// Get a mutable reference to the underlying modal stack.
    ///
    /// **Warning**: Direct manipulation may desync focus state.
    pub fn stack_mut(&mut self) -> &mut ModalStack {
        &mut self.stack
    }

    /// Get a reference to the focus manager.
    pub fn focus_manager(&self) -> &FocusManager {
        &self.focus_manager
    }

    /// Get a mutable reference to the focus manager.
    pub fn focus_manager_mut(&mut self) -> &mut FocusManager {
        &mut self.focus_manager
    }

    fn close_focus_group(&mut self, group_id: u32) {
        let closing_members = self.focus_manager.group_members(group_id);
        self.focus_manager.pop_trap();
        self.focus_manager.remove_group(group_id);
        self.focus_manager
            .repair_focus_after_excluding_ids(&closing_members);
        if !self.focus_manager.is_trapped() {
            self.base_focus = None;
        }
    }

    fn rebuild_focus_traps(&mut self) {
        let group_ids = self.stack.focus_group_ids_in_order();
        self.focus_manager.clear_traps();

        if group_ids.is_empty() {
            if let Some(base_focus) = self.base_focus {
                let _ = self.focus_manager.focus_without_history(base_focus);
            } else if self.focus_manager.current().is_some() {
                let _ = self.focus_manager.blur();
            }

            if self.base_focus.is_some() && self.focus_manager.current() != self.base_focus {
                let _ = self.focus_manager.focus_first_without_history_for_restore();
            }
            if self.focus_manager.current().is_some_and(|id| {
                self.focus_manager
                    .graph()
                    .get(id)
                    .map(|node| !node.is_focusable)
                    .unwrap_or(true)
            }) {
                let _ = self.focus_manager.blur();
            }
            self.base_focus = None;
            return;
        }

        if let Some(base_focus) = self.base_focus {
            let _ = self.focus_manager.focus_without_history(base_focus);
        }

        for group_id in group_ids {
            let _ = self.focus_manager.push_trap(group_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Widget;
    use crate::focus::FocusNode;
    use crate::modal::WidgetModalEntry;
    use ftui_core::event::{KeyCode, KeyEvent, KeyEventKind, Modifiers};
    use ftui_core::geometry::Rect;

    #[derive(Debug, Clone)]
    struct StubWidget;

    impl Widget for StubWidget {
        fn render(&self, _area: Rect, _frame: &mut Frame) {}
    }

    fn make_focus_node(id: FocusId) -> FocusNode {
        FocusNode::new(id, Rect::new(0, 0, 10, 3)).with_tab_index(id as i32)
    }

    #[test]
    fn push_with_trap_creates_focus_trap() {
        let mut modals = FocusAwareModalStack::new();

        // Add focusable nodes
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(2));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(3));

        // Focus node 3 before opening modal
        modals.focus_manager_mut().focus(3);
        assert_eq!(modals.focus_manager().current(), Some(3));

        // Push modal with trap containing nodes 1 and 2
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![1, 2]);

        // Focus should now be on node 1 (first in group)
        assert!(modals.is_focus_trapped());
        assert_eq!(modals.focus_manager().current(), Some(1));
    }

    #[test]
    fn pop_restores_focus() {
        let mut modals = FocusAwareModalStack::new();

        // Add focusable nodes
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(2));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(3));

        // Focus node 3 before opening modal
        modals.focus_manager_mut().focus(3);

        // Push modal with trap
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![1, 2]);
        assert_eq!(modals.focus_manager().current(), Some(1));

        // Pop modal - focus should return to node 3
        modals.pop();
        assert!(!modals.is_focus_trapped());
        assert_eq!(modals.focus_manager().current(), Some(3));
    }

    #[test]
    fn pop_skips_closed_modal_focus_ids_when_background_focus_disappears() {
        let mut modals = FocusAwareModalStack::new();
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(50));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(100));

        modals.focus_manager_mut().focus(100);
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![1]);
        let _ = modals.focus_manager_mut().graph_mut().remove(100);

        modals.pop();
        assert_eq!(modals.focus_manager().current(), Some(50));
        assert!(!modals.is_focus_trapped());
    }

    #[test]
    fn nested_modals_restore_correctly() {
        let mut modals = FocusAwareModalStack::new();

        // Add focusable nodes
        for id in 1..=6 {
            modals
                .focus_manager_mut()
                .graph_mut()
                .insert(make_focus_node(id));
        }

        // Initial focus
        modals.focus_manager_mut().focus(1);

        // First modal traps to nodes 2, 3
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![2, 3]);
        assert_eq!(modals.focus_manager().current(), Some(2));

        // Second modal traps to nodes 4, 5, 6
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![4, 5, 6]);
        assert_eq!(modals.focus_manager().current(), Some(4));

        // Pop second modal - back to first modal's focus (node 2)
        modals.pop();
        assert_eq!(modals.focus_manager().current(), Some(2));

        // Pop first modal - back to original focus (node 1)
        modals.pop();
        assert_eq!(modals.focus_manager().current(), Some(1));
        assert!(!modals.is_focus_trapped());
    }

    #[test]
    fn pop_restores_none_when_modal_opened_without_focus() {
        let mut modals = FocusAwareModalStack::new();
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));

        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![1]);
        assert_eq!(modals.focus_manager().current(), Some(1));

        modals.pop();
        assert_eq!(modals.focus_manager().current(), None);
        assert!(!modals.is_focus_trapped());
    }

    #[test]
    fn handle_event_escape_restores_focus() {
        let mut modals = FocusAwareModalStack::new();

        // Add focusable nodes
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(2));

        // Focus node 2
        modals.focus_manager_mut().focus(2);

        // Push modal
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![1]);
        assert_eq!(modals.focus_manager().current(), Some(1));

        // Escape closes modal
        let escape = Event::Key(KeyEvent {
            code: KeyCode::Escape,
            modifiers: Modifiers::empty(),
            kind: KeyEventKind::Press,
        });

        let result = modals.handle_event(&escape, None);
        assert!(result.is_some());
        assert_eq!(modals.focus_manager().current(), Some(2));
    }

    #[test]
    fn handle_event_focus_loss_blurs_current_focus() {
        let mut modals = FocusAwareModalStack::new();
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));
        modals.focus_manager_mut().focus(1);
        let _ = modals.focus_manager_mut().take_focus_event();

        let result = modals.handle_event(&Event::Focus(false), None);
        assert!(result.is_none());
        assert_eq!(modals.focus_manager().current(), None);
        assert_eq!(
            modals.focus_manager_mut().take_focus_event(),
            Some(crate::focus::FocusEvent::FocusLost { id: 1 })
        );
    }

    #[test]
    fn handle_event_focus_gain_restores_trapped_focus() {
        let mut modals = FocusAwareModalStack::new();
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(2));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(3));
        modals.focus_manager_mut().focus(3);

        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![1, 2]);
        assert_eq!(modals.focus_manager().current(), Some(1));

        let _ = modals.handle_event(&Event::Focus(false), None);
        assert_eq!(modals.focus_manager().current(), None);

        let result = modals.handle_event(&Event::Focus(true), None);
        assert!(result.is_none());
        assert_eq!(modals.focus_manager().current(), Some(1));
    }

    #[test]
    fn push_without_trap_no_focus_change() {
        let mut modals = FocusAwareModalStack::new();

        // Add focusable nodes
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(2));

        // Focus node 2
        modals.focus_manager_mut().focus(2);

        // Push modal without trap
        modals.push(Box::new(WidgetModalEntry::new(StubWidget)));

        // Focus should not change
        assert!(!modals.is_focus_trapped());
        assert_eq!(modals.focus_manager().current(), Some(2));
    }

    #[test]
    fn pop_all_restores_all_focus() {
        let mut modals = FocusAwareModalStack::new();

        // Add focusable nodes
        for id in 1..=4 {
            modals
                .focus_manager_mut()
                .graph_mut()
                .insert(make_focus_node(id));
        }

        // Initial focus
        modals.focus_manager_mut().focus(1);

        // Push multiple modals
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![2]);
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![3]);
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![4]);

        assert_eq!(modals.depth(), 3);
        assert_eq!(modals.focus_manager().current(), Some(4));

        // Pop all
        let results = modals.pop_all();
        assert_eq!(results.len(), 3);
        assert!(modals.is_empty());
        assert!(!modals.is_focus_trapped());
        assert_eq!(modals.focus_manager().current(), Some(1));
    }

    #[test]
    fn pop_id_restores_none_when_last_modal_opened_without_focus() {
        let mut modals = FocusAwareModalStack::new();
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));

        let modal_id = modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![1]);
        assert_eq!(modals.focus_manager().current(), Some(1));

        let _ = modals.pop_id(modal_id);
        assert_eq!(modals.focus_manager().current(), None);
        assert!(!modals.is_focus_trapped());
    }

    #[test]
    fn tab_navigation_trapped_in_modal() {
        let mut modals = FocusAwareModalStack::new();

        // Add focusable nodes
        for id in 1..=5 {
            modals
                .focus_manager_mut()
                .graph_mut()
                .insert(make_focus_node(id));
        }

        // Push modal with nodes 2 and 3
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![2, 3]);

        // Focus should be on 2
        assert_eq!(modals.focus_manager().current(), Some(2));

        // Tab forward should go to 3
        modals.focus_manager_mut().focus_next();
        assert_eq!(modals.focus_manager().current(), Some(3));

        // Tab forward should wrap to 2 (trapped)
        modals.focus_manager_mut().focus_next();
        assert_eq!(modals.focus_manager().current(), Some(2));

        // Attempt to focus outside trap should fail
        assert!(modals.focus_manager_mut().focus(5).is_none());
        assert_eq!(modals.focus_manager().current(), Some(2));
    }

    #[test]
    fn empty_focus_group_no_panic() {
        let mut modals = FocusAwareModalStack::new();

        // Push modal with empty focus group (edge case).
        // The trap is NOT pushed because the group has no focusable members,
        // preventing a deadlock where no widget could receive focus.
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![]);

        // Should not panic, and focus should NOT be trapped (empty group).
        assert!(!modals.is_focus_trapped());

        // Pop should still work
        modals.pop();
        assert!(!modals.is_focus_trapped());
    }

    #[test]
    fn rejected_empty_trap_does_not_leave_focus_group_behind() {
        let mut modals = FocusAwareModalStack::new();
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));
        modals.focus_manager_mut().focus(1);
        let group_count_before = modals.focus_manager().group_count();

        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![]);

        assert!(!modals.is_focus_trapped());
        assert_eq!(modals.focus_manager().group_count(), group_count_before);
        assert_eq!(modals.focus_manager().current(), Some(1));
    }

    #[test]
    fn push_with_trap_does_not_collide_with_existing_group_ids() {
        let mut modals = FocusAwareModalStack::new();
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(99));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(100));

        let reserved_group_id = FOCUS_GROUP_COUNTER.load(Ordering::Relaxed);
        modals
            .focus_manager_mut()
            .create_group(reserved_group_id, vec![99]);
        modals.focus_manager_mut().focus(100);

        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![1]);
        let _ = modals.pop().unwrap();

        assert!(modals.focus_manager_mut().push_trap(reserved_group_id));
        assert_eq!(modals.focus_manager().current(), Some(99));
    }

    #[test]
    fn pop_id_non_top_modal_rebuilds_focus_traps() {
        let mut modals = FocusAwareModalStack::new();

        // Add focusable nodes
        for id in 1..=6 {
            modals
                .focus_manager_mut()
                .graph_mut()
                .insert(make_focus_node(id));
        }

        // Initial focus
        modals.focus_manager_mut().focus(1);

        // Push three modals with focus traps.
        let id1 = modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![2]);
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![3]);
        let _id3 = modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![4]);

        // Focus should be on node 4 (top modal)
        assert_eq!(modals.focus_manager().current(), Some(4));

        // Pop the BOTTOM modal (id1) by ID - this is non-LIFO.
        modals.pop_id(id1);

        // Focus should still be on the top modal.
        assert_eq!(modals.focus_manager().current(), Some(4));
        assert_eq!(modals.depth(), 2);
        assert!(modals.is_focus_trapped());

        // Pop remaining modals normally. Focus should restore as if the removed modal never
        // existed: top -> next modal -> original background focus.
        modals.pop();
        assert_eq!(modals.focus_manager().current(), Some(3));

        modals.pop();
        assert_eq!(modals.focus_manager().current(), Some(1));
        assert!(modals.is_empty());
        assert!(!modals.is_focus_trapped());
    }

    #[test]
    fn pop_id_middle_modal_retargets_upper_return_focus() {
        let mut modals = FocusAwareModalStack::new();

        for id in 1..=6 {
            modals
                .focus_manager_mut()
                .graph_mut()
                .insert(make_focus_node(id));
        }

        modals.focus_manager_mut().focus(1);

        let _id1 = modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![2]);
        let id2 = modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![3]);
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![4]);

        assert_eq!(modals.focus_manager().current(), Some(4));

        // Remove the middle modal. The top modal should now restore to modal1's focus.
        modals.pop_id(id2);
        assert_eq!(modals.focus_manager().current(), Some(4));
        assert_eq!(modals.depth(), 2);

        modals.pop();
        assert_eq!(modals.focus_manager().current(), Some(2));

        modals.pop();
        assert_eq!(modals.focus_manager().current(), Some(1));
        assert!(!modals.is_focus_trapped());
    }

    #[test]
    fn pop_id_rebuild_does_not_pollute_focus_history() {
        let mut modals = FocusAwareModalStack::new();

        for id in 1..=6 {
            modals
                .focus_manager_mut()
                .graph_mut()
                .insert(make_focus_node(id));
        }

        modals.focus_manager_mut().focus(1);
        modals.focus_manager_mut().focus(6);

        let id1 = modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![2]);
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![3]);

        modals.pop_id(id1);
        assert_eq!(modals.focus_manager().current(), Some(3));

        modals.pop();
        assert_eq!(modals.focus_manager().current(), Some(6));
        assert!(modals.focus_manager_mut().focus_back());
        assert_eq!(modals.focus_manager().current(), Some(1));
        assert!(!modals.focus_manager_mut().focus_back());
    }

    #[test]
    fn pop_id_top_modal_restores_focus_correctly() {
        let mut modals = FocusAwareModalStack::new();

        // Add focusable nodes
        for id in 1..=4 {
            modals
                .focus_manager_mut()
                .graph_mut()
                .insert(make_focus_node(id));
        }

        // Initial focus
        modals.focus_manager_mut().focus(1);

        // Push two modals
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![2]);
        let id2 = modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![3]);

        assert_eq!(modals.focus_manager().current(), Some(3));

        // Pop the TOP modal by ID - this should work correctly
        modals.pop_id(id2);

        // Focus should restore to modal1's focus (2)
        assert_eq!(modals.focus_manager().current(), Some(2));
        assert!(modals.is_focus_trapped()); // Still in modal1's trap

        // Pop the last modal
        modals.pop();
        assert_eq!(modals.focus_manager().current(), Some(1));
        assert!(!modals.is_focus_trapped());
    }

    #[test]
    fn pop_removes_closed_modal_focus_group() {
        let mut modals = FocusAwareModalStack::new();
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(2));

        modals.focus_manager_mut().focus(1);
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![2]);

        let result = modals.pop().unwrap();
        let group_id = result.focus_group_id.unwrap();

        assert!(!modals.focus_manager_mut().push_trap(group_id));
        assert!(!modals.is_focus_trapped());
        assert_eq!(modals.focus_manager().current(), Some(1));
    }

    #[test]
    fn pop_last_modal_clears_invalid_stale_focus_when_no_fallback_exists() {
        let mut modals = FocusAwareModalStack::new();
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(1));
        modals
            .focus_manager_mut()
            .graph_mut()
            .insert(make_focus_node(2));

        modals.focus_manager_mut().focus(1);
        modals.push_with_trap(Box::new(WidgetModalEntry::new(StubWidget)), vec![2]);
        assert_eq!(modals.focus_manager().current(), Some(2));

        let _ = modals.focus_manager_mut().graph_mut().remove(1);
        let _ = modals.focus_manager_mut().graph_mut().remove(2);

        modals.pop();
        assert_eq!(modals.focus_manager().current(), None);
        assert!(!modals.is_focus_trapped());
    }

    #[test]
    fn default_creates_empty_stack() {
        let modals = FocusAwareModalStack::default();
        assert!(modals.is_empty());
        assert_eq!(modals.depth(), 0);
        assert!(!modals.is_focus_trapped());
    }

    #[test]
    fn with_focus_manager_uses_provided() {
        let mut fm = FocusManager::new();
        fm.graph_mut().insert(make_focus_node(42));
        fm.focus(42);

        let modals = FocusAwareModalStack::with_focus_manager(fm);
        assert!(modals.is_empty());
        assert_eq!(modals.focus_manager().current(), Some(42));
    }

    #[test]
    fn stack_accessors() {
        let mut modals = FocusAwareModalStack::new();
        assert!(modals.stack().is_empty());
        modals.push(Box::new(WidgetModalEntry::new(StubWidget)));
        assert!(!modals.stack().is_empty());
        assert_eq!(modals.stack_mut().depth(), 1);
    }

    #[test]
    fn depth_tracks_push_pop() {
        let mut modals = FocusAwareModalStack::new();
        assert_eq!(modals.depth(), 0);
        modals.push(Box::new(WidgetModalEntry::new(StubWidget)));
        assert_eq!(modals.depth(), 1);
        modals.push(Box::new(WidgetModalEntry::new(StubWidget)));
        assert_eq!(modals.depth(), 2);
        modals.pop();
        assert_eq!(modals.depth(), 1);
    }

    #[test]
    fn pop_empty_stack_returns_none() {
        let mut modals = FocusAwareModalStack::new();
        assert!(modals.pop().is_none());
    }
}
