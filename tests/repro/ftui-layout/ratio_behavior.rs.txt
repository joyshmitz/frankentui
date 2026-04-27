#[cfg(test)]
mod tests {
    use crate::{Constraint, Flex, Rect};

    #[test]
    fn ratio_vs_percentage_behavior() {
        let area = Rect::new(0, 0, 100, 10);

        // Case 1: Percentage(25) alone
        // Expectation: Takes 25, leaves 75 empty
        let flex_p = Flex::horizontal().constraints([Constraint::Percentage(25.0)]);
        let rects_p = flex_p.split(area);
        assert_eq!(rects_p[0].width, 25, "Percentage(25) should take 25%");

        // Case 2: Ratio(1, 4) alone
        // Expectation: Should match Percentage(25) -> Takes 25, leaves 75
        // Actual (suspected): Takes 100 (acts like Fill with weight 0.25, but sole survivor)
        let flex_r = Flex::horizontal().constraints([Constraint::Ratio(1, 4)]);
        let rects_r = flex_r.split(area);
        
        // This assertion will likely fail if Ratio acts like Fill
        assert_eq!(rects_r[0].width, 25, "Ratio(1, 4) should take 25% (got {})", rects_r[0].width);
    }

    #[test]
    fn ratio_vs_fill_interaction() {
        let area = Rect::new(0, 0, 100, 10);

        // Case: Ratio(1, 4) vs Fill
        // If Ratio is fixed size: Ratio takes 25, Fill takes 75.
        // If Ratio is weight 0.25: Fill is weight 1.0. Total 1.25.
        // Ratio gets 0.25/1.25 = 1/5 = 20. Fill gets 4/5 = 80.
        let flex = Flex::horizontal().constraints([Constraint::Ratio(1, 4), Constraint::Fill]);
        let rects = flex.split(area);

        // We assume the user intends Ratio(1, 4) to mean "1/4th of space", i.e. 25.
        assert_eq!(rects[0].width, 25, "Ratio(1, 4) should be fixed 25%, got {}", rects[0].width);
        assert_eq!(rects[1].width, 75, "Fill should take remainder 75%, got {}", rects[1].width);
    }
}
