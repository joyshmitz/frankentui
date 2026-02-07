
use ftui_layout::{Constraint, Flex, Rect};

#[test]
fn ratio_vs_percentage_behavior() {
    let area = Rect::new(0, 0, 100, 10);

    // Case 1: Percentage(25) alone
    let flex_p = Flex::horizontal().constraints([Constraint::Percentage(25.0)]);
    let rects_p = flex_p.split(area);
    assert_eq!(rects_p[0].width, 25, "Percentage(25) should take 25%");

    // Case 2: Ratio(1, 4) alone
    let flex_r = Flex::horizontal().constraints([Constraint::Ratio(1, 4)]);
    let rects_r = flex_r.split(area);
    
    assert_eq!(rects_r[0].width, 25, "Ratio(1, 4) should take 25% (got {})", rects_r[0].width);
}

#[test]
fn ratio_vs_fill_interaction() {
    let area = Rect::new(0, 0, 100, 10);

    // Case: Ratio(1, 4) vs Fill
    let flex = Flex::horizontal().constraints([Constraint::Ratio(1, 4), Constraint::Fill]);
    let rects = flex.split(area);

    assert_eq!(rects[0].width, 25, "Ratio(1, 4) should be fixed 25%, got {}", rects[0].width);
    assert_eq!(rects[1].width, 75, "Fill should take remainder 75%, got {}", rects[1].width);
}

fn main() {
    ratio_vs_percentage_behavior();
    ratio_vs_fill_interaction();
    println!("Constraint::Ratio tests passed!");
}
