//! TreeView component.
//!
//! A generic collapsible tree: branches render as native `<details>/<summary>`
//! (zero-latency expand/collapse, no per-node server state, keyboard/a11y for
//! free — the same first-class call as `ChatDetail`), leaves as selectable
//! rows. Selection is server state: leaf rows fire `on_select` with a
//! caller-provided index param (`ctx.item_index()`), so the app highlights the
//! selected row on re-render.
//!
//! [`FileTree`](crate::FileTree) specializes it for filesystem snapshots.

use std::borrow::Cow;

use rwire::style_tokens::St;
use rwire::{el, At, El, ElementBuilder, HandlerSpec};

/// One node of a [`TreeView`].
pub struct TreeNode {
    label: ElementBuilder,
    key: Cow<'static, str>,
    children: Vec<TreeNode>,
    expanded: bool,
    selected: bool,
    on_select: Option<HandlerSpec>,
}

impl TreeNode {
    /// A leaf (selectable when `on_select` is set).
    pub fn leaf(key: impl Into<Cow<'static, str>>, label: ElementBuilder) -> Self {
        Self {
            label,
            key: key.into(),
            children: Vec::new(),
            expanded: false,
            selected: false,
            on_select: None,
        }
    }

    /// A branch with children (collapsible).
    pub fn branch(
        key: impl Into<Cow<'static, str>>,
        label: ElementBuilder,
        children: Vec<TreeNode>,
    ) -> Self {
        Self {
            label,
            key: key.into(),
            children,
            expanded: false,
            selected: false,
            on_select: None,
        }
    }

    /// Expand this branch on first render.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Highlight as the current selection.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Fire `handler` on click. The handler is bound AS GIVEN — encode any
    /// params (an index, an action code) before passing it; TreeView never
    /// rewrites them.
    pub fn on_select(mut self, handler: HandlerSpec) -> Self {
        self.on_select = Some(handler);
        self
    }

    fn build(self) -> ElementBuilder {
        let row_tokens = [
            St::DisplayFlex,
            St::ItemsCenter,
            St::GapSm,
            St::PxSm,
            St::PySm,
            St::RoundedSm,
            St::CursorPointer,
            St::TextSm,
        ];
        let mut row = el(El::Div).st(row_tokens).append([self.label]);
        if self.selected {
            row = row.st([St::BgAccent, St::TextOnEmphasis]);
        } else {
            row = row.hover([St::BgSubtle]);
        }
        if let Some(handler) = self.on_select {
            row = row.on(rwire::Ev::Click, handler);
        }
        if self.children.is_empty() {
            return row.id(format!("tn-{}", self.key).as_str());
        }
        let mut details = el(El::Details)
            .id(format!("tn-{}", self.key).as_str())
            .append([
                el(El::Summary)
                    .st([St::CursorPointer, St::ListStyleNone])
                    .append([row]),
                el(El::Div)
                    .st([St::DisplayFlex, St::FlexCol, St::PlMd])
                    .append(self.children.into_iter().map(TreeNode::build)),
            ]);
        if self.expanded {
            details = details.bool_attr(At::Open);
        }
        details
    }
}

/// TreeView builder.
#[derive(Default)]
pub struct TreeView {
    roots: Vec<TreeNode>,
}

impl TreeView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn roots(mut self, roots: Vec<TreeNode>) -> Self {
        self.roots.extend(roots);
        self
    }

    pub fn build(self) -> ElementBuilder {
        el(El::Div)
            .st([St::DisplayFlex, St::FlexCol, St::MinW0, St::OverflowAuto])
            .append(self.roots.into_iter().map(TreeNode::build))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branches_are_details_and_leaves_are_rows() {
        let tree = TreeView::new()
            .roots(vec![TreeNode::branch(
                "src",
                el(El::Span).text("src"),
                vec![TreeNode::leaf("main", el(El::Span).text("main.rs")).selected(true)],
            )
            .expanded(true)])
            .build();
        let root = &tree.children()[0];
        assert_eq!(root.el_type(), El::Details);
        let leaf = &root.children()[1].children()[0];
        assert_eq!(leaf.el_type(), El::Div);
    }
}
