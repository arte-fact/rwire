//! Table component.
//!
//! Semantic HTML table for structured data display.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Table, TableRow};
//!
//! Table::new()
//!     .headers(["Name", "Email", "Role"])
//!     .row(TableRow::new().cells(["Alice", "alice@example.com", "Admin"]))
//!     .row(TableRow::new().cells(["Bob", "bob@example.com", "User"]))
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// A single table row.
#[derive(Clone, Debug, Default)]
pub struct TableRow {
    cells: Vec<Cow<'static, str>>,
}

impl TableRow {
    /// Create a new table row.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set cells from an iterator.
    pub fn cells<I, S>(mut self, cells: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Cow<'static, str>>,
    {
        self.cells = cells.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Add a single cell.
    pub fn cell(mut self, cell: impl Into<Cow<'static, str>>) -> Self {
        self.cells.push(cell.into());
        self
    }
}

/// Table builder.
#[derive(Clone, Debug, Default)]
pub struct Table {
    headers: Vec<Cow<'static, str>>,
    rows: Vec<TableRow>,
    striped: bool,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Table {
    /// Create a new table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set table headers from an iterator.
    pub fn headers<I, S>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Cow<'static, str>>,
    {
        self.headers = headers.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Add a row to the table.
    pub fn row(mut self, row: TableRow) -> Self {
        self.rows.push(row);
        self
    }

    /// Enable striped rows.
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the table container.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![
            St::WFull,
            St::BorderSubtle,
            St::RoundedMd,
            St::OverflowHidden,
        ]
    }

    /// Build the table into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut table = el(El::Table).st(self.compute_tokens());

        if let Some(ref extra) = self.extra_class {
            table = table.class(extra.as_ref());
        }

        // Add header row if headers are provided
        if !self.headers.is_empty() {
            let mut header_row = el(El::Tr).st([St::BgMuted, St::FontMedium]);

            for header in &self.headers {
                header_row = header_row.append([el(El::Th)
                    .st([St::TextSm, St::TextHigh, St::PSp3, St::TextLeft])
                    .text(header)]);
            }

            table = table.append([el(El::Thead).append([header_row])]);
        }

        // Add data rows
        if !self.rows.is_empty() {
            let body_children: Vec<ElementBuilder> = self
                .rows
                .into_iter()
                .map(|row| {
                    let mut row_el = el(El::Tr).not_last_child([St::BorderBSubtle]);

                    if self.striped {
                        row_el = row_el.nth_even([St::BgSubtle]);
                    }

                    for cell in row.cells {
                        row_el = row_el.append([el(El::Td)
                            .st([St::TextSm, St::TextHigh, St::PSp3])
                            .text(&cell)]);
                    }

                    row_el
                })
                .collect();

            table = table.append([el(El::Tbody).append(body_children)]);
        }

        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_defaults() {
        let table = Table::new();
        assert!(table.headers.is_empty());
        assert!(table.rows.is_empty());
        assert!(!table.striped);
    }

    #[test]
    fn test_table_tokens() {
        let table = Table::new();
        let tokens = table.compute_tokens();
        assert!(tokens.contains(&St::WFull));
        assert!(tokens.contains(&St::BorderSubtle));
        assert!(tokens.contains(&St::RoundedMd));
        assert!(tokens.contains(&St::OverflowHidden));
    }

    #[test]
    fn test_table_with_data() {
        let table = Table::new()
            .headers(["A", "B"])
            .row(TableRow::new().cells(["1", "2"]))
            .row(TableRow::new().cells(["3", "4"]));
        assert_eq!(table.headers.len(), 2);
        assert_eq!(table.rows.len(), 2);
    }

    #[test]
    fn test_table_build_uses_semantic_elements() {
        let built = Table::new()
            .headers(["H"])
            .row(TableRow::new().cells(["D"]))
            .build();
        assert_eq!(built.el_type(), El::Table);
        // Thead > Tr > Th structure
        let thead = &built.children()[0];
        assert_eq!(thead.el_type(), El::Thead);
        let tr = &thead.children()[0];
        assert_eq!(tr.el_type(), El::Tr);
        let th = &tr.children()[0];
        assert_eq!(th.el_type(), El::Th);
        // Tbody > Tr > Td structure
        let tbody = &built.children()[1];
        assert_eq!(tbody.el_type(), El::Tbody);
        let data_tr = &tbody.children()[0];
        assert_eq!(data_tr.el_type(), El::Tr);
        let td = &data_tr.children()[0];
        assert_eq!(td.el_type(), El::Td);
    }
}
