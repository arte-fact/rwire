//! FileTree component — [`TreeView`](crate::TreeView) specialized for a
//! filesystem snapshot ([`FsSnapshot`](crate::FsSnapshot)): folder/file icons,
//! selection by entry index, branches expanded to reveal the selection.

use rwire::style_tokens::St;
use rwire::{el, icons, El, ElementBuilder, HandlerSpec};

use crate::fs_source::FsEntry;
use crate::{TreeNode, TreeView};

/// FileTree builder.
pub struct FileTree<'a> {
    entries: &'a [FsEntry],
    selected: Option<usize>,
    on_select: Option<HandlerSpec>,
    expand_all: bool,
}

impl<'a> FileTree<'a> {
    pub fn new(entries: &'a [FsEntry]) -> Self {
        Self {
            entries,
            selected: None,
            on_select: None,
            expand_all: false,
        }
    }

    /// Highlight the entry at this index (into the snapshot's `entries`).
    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    /// Fire on file selection with the entry index (`ctx.item_index()`).
    pub fn on_select(mut self, handler: HandlerSpec) -> Self {
        self.on_select = Some(handler);
        self
    }

    /// Expand every branch on first render (small trees).
    pub fn expand_all(mut self) -> Self {
        self.expand_all = true;
        self
    }

    fn label(entry: &FsEntry, selected: bool) -> ElementBuilder {
        let glyph = if entry.is_dir {
            icons::icon_sized(icons::Icon::Folder, 14)
        } else {
            icons::icon_sized(icons::Icon::FileText, 14)
        };
        el(El::Span)
            .st([St::DisplayFlex, St::ItemsCenter, St::GapSm, St::MinW0])
            .append([
                el(El::Span).st([St::TextMuted]).append([glyph]),
                el(El::Span)
                    .st(if selected {
                        [St::TextSm, St::TextHigh]
                    } else {
                        [St::TextSm, St::TextDefault]
                    })
                    .text(&entry.name),
            ])
    }

    /// Build one depth level: consumes entries until depth drops.
    fn level(&self, idx: &mut usize, depth: usize) -> Vec<TreeNode> {
        let mut nodes = Vec::new();
        while *idx < self.entries.len() {
            let entry = &self.entries[*idx];
            if entry.depth < depth {
                break;
            }
            let i = *idx;
            *idx += 1;
            let is_selected = self.selected == Some(i);
            if entry.is_dir {
                let children = self.level(idx, depth + 1);
                let contains_selection = self.selected.is_some_and(|s| s > i && s < *idx);
                nodes.push(
                    TreeNode::branch(entry.rel.clone(), Self::label(entry, false), children)
                        .expanded(self.expand_all || contains_selection),
                );
            } else {
                let mut node = TreeNode::leaf(entry.rel.clone(), Self::label(entry, is_selected))
                    .selected(is_selected);
                if let Some(h) = &self.on_select {
                    // Encode the entry index so handlers read ctx.item_index().
                    let mut params = Vec::new();
                    rwire::ItemRef::<()>::new(i).encode(&mut params);
                    node = node.on_select(h.clone().with_param_bytes(params));
                }
                nodes.push(node);
            }
        }
        nodes
    }

    pub fn build(self) -> ElementBuilder {
        let mut idx = 0;
        let roots = self.level(&mut idx, 0);
        TreeView::new().roots(roots).build()
    }
}
