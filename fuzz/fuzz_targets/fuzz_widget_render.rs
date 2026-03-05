#![no_main]

use ftui_core::geometry::Rect;
use ftui_render::frame::Frame;
use ftui_render::grapheme_pool::GraphemePool;
use ftui_widgets::block::Block;
use ftui_widgets::borders::Borders;
use ftui_widgets::paragraph::Paragraph;
use ftui_widgets::progress::ProgressBar;
use ftui_widgets::sparkline::Sparkline;
use ftui_widgets::Widget;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Need at least 4 bytes: width, height, widget selector, payload.
    if data.len() < 4 {
        return;
    }

    let width = ((data[0] as u16) % 200).max(1);
    let height = ((data[1] as u16) % 60).max(1);
    let widget_kind = data[2] % 5;
    let payload = &data[3..];

    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(width, height, &mut pool);
    let area = Rect::new(0, 0, width, height);

    match widget_kind {
        0 => {
            // Block with various border combos.
            let borders = Borders::from_bits_truncate(payload.first().copied().unwrap_or(0));
            let block = Block::new().borders(borders);
            block.render(area, &mut frame);
        }
        1 => {
            // Paragraph with arbitrary UTF-8 text.
            let text = String::from_utf8_lossy(payload);
            let para = Paragraph::new(text.as_ref());
            para.render(area, &mut frame);
        }
        2 => {
            // Sparkline with arbitrary f64 data derived from bytes.
            let values: Vec<f64> = payload.iter().map(|&b| b as f64).collect();
            if !values.is_empty() {
                let spark = Sparkline::new(&values);
                spark.render(area, &mut frame);
            }
        }
        3 => {
            // ProgressBar with arbitrary ratio.
            let ratio = if payload.is_empty() {
                0.5
            } else {
                payload[0] as f64 / 255.0
            };
            let bar = ProgressBar::new().ratio(ratio);
            bar.render(area, &mut frame);
        }
        4 => {
            // Block containing paragraph (nested render).
            let text = String::from_utf8_lossy(payload);
            let block = Block::bordered();
            let inner = block.inner(area);
            block.render(area, &mut frame);
            if inner.width > 0 && inner.height > 0 {
                let para = Paragraph::new(text.as_ref());
                para.render(inner, &mut frame);
            }
        }
        _ => unreachable!(),
    }

    // Post-condition: buffer dimensions unchanged.
    assert_eq!(frame.buffer.width(), width, "buffer width changed");
    assert_eq!(frame.buffer.height(), height, "buffer height changed");
});
