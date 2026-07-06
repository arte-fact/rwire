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
        }
    }

    /// Per-line dirty flags (working copy vs baseline); short slices are fine —
    /// missing entries render clean.
    pub fn dirty_lines(mut self, dirty: &'a [bool]) -> Self {
        self.dirty_lines = dirty;
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

        let mut field = el(El::Textarea)
            .id(self.id)
            .at_str(At::Name, "content")
            .at(At::Spellcheck, Av::False)
            .at(At::Autocomplete, Av::Off)
            .at_str(At::Wrap, "off")
            .st([
                St::Flex1,
                St::MinW0,
                St::FontMono,
                St::TextXs,
                St::BgTransparent,
                St::TextDefault,
                St::BorderNone,
                St::OutlineNone,
                St::ResizeNone,
                St::FieldSizingContent,
                St::WhitespacePre,
            ])
            .style(Style::new().set("line-height", "1.5"))
            .text(self.content);
        if let Some(handler) = self.on_edit {
            field = field.on(Ev::Input, handler);
        }

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
                .append([gutter, field]),
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
