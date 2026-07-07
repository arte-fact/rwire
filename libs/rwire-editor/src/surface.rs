//! The FileEditor surface, per the locked design mock: resizable split with
//! the tree pane (managed affordances inline), a breadcrumb toolbar with the
//! dirty chip and mode control, inline pending banners (guard / conflict /
//! delete), the gutter editor or rendered view, and a status bar carrying the
//! autosave toggle. The Cmd/Ctrl+S trigger (`data-save-key`) is a visible
//! Save button in manual mode and a hidden span under autosave.

use rwire::style::Style;
use rwire::style_tokens::St;
use rwire::{el, icons, At, Av, El, ElementBuilder, Ev, HandlerSpec};
use rwire_components::{
    Button, ButtonSize, Chip, CodeEditor, FsEntry, FsSnapshot, SplitPane, Switch, Tooltip,
    TreeNode, TreeView,
};
use rwire_markdown::{highlight_lines, Markdown};

use crate::state::{Action, FileEditorState, FileKind, Pending};

/// FileEditor builder — render an embedded [`FileEditorState`].
pub struct FileEditor<'a> {
    state: &'a FileEditorState,
    snap: &'a FsSnapshot,
    on_event: Option<HandlerSpec>,
    managed: bool,
}

impl<'a> FileEditor<'a> {
    pub fn new(state: &'a FileEditorState, snap: &'a FsSnapshot) -> Self {
        Self {
            state,
            snap,
            on_event: None,
            managed: false,
        }
    }

    /// The single dispatcher handler (route it to [`FileEditorState::apply`]).
    pub fn on_event(mut self, handler: HandlerSpec) -> Self {
        self.on_event = Some(handler);
        self
    }

    /// Enable create / rename / delete affordances.
    pub fn managed(mut self, managed: bool) -> Self {
        self.managed = managed;
        self
    }

    fn act(&self, action: Action, index: Option<u32>) -> HandlerSpec {
        self.on_event
            .clone()
            .expect("FileEditor::on_event is required")
            .with_param_bytes(action.params(index))
    }

    fn chip_action(&self, label: &str, action: Action, index: Option<u32>) -> ElementBuilder {
        Chip::new(label.to_string())
            .on_click(self.act(action, index))
            .build()
    }

    /// A small icon button with a tooltip — the tree affordances.
    fn icon_action(
        &self,
        icon: icons::Icon,
        tip: &str,
        action: Action,
        index: Option<u32>,
    ) -> ElementBuilder {
        Tooltip::new(tip.to_string())
            .child(
                el(El::Button)
                    .st([
                        St::DisplayFlex,
                        St::ItemsCenter,
                        St::JustifyCenter,
                        St::PxSm,
                        St::PySm,
                        St::RoundedSm,
                        St::TextMuted,
                        St::BgTransparent,
                        St::BorderNone,
                        St::CursorPointer,
                    ])
                    .hover([St::BgSubtle, St::TextHigh])
                    .on(Ev::Click, self.act(action, index))
                    .append([icons::icon_sized(icon, 13)]),
            )
            .build()
    }

    // ---------------------------------------------------------------- tree

    fn tree_label(entry: &FsEntry, selected: bool, dirty: bool) -> ElementBuilder {
        let glyph = if entry.is_dir {
            icons::icon_sized(icons::Icon::Folder, 14)
        } else {
            icons::icon_sized(icons::Icon::FileText, 14)
        };
        let mut label = el(El::Span)
            .st([
                St::DisplayFlex,
                St::ItemsCenter,
                St::GapSm,
                St::MinW0,
                St::Flex1,
            ])
            .append([
                el(El::Span).st([St::TextMuted]).append([glyph]),
                el(El::Span)
                    .st(if selected {
                        [St::TextSm, St::TextHigh]
                    } else {
                        [St::TextSm, St::TextDefault]
                    })
                    .text(&entry.name),
            ]);
        if dirty {
            label = label.append([el(El::Span)
                .st([St::BgWarning, St::RoundedFull, St::DisplayInlineBlock])
                .style(Style::new().width("0.35rem").height("0.35rem"))]);
        }
        label
    }

