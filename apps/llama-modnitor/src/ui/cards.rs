//! Hardware status tiles for the top strip.
//!
//! Compact CPU/GPU readouts (name, temperature, load, memory/VRAM) that flex
//! into a horizontal row and wrap onto more lines on narrow screens. Each tile
//! is wrapped by [`super::selectable`] to drive node-creation selection.

// Rust guideline compliant 2026-02-21

use rwire::{El, ElementBuilder, St, Style, el};
use rwire_components::{Badge, Gap, Progress, Stack, StackJustify, Text};

use crate::convert;
use crate::snapshot::{GpuCard, HardwareSnapshot};

/// Compact tile shell: a header row, then the supplied metric rows. Flexes to
/// share the strip width and shrinks (never overflows) on narrow screens.
fn tile(header: ElementBuilder, body: Vec<ElementBuilder>) -> ElementBuilder {
    let mut rows = vec![header];
    rows.extend(body);
    el(El::Div)
        .st([St::PSm, St::RoundedLg])
        .style(
            Style::new()
                .background("var(--b)")
                .border("1px solid var(--g)")
                .set("flex", "1 1 13rem")
                .min_width("0"),
        )
        .append([Stack::column().gap(Gap::Xs).children(rows).build()])
}

/// Tile header: a truncating name on the left, temperature badge on the right.
fn tile_header(name: &str, temp: Option<f32>) -> ElementBuilder {
    let mut right = Vec::new();
    if let Some(t) = temp {
        right.push(temp_badge(t));
    }
    Stack::row()
        .justify(StackJustify::Between)
        .align_center()
        .children([
            el(El::Span)
                .st([St::Truncate])
                .style(Style::new().set("font-weight", "600").min_width("0"))
                .text(name),
            Stack::row().gap(Gap::Xs).children(right).build(),
        ])
        .build()
}

/// Compact status tile for the host CPU and system memory.
pub fn cpu_tile(snap: &HardwareSnapshot) -> ElementBuilder {
    tile(
        tile_header("CPU", snap.cpu_temp),
        vec![
            caption_line(&snap.cpu_model),
            meter(
                "Load",
                format!("{:.0}%", snap.cpu_usage),
                snap.cpu_usage,
                100.0,
            ),
            caption_line(&format!(
                "{} / {} \u{b7} {} cores",
                fmt_kib(snap.mem.used_kib),
                fmt_kib(snap.mem.total_kib),
                snap.cpu_cores.len()
            )),
        ],
    )
}

/// Compact status tile for a single GPU.
pub fn gpu_tile(gpu: &GpuCard) -> ElementBuilder {
    let m = &gpu.metrics;
    tile(
        tile_header(short_name(&gpu.name), Some(m.temp)),
        vec![
            meter(
                "Load",
                format!("{}%", m.load),
                convert::u32_f32(m.load),
                100.0,
            ),
            caption_line(&format!(
                "{} / {} VRAM",
                fmt_mib(m.vram_used),
                fmt_mib(m.vram_total)
            )),
            caption_line(&format!(
                "{:.0}/{} W \u{b7} {}/{} MHz",
                m.power_consumption, m.power_limit, m.sclk_mhz, m.mclk_mhz
            )),
        ],
    )
}

/// A muted, truncating one-line caption (keeps long values inside the tile).
fn caption_line(text: &str) -> ElementBuilder {
    el(El::Div)
        .st([St::Truncate])
        .style(
            Style::new()
                .set("color", "var(--i)")
                .set("font-size", "0.75rem"),
        )
        .text(text)
}

/// Shorten a GPU name to its distinguishing tail, e.g. the `cardN` in
/// `"AMD Instinct MI50/MI60 (card0)"`; names without parentheses pass through.
fn short_name(full: &str) -> &str {
    if let Some(open) = full.rfind('(') {
        let rest = &full[open + 1..];
        if let Some(close) = rest.find(')') {
            return &rest[..close];
        }
    }
    full
}

/// A labeled progress bar: a label/value row above the bar.
pub fn meter(label: &str, value_text: String, value: f32, max: f32) -> ElementBuilder {
    // Progress is integer-valued; round and guard against a zero maximum.
    let value = convert::f32_u32(value.round().max(0.0));
    let max = convert::f32_u32(max.round()).max(1);
    Stack::column()
        .gap(Gap::Xs)
        .children([
            Stack::row()
                .justify(StackJustify::Between)
                .children([
                    Text::label(label.to_string()).build(),
                    Text::body_small(value_text).build(),
                ])
                .build(),
            Progress::new().value(value.min(max)).max(max).build(),
        ])
        .build()
}

/// Temperature badge colored by severity.
pub fn temp_badge(temp: f32) -> ElementBuilder {
    let text = format!("{temp:.0}\u{b0}C");
    // Thresholds: GPUs typically throttle near 90-110C; warn earlier for headroom.
    if temp >= 85.0 {
        Badge::error(text).build()
    } else if temp >= 70.0 {
        Badge::warning(text).build()
    } else {
        Badge::success(text).build()
    }
}

/// Format mebibytes as gibibytes with one decimal place.
pub fn fmt_mib(mib: u64) -> String {
    format!("{:.1} GiB", convert::u64_f64(mib) / 1024.0)
}

/// Format kibibytes as gibibytes with one decimal place.
pub fn fmt_kib(kib: u64) -> String {
    format!("{:.1} GiB", convert::u64_f64(kib) / 1024.0 / 1024.0)
}
