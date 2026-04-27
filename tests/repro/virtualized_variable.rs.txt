use ftui_widgets::virtualized::{ItemHeight, Virtualized};

#[test]
fn test_virtualized_variable_fenwick_dynamic_push() {
    // 1. Create Virtualized with VariableFenwick strategy
    let mut virt: Virtualized<String> = Virtualized::new(100)
        .with_variable_heights_fenwick(1, 100); // default height 1

    // 2. Push items
    virt.push("Item 1".to_string());
    virt.push("Item 2".to_string());

    // 3. Check visible range for viewport height 10
    // Expected: 2 items visible (height 1+1 = 2 < 10)
    let range = virt.visible_range(10);
    
    // CURRENT BEHAVIOR (Hypothesis):
    // VariableHeightsFenwick was initialized with capacity 100 but len 100? 
    // No, VariableHeightsFenwick::new(default, capacity) sets len=capacity.
    // Wait, let's check the constructor.
    
    // If len=100, then it thinks it has 100 items of height 1.
    // The storage has 2 items.
    // visible_range uses tracker.visible_count.
    // tracker.visible_count might return 10 (fits 10 items).
    // range is start..min(start+visible, len).
    // len is 2.
    // So 0..2.
    
    // BUT, what if I initialized with capacity 0?
    let mut virt_empty: Virtualized<String> = Virtualized::new(100)
        .with_variable_heights_fenwick(1, 0);
    
    virt_empty.push("Item 1".to_string());
    
    // Tracker len is 0. visible_count returns 0.
    // Range is 0..0.
    // Item 1 is NOT rendered.
    let range_empty = virt_empty.visible_range(10);
    assert_eq!(range_empty, 0..1, "Should show 1 item if tracker updated, but shows {:?}", range_empty);
}
