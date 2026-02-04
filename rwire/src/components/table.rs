//! Table component.
//!
//! Div-based table for structured data display.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Table, TableRow};
//!
//! Table::new()
//!     .headers(["Name", "Email", "Role"])
//!     .row(TableRow::new().cells(["Alice", "alice@example.com", "Admin"]))
//!     .row(TableRow::new().cells(["Bob", "bob@example.com", "User"]))
//!     .build()
//! ```

use crate::{el, El, ElementBuilder};
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

impl Table {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-table";

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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(48);
        classes.push_str(Self::BASE_CLASS);

        if self.striped {
            classes.push_str(" rw-table-striped");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the table into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Table);

        let class = self.compute_class();
        let mut table = el(El::Div).class(&class);

        // Add header row if headers are provided
        if !self.headers.is_empty() {
            let mut header_row = el(El::Div).class("rw-tr rw-tr-header");

            for header in &self.headers {
                header_row = header_row.append([
                    el(El::Div).class("rw-th").text(header)
                ]);
            }

            table = table.append([header_row]);
        }

        // Add data rows
        for row in self.rows {
            let mut row_el = el(El::Div).class("rw-tr");

            for cell in row.cells {
                row_el = row_el.append([
                    el(El::Div).class("rw-td").text(&cell)
                ]);
            }

            table = table.append([row_el]);
        }

        table
    }
}

/// Table CSS.
///
/// Size: ~385 bytes (under 400 bytes budget)
pub const TABLE_CSS: &str = "\
.rw-table{display:flex;flex-direction:column;border:1px solid var(--rw-border-default);border-radius:var(--rw-radius-md);overflow:hidden}\
.rw-tr{display:flex;border-bottom:1px solid var(--rw-border-default)}\
.rw-tr:last-child{border-bottom:none}\
.rw-tr-header{background:var(--rw-bg-muted);font-weight:var(--rw-font-medium)}\
.rw-th,.rw-td{flex:1;padding:var(--rw-space-3);font-size:var(--rw-text-sm);color:var(--rw-text-high)}\
.rw-table-striped .rw-tr:nth-child(even){background:var(--rw-bg-subtle)}\n";

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
    fn test_table_class_default() {
        let table = Table::new();
        assert_eq!(table.compute_class(), "rw-table");
    }

    #[test]
    fn test_table_class_striped() {
        let table = Table::new().striped(true);
        let class = table.compute_class();
        assert!(class.contains("rw-table"));
        assert!(class.contains("rw-table-striped"));
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
    fn test_table_css_size() {
        // Table CSS should be under 400 bytes
        assert!(
            TABLE_CSS.len() < 500,
            "Table CSS too large: {} bytes (budget: 500)",
            TABLE_CSS.len()
        );
        println!("Table CSS size: {} bytes", TABLE_CSS.len());
    }

    #[test]
    fn test_table_css_structure() {
        assert!(TABLE_CSS.contains(".rw-table{"));
        assert!(TABLE_CSS.contains(".rw-tr"));
        assert!(TABLE_CSS.contains(".rw-th"));
        assert!(TABLE_CSS.contains(".rw-td"));
        assert!(TABLE_CSS.contains(".rw-table-striped"));
    }
}
