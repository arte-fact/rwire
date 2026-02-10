//! Grid layout component.
//!
//! CSS Grid-based layout with configurable columns and spacing.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Grid, GridColumns, Gap};
//!
//! Grid::new()
//!     .columns(GridColumns::Fixed(3))
//!     .gap(Gap::Lg)
//!     .children([child1, child2, child3])
//!     .build()
//!
//! // Responsive auto-fill grid
//! Grid::auto()
//!     .gap(Gap::Md)
//!     .children(cards)
//!     .build()
//! ```

use super::stack::Gap;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};

/// Grid column configuration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridColumns {
    /// Fixed number of equal columns (1-4).
    #[default]
    Auto,
    /// 1 column
    Fixed1,
    /// 2 columns
    Fixed2,
    /// 3 columns
    Fixed3,
    /// 4 columns
    Fixed4,
}

/// Grid layout builder.
#[derive(Clone, Default)]
pub struct Grid {
    columns: GridColumns,
    gap: Gap,
    children: Vec<ElementBuilder>,
}

impl Grid {
    /// Create a new grid with responsive auto-fill columns.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a grid with responsive auto-fill columns.
    pub fn auto() -> Self {
        Self {
            columns: GridColumns::Auto,
            ..Self::default()
        }
    }

    /// Set the column configuration.
    pub fn columns(mut self, columns: GridColumns) -> Self {
        self.columns = columns;
        self
    }

    /// Set the gap between items.
    pub fn gap(mut self, gap: Gap) -> Self {
        self.gap = gap;
        self
    }

    /// Add children to the grid.
    pub fn children(mut self, children: impl IntoIterator<Item = ElementBuilder>) -> Self {
        self.children.extend(children);
        self
    }

    /// Add a single child.
    pub fn child(mut self, child: ElementBuilder) -> Self {
        self.children.push(child);
        self
    }

    /// Compute style tokens for this grid configuration.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![St::DisplayGrid];

        match self.columns {
            GridColumns::Auto => tokens.push(St::GridColsAuto),
            GridColumns::Fixed1 => tokens.push(St::GridCols1),
            GridColumns::Fixed2 => tokens.push(St::GridCols2),
            GridColumns::Fixed3 => tokens.push(St::GridCols3),
            GridColumns::Fixed4 => tokens.push(St::GridCols4),
        }

        match self.gap {
            Gap::None => tokens.push(St::Gap0),
            Gap::Xs => tokens.push(St::GapXs),
            Gap::Sm => tokens.push(St::GapSm),
            Gap::Md => tokens.push(St::GapMd),
            Gap::Lg => tokens.push(St::GapLg),
            Gap::Xl => tokens.push(St::GapXl),
        }

        tokens
    }

    /// Build the grid into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut builder = el(El::Div).st(self.compute_tokens());

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_defaults() {
        let grid = Grid::new();
        assert_eq!(grid.columns, GridColumns::Auto);
        assert_eq!(grid.gap, Gap::Md);
    }

    #[test]
    fn test_grid_auto_tokens() {
        let grid = Grid::auto();
        let tokens = grid.compute_tokens();
        assert!(tokens.contains(&St::DisplayGrid));
        assert!(tokens.contains(&St::GridColsAuto));
        assert!(tokens.contains(&St::GapMd));
    }

    #[test]
    fn test_grid_fixed_tokens() {
        let grid = Grid::new()
            .columns(GridColumns::Fixed3)
            .gap(Gap::Lg);
        let tokens = grid.compute_tokens();
        assert!(tokens.contains(&St::DisplayGrid));
        assert!(tokens.contains(&St::GridCols3));
        assert!(tokens.contains(&St::GapLg));
    }
}
