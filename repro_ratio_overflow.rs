#[cfg(test)]
mod tests {
    use ftui_layout::{Constraint, Flex};
    use ftui_core::geometry::Rect;

    #[test]
    fn ratio_overflow_repro() {
        // Available space 40000.
        // Ratio 2/1 means target 80000.
        // 80000 as u16 wraps to 14464.
        // Expectation: saturates to 40000 (available).
        
        let flex = Flex::horizontal().constraints([Constraint::Ratio(2, 1)]);
        let rects = flex.split(Rect::new(0, 0, 40000, 1));
        
        assert_eq!(rects[0].width, 40000, "Ratio(2,1) should fill available space, not wrap");
    }
}