    fn name_form(&self, action: Action, placeholder: &str, initial: &str) -> ElementBuilder {
        el(El::Form)
            .on(Ev::Submit, self.act(action, None))
            .st([
                St::DisplayFlex,
                St::GapXs,
                St::ItemsCenter,
                St::PxSm,
                St::PySm,
            ])
            .append([
                el(El::Input)
                    .at_str(At::Name, "name")
                    .at(At::Autocomplete, Av::Off)
                    .at_str(At::Placeholder, placeholder)
                    .at_str(At::Value, initial)
                    .st([
                        St::Flex1,
                        St::MinW0,
                        St::TextXs,
                        St::PxSm,
                        St::BorderDefault,
                        St::RoundedSm,
                        St::BgApp,
                    ])
                    .focus([St::BorderAccent]),
                self.icon_action(icons::Icon::Close, "Cancel", Action::CancelOp, None),
            ])
    }

    fn build_tree(&self) -> ElementBuilder {
        let entries = self.state.entries_or(&self.snap.entries);
        let mut idx = 0usize;
        let roots = self.tree_level(entries, &mut idx, 0);

        let mut head = el(El::Div)
            .st([
                St::DisplayFlex,
                St::ItemsCenter,
                St::GapSm,
                St::PxSm,
                St::PySm,
            ])
            .append([el(El::Span)
                .st([St::TextXs, St::TextLow, St::FontMono, St::Flex1])
                .text(&format!(
                    "{} · {} files",
                    self.snap
                        .root()
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "root".into()),
                    entries.iter().filter(|e| !e.is_dir).count()
                ))]);
        if self.managed {
            head = head.append([self.icon_action(
                icons::Icon::Plus,
                "New file",
                Action::CreateStart,
                None,
            )]);
        }

