//! CodeEditor component — a textarea editor with a server-rendered gutter
//! (line numbers + dirty marks) and a gated save bar.
//!
//! No scroll-sync machinery: the textarea uses `field-sizing: content`, so it
//! is exactly as tall as its text and the gutter shares one scrolling
//! container in normal flow. Line numbers and dirty marks re-render on the
//! debounced input round-trip; the server owns the working copy, so **save
//! needs no payload** — it persists state the server already has.

use rwire::style::Style;
use rwire::style_tokens::St;
use rwire::{el, At, Av, El, ElementBuilder, Ev, HandlerSpec};

use crate::{Button, ButtonSize};

/// CodeEditor builder.
pub struct CodeEditor<'a> {
    id: &'a str,
    content: &'a str,
    dirty_lines: &'a [bool],
    on_edit: Option<HandlerSpec>,
    save: Option<(HandlerSpec, bool)>,
    overlay: Option<Vec<ElementBuilder>>,
}

impl<'a> CodeEditor<'a> {
    /// `id` keys the textarea across morphs; `content` is the working copy.
    pub fn new(id: &'a str, content: &'a str) -> Self {
        Self {
            id,
            content,
            dirty_lines: &[],
            on_edit: None,
            save: None,
            overlay: None,
        }
    }

    /// Per-line dirty flags (working copy vs baseline); short slices are fine —
    /// missing entries render clean.
    pub fn dirty_lines(mut self, dirty: &'a [bool]) -> Self {
        self.dirty_lines = dirty;
        self
    }

    /// Syntax-colored line rows rendered UNDER a transparent-ink textarea
    /// (e.g. `rwire_markdown::highlight_lines`). Caret, selection, undo, and
    /// IME stay native; the runtime echoes keystrokes into the overlay as
    /// plain text so typing is never invisible, and colors return with the
    /// debounced round-trip. Rows must match the textarea's line grid — the
    /// component pins line-height and min-height on each row.
    pub fn overlay(mut self, lines: Vec<ElementBuilder>) -> Self {
        self.overlay = Some(lines);
        self
    }

    /// Debounced input handler maintaining the server-side working copy.
    pub fn on_edit(mut self, handler: HandlerSpec) -> Self {
        self.on_edit = Some(handler);
        self
    }

    /// Save bar: the button is enabled only while `dirty`.
    pub fn save_bar(mut self, handler: HandlerSpec, dirty: bool) -> Self {
        self.save = Some((handler, dirty));
        self
    }

    pub fn build(self) -> ElementBuilder {
        let lines = self.content.lines().count().max(1);
        let gutter = el(El::Div)
            .st([
                St::DisplayFlex,
                St::FlexCol,
                St::ItemsEnd,
                St::FlexShrink0,
                St::TextXs,
                St::TextLow,
                St::FontMono,
                St::PrSm,
                St::BorderR,
            ])
            .append((1..=lines).map(|n| {
                let dirty = self.dirty_lines.get(n - 1).copied().unwrap_or(false);
                let mut row = el(El::Div)
                    .st([St::DisplayFlex, St::ItemsCenter, St::GapXs])
                    // match the textarea's line box exactly
                    .style(Style::new().set("line-height", "1.5"));
                if dirty {
                    row = row.append([el(El::Span)
                        .st([St::BgWarning, St::RoundedFull, St::DisplayInlineBlock])
                        .style(Style::new().width("0.35rem").height("0.35rem"))]);
                }
                row.append([el(El::Span).text(&n.to_string())])
            }));

        let overlay_id = format!("{}-hl", self.id);
        // Explicit geometry so browsers without field-sizing:content behave
        // identically: rows pins the height (+1 for the phantom line after a
        // trailing newline), ch pins the width (exact in mono), and the CELL
        // scrolls horizontally so textarea and underlay move together.
        let max_cols = self.content.lines().map(str::len).max().unwrap_or(0);
        let layer_w = format!("max(100%, {}ch)", max_cols + 2);
        let mut field = el(El::Textarea)
            .id(self.id)
            .at_str(At::Name, "content")
            .at(At::Spellcheck, Av::False)
            .at(At::Autocomplete, Av::Off)
            .at_str(At::Wrap, "off")
            .st([
                St::FontMono,
                St::TextXs,
                St::BgTransparent,
                St::BorderNone,
                St::OutlineNone,
                St::ResizeNone,
                St::FieldSizingContent,
                St::WhitespacePre,
            ])
            .at_str(At::Rows, &(lines + 1).to_string())
            .style(
                Style::new()
                    .set("line-height", "1.5")
                    .set("padding", "0")
                    .set("overflow", "hidden")
                    .set("width", &layer_w),
            )
            .text(self.content);
        if let Some(handler) = self.on_edit {
            field = field.on(Ev::Input, handler);
        }
        let code_cell = match self.overlay {
            Some(lines) => {
                // Transparent ink over a colored underlay: the two layers share
                // one font grid, so glyphs align; only the caret stays inked.
                field = field
                    .st([
                        St::TextTransparent,
                        St::CaretInk,
                        St::PositionRelative,
                        St::WFull,
                    ])
                    .attr("data-echo", &overlay_id);
                let row_metrics = Style::new()
                    .set("line-height", "1.5")
                    .set("min-height", "1.5em");
                let underlay = el(El::Div)
                    .id(&overlay_id)
                    .at(At::AriaHidden, Av::True)
                    .st([
                        St::AbsoluteFill,
                        St::FontMono,
                        St::TextXs,
                        St::TextDefault,
                        St::WhitespacePre,
                        St::PointerEventsNone,
                    ])
                    .style(Style::new().set("line-height", "1.5"))
                    .style(Style::new().set("width", &layer_w))
                    .append(lines.into_iter().map(|l| l.style(row_metrics.clone())));
                el(El::Div)
                    .st([
                        St::Flex1,
                        St::MinW0,
                        St::PositionRelative,
                        St::OverflowXAuto,
                    ])
                    .append([underlay, field])
            }
            None => field.st([St::Flex1, St::MinW0, St::TextDefault]),
        };

        let mut column: Vec<ElementBuilder> = Vec::new();
        if let Some((handler, dirty)) = self.save {
            column.push(
                el(El::Div)
                    .st([
                        St::DisplayFlex,
                        St::JustifyEnd,
                        St::GapSm,
                        St::PbSm,
                        St::FlexShrink0,
                    ])
                    .append([Button::primary(if dirty { "Save" } else { "Saved" })
                        .size(ButtonSize::Sm)
                        .disabled(!dirty)
                        .on_click(handler)]),
            );
        }
        column.push(
            el(El::Div)
                .st([
                    St::Flex1,
                    St::MinH0,
                    St::OverflowAuto,
                    St::DisplayFlex,
                    St::GapSm,
                ])
                .append([gutter, code_cell]),
        );
        el(El::Div)
            .st([
                St::DisplayFlex,
                St::FlexCol,
                St::Flex1,
                St::MinH0,
                St::MinW0,
            ])
            .append(column)
    }
}
