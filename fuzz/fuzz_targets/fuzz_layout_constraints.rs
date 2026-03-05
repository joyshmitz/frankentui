#![no_main]

use ftui_layout::{Constraint, Flex, Rect};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Need at least 6 bytes: 2 for area dimensions, then constraint pairs.
    if data.len() < 6 {
        return;
    }

    let width = (data[0] as u16).max(1);
    let height = (data[1] as u16).max(1);
    let gap = data[2] as u16 % 10;
    let direction_byte = data[3];
    let payload = &data[4..];

    // Build constraints from remaining bytes (2 bytes each: type + value).
    let mut constraints = Vec::new();
    let mut i = 0;
    while i + 1 < payload.len() && constraints.len() < 32 {
        let kind = payload[i] % 9;
        let val = payload[i + 1] as u16;
        let constraint = match kind {
            0 => Constraint::Fixed(val),
            1 => Constraint::Percentage(val as f32),
            2 => Constraint::Min(val),
            3 => Constraint::Max(val),
            4 => Constraint::Ratio(val as u32, (payload[i + 1] as u32).max(1)),
            5 => Constraint::Fill,
            6 => Constraint::FitContent,
            7 => Constraint::FitContentBounded {
                min: val.min(128),
                max: val.max(val.min(128)),
            },
            8 => Constraint::FitMin,
            _ => unreachable!(),
        };
        constraints.push(constraint);
        i += 2;
    }

    if constraints.is_empty() {
        return;
    }

    let area = Rect::new(0, 0, width, height);

    // Test vertical layout.
    let flex_v = Flex::vertical().constraints(constraints.iter().copied()).gap(gap);
    let rects_v = flex_v.split(area);
    validate_rects(&rects_v, area, constraints.len());

    // Test horizontal layout.
    let flex_h = Flex::horizontal().constraints(constraints.iter().copied()).gap(gap);
    let rects_h = flex_h.split(area);
    validate_rects(&rects_h, area, constraints.len());

    // Test with direction toggle.
    let flex = if direction_byte % 2 == 0 {
        Flex::vertical()
    } else {
        Flex::horizontal()
    }
    .constraints(constraints.iter().copied())
    .gap(gap);
    let rects = flex.split(area);
    validate_rects(&rects, area, constraints.len());
});

fn validate_rects(rects: &[Rect], _area: Rect, expected_count: usize) {
    // Must produce exactly one rect per constraint.
    assert_eq!(
        rects.len(),
        expected_count,
        "rect count mismatch: got {}, expected {}",
        rects.len(),
        expected_count
    );

    for (i, r) in rects.iter().enumerate() {
        // No rect should have dimensions that overflow u16.
        assert!(
            r.x.checked_add(r.width).is_some(),
            "rect[{i}] x+width overflows: x={}, width={}",
            r.x,
            r.width
        );
        assert!(
            r.y.checked_add(r.height).is_some(),
            "rect[{i}] y+height overflows: y={}, height={}",
            r.y,
            r.height
        );
    }
}