        let mut column = vec![head];
        if self.state.pending == Pending::Create {
            column.push(self.name_form(Action::CreateSubmit, "new-file.md", ""));
        }
        column.push(TreeView::new().roots(roots).build());
        el(El::Div)
            .st([St::DisplayFlex, St::FlexCol, St::MinW0, St::PrSm, St::Flex1])
            .append(column)
    }

    fn tree_level(&self, entries: &[FsEntry], idx: &mut usize, depth: usize) -> Vec<TreeNode> {
        let mut nodes = Vec::new();
        while *idx < entries.len() {
            let entry = &entries[*idx];
            if entry.depth < depth {
                break;
            }
            let i = *idx;
            *idx += 1;
            if entry.is_dir {
                let children = self.tree_level(entries, idx, depth + 1);
                nodes.push(
                    TreeNode::branch(
                        entry.rel.clone(),
                        Self::tree_label(entry, false, false),
                        children,
                    )
                    .expanded(true),
                );
            } else if self.state.pending == (Pending::Rename { index: i }) {
                nodes.push(TreeNode::leaf(
                    entry.rel.clone(),
                    self.name_form(Action::RenameSubmit, &entry.name, &entry.name),
                ));
            } else {
                let selected = self.state.selected == Some(i);
                let dirty = selected && self.state.dirty();
                let mut label = Self::tree_label(entry, selected, dirty);
                if selected && self.managed {
                    label = label.append([
                        self.icon_action(
                            icons::Icon::Edit,
                            "Rename",
                            Action::RenameStart,
                            Some(i as u32),
                        ),
                        self.icon_action(
                            icons::Icon::Trash,
                            "Delete",
                            Action::Delete,
                            Some(i as u32),
                        ),
                    ]);
                }
                nodes.push(
                    TreeNode::leaf(entry.rel.clone(), label)
                        .selected(selected)
                        .on_select(self.act(Action::Select, Some(i as u32))),
                );
            }
        }
        nodes
    }

    // ------------------------------------------------------------- banners

    fn banner(&self, tone: St, text: &str, actions: Vec<ElementBuilder>) -> ElementBuilder {
        el(El::Div)
            .st([
                St::DisplayFlex,
                St::ItemsCenter,
                St::GapSm,
                St::PxSm,
                St::PySm,
                St::RoundedSm,
                St::TextXs,
                tone,
            ])
            .append(std::iter::once(el(El::Span).st([St::Flex1]).text(text)).chain(actions))
    }

    fn pending_banner(&self) -> Option<ElementBuilder> {
        let entries = self.state.entries_or(&self.snap.entries);
        match self.state.pending {
            Pending::Conflict => Some(self.banner(
                St::BgError,
                "File changed on disk since you opened it.",
                vec![
                    self.chip_action("Reload theirs", Action::ConflictReload, None),
                    self.chip_action("Overwrite", Action::ConflictOverwrite, None),
                ],
            )),
            Pending::Guard { .. } => {
                let n = self.state.dirty_lines().iter().filter(|d| **d).count();
                Some(self.banner(
                    St::BgWarning,
                    &format!("Discard {n} unsaved line{}?", if n == 1 { "" } else { "s" }),
                    vec![
                        self.chip_action("Keep editing", Action::GuardKeep, None),
                        self.chip_action("Discard", Action::GuardDiscard, None),
                    ],
                ))
            }
            Pending::Delete { index } => {
                let name = entries
                    .get(index)
                    .map(|e| e.name.as_str())
                    .unwrap_or("file");
                Some(self.banner(
                    St::BgError,
                    &format!("Delete {name}?"),
                    vec![
                        self.chip_action("Cancel", Action::CancelOp, None),
                        self.chip_action("Delete", Action::DeleteConfirm, None),
                    ],
                ))
            }
            _ => None,
        }
    }

    // ---------------------------------------------------------------- main

    fn toolbar(&self, entry: Option<&FsEntry>) -> ElementBuilder {
        let mut bar = el(El::Div).st([
            St::DisplayFlex,
            St::ItemsCenter,
            St::GapSm,
            St::PbSm,
            St::BorderB,
            St::FlexShrink0,
        ]);
        let crumb = entry.map(|e| e.rel.replace('/', " / ")).unwrap_or_default();
        bar = bar.append([el(El::Span)
            .st([St::FontMono, St::TextXs, St::TextLow, St::MinW0])
            .text(&crumb)]);
        let dirty_count = self.state.dirty_lines().iter().filter(|d| **d).count();
        if self.state.dirty() {
            bar = bar.append([el(El::Span)
                .st([St::FontMono, St::TextXs, St::TextWarning, St::PxSm])
                .text(&format!(
                    "{dirty_count} unsaved line{}",
                    if dirty_count == 1 { "" } else { "s" }
                ))]);
        }
        bar = bar.append([el(El::Span).st([St::Flex1])]);
        if entry.is_some() && self.state.kind == FileKind::Text {
            bar = bar.append([el(El::Span).st([St::DisplayFlex, St::GapXs]).append([
                Chip::new("View")
                    .active(!self.state.editing)
                    .on_click(self.act(Action::ToggleMode, None))
                    .build(),
                Chip::new("Edit")
                    .active(self.state.editing)
                    .on_click(self.act(Action::ToggleMode, None))
                    .build(),
            ])]);
        }
        // save trigger: visible button in manual mode, hidden span under autosave
        if self.state.autosave {
            bar = bar.append([el(El::Span)
                .st([St::DisplayNone])
                .attr("data-save-key", "1")
                .on(Ev::Click, self.act(Action::Save, None))]);
        } else {
            bar = bar.append([el(El::Span)
                .attr("data-save-key", "1")
                .append([Button::primary(if self.state.dirty() {
                    "Save ⌘S"
                } else {
                    "Saved"
                })
                .size(ButtonSize::Sm)
                .disabled(!self.state.dirty())
                .on_click(self.act(Action::Save, None))])]);
        }
        bar
    }

    /// Read-only code view matching the editor's anatomy: the same gutter and
    /// mono line grid, with syntax coloration instead of a textarea.
    fn code_view(&self, lang: Option<&str>) -> ElementBuilder {
        let lines = highlight_lines(&self.state.working, lang);
        let count = lines.len().max(1);
        let line_h = Style::new().set("line-height", "1.5");
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
            .append((1..=count).map(|n| el(El::Div).style(line_h.clone()).text(&n.to_string())));
        let code = el(El::Div)
            .st([
                St::Flex1,
                St::MinW0,
                St::FontMono,
                St::TextXs,
                St::OverflowXAuto,
            ])
            .append(lines.into_iter().map(|l| l.style(line_h.clone())));
        el(El::Div)
            .st([St::DisplayFlex, St::GapSm, St::MinW0])
            .append([gutter, code])
    }

    fn body(&self, entry: Option<&FsEntry>) -> ElementBuilder {
        let Some(entry) = entry else {
            return el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::ItemsCenter,
                    St::JustifyCenter,
                    St::Flex1,
                    St::TextMuted,
                ])
                .text("Select a file to view it.");
        };
        let lang = entry.rel.rsplit_once('.').map(|(_, e)| e);
        match self.state.kind {
            FileKind::Text if self.state.editing => {
                let flags = self.state.dirty_lines();
                CodeEditor::new("fe-field", &self.state.working)
                    .dirty_lines(&flags)
                    .on_edit(self.act(Action::Edit, None))
                    .build()
            }
            FileKind::Text if lang == Some("md") => {
                Markdown::new(self.state.working.clone()).build()
            }
            FileKind::Text => self.code_view(lang),
            FileKind::Image => {
                let body = match &self.state.preview_b64 {
                    Some(b64) => el(El::Img)
                        .attr("src", &format!("data:image;base64,{b64}"))
                        .attr("alt", &entry.name)
                        .st([St::RoundedMd])
                        .style(
                            Style::new()
                                .set("max-width", "100%")
                                .set("max-height", "70vh"),
                        ),
                    None => el(El::Div)
                        .st([St::TextMuted, St::TextSm])
                        .text("Image too large to preview."),
                };
                el(El::Div)
                    .st([St::DisplayFlex, St::JustifyCenter, St::PMd])
                    .append([body])
            }
            FileKind::Binary => el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::ItemsCenter,
                    St::JustifyCenter,
                    St::Flex1,
                    St::TextMuted,
                    St::TextSm,
                ])
                .text(&format!("{} — binary file (no preview)", entry.name)),
        }
    }

    fn statusbar(&self, entry: Option<&FsEntry>) -> ElementBuilder {
        let lang = entry
            .and_then(|e| e.rel.rsplit_once('.'))
            .map(|(_, e)| e.to_string())
            .unwrap_or_default();
        let lines = self.state.working.lines().count();
        let dirty_count = self.state.dirty_lines().iter().filter(|d| **d).count();
        let mut bar = el(El::Div).st([
            St::DisplayFlex,
            St::ItemsCenter,
            St::GapMd,
            St::PtSm,
            St::BorderT,
            St::FontMono,
            St::TextXs,
            St::TextLow,
            St::FlexShrink0,
        ]);
        if entry.is_some() && self.state.kind == FileKind::Text {
            bar = bar.append([
                el(El::Span).text(&lang),
                el(El::Span).text(&format!("{lines} lines")),
            ]);
            if dirty_count > 0 {
                bar = bar.append([el(El::Span)
                    .st([St::TextWarning])
                    .text(&format!("{dirty_count} changed"))]);
            }
        }
        bar = bar.append([el(El::Span).st([St::Flex1])]);
        bar = bar.append([el(El::Span)
            .st([
                St::DisplayFlex,
                St::ItemsCenter,
                St::GapXs,
                St::CursorPointer,
            ])
            .on(Ev::Click, self.act(Action::ToggleAutosave, None))
            .append([
                el(El::Span).text("autosave"),
                Switch::new().checked(self.state.autosave).build(),
            ])]);
        if let Some(at) = &self.state.saved_at {
            bar = bar.append([el(El::Span).text(&format!(
                "saved{} {at}",
                if self.state.autosave { " · auto" } else { "" }
            ))]);
        }
        bar
    }

    /// Build the full surface.
    pub fn build(self) -> ElementBuilder {
        assert!(self.on_event.is_some(), "FileEditor::on_event is required");
        let entries = self.state.entries_or(&self.snap.entries);
        let entry = self.state.selected.and_then(|i| entries.get(i)).cloned();

        let mut main_col = vec![self.toolbar(entry.as_ref())];
        if let Some(err) = &self.state.error {
            main_col.push(self.banner(St::BgError, err, vec![]));
        }
        if let Some(banner) = self.pending_banner() {
            main_col.push(banner);
        }
        let scrolls = !(self.state.editing && self.state.kind == FileKind::Text);
        let mut body_tokens = vec![
            St::Flex1,
            St::MinH0,
            St::MinW0,
            St::DisplayFlex,
            St::FlexCol,
            St::PtSm,
        ];
        if scrolls {
            body_tokens.push(St::OverflowAuto);
        }
        main_col.push(
            el(El::Div)
                .st(body_tokens)
                .append([self.body(entry.as_ref())]),
        );
        main_col.push(self.statusbar(entry.as_ref()));

        let main = el(El::Div)
            .st([
                St::DisplayFlex,
                St::FlexCol,
                St::Flex1,
                St::MinH0,
                St::MinW0,
                St::PlMd,
            ])
            .append(main_col);

        SplitPane::new(self.build_tree(), main)
            .initial("16rem")
            .build()
            .st([St::Flex1, St::MinH0])
    }
}
