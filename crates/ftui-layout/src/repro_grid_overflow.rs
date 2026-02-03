#[cfg(test)]
mod tests {
    use super::*;
    use crate::Constraint;
    use ftui_core::geometry::Rect;

    #[test]
    fn grid_gap_overflow_reproduction() {
        // Create a grid with enough rows to cause (rows-1) to truncate when cast to u16.
        // 65537 rows. (65537 - 1) = 65536. 65536 as u16 = 0.
        // So total_gap calculation will result in 0 if the bug exists.
        // If row_gap is 1, correct total_gap is 65536 (saturating to 65535 or larger than available).
        
        let rows = vec![Constraint::Min(1); 65537];
        let grid = Grid::new()
            .rows(rows)
            .columns([Constraint::Min(10)])
            .row_gap(1);

        // Area height 100.
        // Gaps alone need 65536 space.
        // Available space should be 0.
        // All rows should effectively be size 0 (or constraint solver handles 0 available).
        
        // With the bug:
        // (65537 - 1) as u16 = 0.
        // total_gap = 1 * 0 = 0.
        // available = 100 - 0 = 100.
        // Solver distributes 100 among rows. First 100 rows get size 1?
        
        let layout = grid.split(Rect::new(0, 0, 100, 100));
        
        // If available was 0, row_height(0) should be 0.
        // If available was 100, row_height(0) will be 1 (Constraint::Min(1)).
        
        assert_eq!(layout.row_height(0), 0, "Row height should be 0 due to massive gap consumption");
    }
}
