//! SplitPane component — two panes side by side with a pointer-drag divider
//! (the runtime's `BIND_RESIZE` primitive: dragging resizes the left pane,
//! entirely client-side).

use std::borrow::Cow;

use rwire::style::Style;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};

/// SplitPane builder.
pub struct SplitPane {
    left: ElementBuilder,
    right: ElementBuilder,
    initial: Cow<'static, str>,
}

impl SplitPane {
    pub fn new(left: ElementBuilder, right: ElementBuilder) -> Self {
        Self {
            left,
            right,
            initial: Cow::Borrowed("18rem"),
        }
    }

    /// Initial width of the left pane (CSS length). Default `18rem`.
    pub fn initial(mut self, width: impl Into<Cow<'static, str>>) -> Self {
        self.initial = width.into();
        self
    }

    pub fn build(self) -> ElementBuilder {
        el(El::Div)
            .st([St::DisplayFlex, St::Flex1, St::MinH0, St::MinW0])
            .append([
                // Panes are flex columns so their content can Flex1 to full
                // height (a block pane would collapse to content height).
                el(El::Div)
                    .st([
                        St::MinW0,
                        St::OverflowAuto,
                        St::FlexShrink0,
                        St::DisplayFlex,
                        St::FlexCol,
                    ])
                    // initial width is caller data; the drag overwrites it in px
                    .style(Style::new().width(self.initial.as_ref()))
                    .append([self.left]),
                // The divider: a hairline the drag primitive widens on hover.
                el(El::Div)
                    .resize_handle()
                    .st([
                        St::FlexShrink0,
                        St::CursorColResize,
                        St::BgMuted,
                        St::SelfStretch,
                    ])
                    .style(Style::new().width("3px"))
                    .hover([St::BgAccent]),
                el(El::Div)
                    .st([
                        St::Flex1,
                        St::MinW0,
                        St::MinH0,
                        St::OverflowAuto,
                        St::DisplayFlex,
                        St::FlexCol,
                    ])
                    .append([self.right]),
            ])
    }
}
