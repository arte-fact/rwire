//! Automatic style grouping for optimal wire compression.
//!
//! This module analyzes style token usage patterns and automatically
//! generates composite styles for frequently co-occurring combinations.
//!
//! # Algorithm
//!
//! 1. **Pattern Collection**: Walk element tree, collect all style token sets
//! 2. **Frequent Itemset Mining**: Find patterns that co-occur frequently
//! 3. **Composite Generation**: Create optimized composites for best compression
//! 4. **Emission**: Replace atomic tokens with composite references
//!
//! # Wire Format
//!
//! Composites use varint IDs starting at 0x100 (256) to avoid conflict with
//! predefined utilities (0x00-0xBF) and reserved range (0xC0-0xFF).
//!
//! ```text
//! COMPOSITE_TABLE (0x86): [opcode, count_varint,
//!     id_varint, util_count, util1, util2, ...,
//!     id_varint, util_count, util1, util2, ...,
//! ]
//! STYLE_COMPOSITE (0x85): [opcode, ref, composite_id_varint]
//! ```
//!
//! # Example
//!
//! ```ignore
//! use rwire::{el, El, St};
//!
//! // Before: 50 elements × 4 utils × 3 bytes = 600 bytes
//! el(El::Div).st([St::DisplayFlex, St::FlexCol, St::GapMd, St::PMd])
//!
//! // After: table(~12 bytes) + 50 × 3 bytes = 162 bytes (73% savings)
//! ```

use std::collections::HashMap;

/// Starting ID for composite styles (varint encoded).
/// Below 256 is reserved for predefined utilities.
pub const COMPOSITE_ID_START: u32 = 0x100;

/// Minimum frequency for a pattern to be worth compositing.
/// With composite overhead, patterns need to appear at least twice.
pub const MIN_PATTERN_FREQUENCY: usize = 2;

/// Minimum pattern size to consider for compositing.
/// Single-utility "patterns" don't benefit from compositing.
pub const MIN_PATTERN_SIZE: usize = 2;

/// A pattern of style utilities that appear together.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StylePattern {
    /// Sorted utility codes for consistent hashing
    utils: Vec<u8>,
}

impl StylePattern {
    /// Create a new pattern from utility codes.
    /// Codes are automatically sorted for consistent comparison.
    pub fn new(utils: impl IntoIterator<Item = u8>) -> Self {
        let mut utils: Vec<u8> = utils.into_iter().collect();
        utils.sort_unstable();
        utils.dedup();
        Self { utils }
    }

    /// Get the utility codes in this pattern.
    pub fn utils(&self) -> &[u8] {
        &self.utils
    }

    /// Get the number of utilities in this pattern.
    pub fn len(&self) -> usize {
        self.utils.len()
    }

    /// Check if pattern is empty.
    pub fn is_empty(&self) -> bool {
        self.utils.is_empty()
    }

    /// Check if this pattern contains another pattern (subset relationship).
    pub fn contains(&self, other: &StylePattern) -> bool {
        other.utils.iter().all(|u| self.utils.contains(u))
    }

    /// Calculate byte cost for atomic emission (3 bytes per util).
    pub fn atomic_cost(&self) -> usize {
        self.utils.len() * 3
    }

    /// Calculate byte cost for composite emission (3 bytes reference).
    pub fn composite_cost(&self) -> usize {
        3 // STYLE_UTIL opcode + ref + composite_id (varint, usually 2 bytes)
    }

    /// Calculate the definition cost for this composite in the table.
    pub fn definition_cost(&self) -> usize {
        // id_varint (2 bytes typical) + count (1 byte) + utils
        2 + 1 + self.utils.len()
    }
}

/// Tracks pattern frequencies across an element tree.
#[derive(Clone, Debug, Default)]
pub struct PatternCollector {
    /// Count of each exact pattern
    exact_patterns: HashMap<StylePattern, usize>,
    /// Count of each individual utility (for itemset mining)
    utility_counts: HashMap<u8, usize>,
    /// Count of each utility pair (for itemset mining)
    pair_counts: HashMap<(u8, u8), usize>,
}

impl PatternCollector {
    /// Create a new pattern collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a pattern observation.
    pub fn observe(&mut self, utils: &[u8]) {
        if utils.len() < MIN_PATTERN_SIZE {
            return;
        }

        let pattern = StylePattern::new(utils.iter().copied());
        *self.exact_patterns.entry(pattern).or_insert(0) += 1;

        // Track individual utilities
        for &u in utils {
            *self.utility_counts.entry(u).or_insert(0) += 1;
        }

        // Track pairs for itemset mining
        let sorted: Vec<u8> = {
            let mut v: Vec<u8> = utils.to_vec();
            v.sort_unstable();
            v
        };
        for i in 0..sorted.len() {
            for j in (i + 1)..sorted.len() {
                let pair = (sorted[i], sorted[j]);
                *self.pair_counts.entry(pair).or_insert(0) += 1;
            }
        }
    }

    /// Get exact pattern frequencies.
    pub fn exact_patterns(&self) -> &HashMap<StylePattern, usize> {
        &self.exact_patterns
    }

    /// Get utility frequencies.
    pub fn utility_counts(&self) -> &HashMap<u8, usize> {
        &self.utility_counts
    }

    /// Get pair frequencies.
    pub fn pair_counts(&self) -> &HashMap<(u8, u8), usize> {
        &self.pair_counts
    }

    /// Get total number of observations.
    pub fn total_observations(&self) -> usize {
        self.exact_patterns.values().sum()
    }
}

/// A composite style definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Composite {
    /// Unique ID for this composite (varint encoded, >= COMPOSITE_ID_START)
    pub id: u32,
    /// The utility codes this composite expands to
    pub pattern: StylePattern,
    /// How many times this pattern appears
    pub frequency: usize,
}

impl Composite {
    /// Calculate bytes saved by using this composite vs atomic emission.
    pub fn bytes_saved(&self) -> isize {
        let atomic_total = self.pattern.atomic_cost() * self.frequency;
        let composite_total = self.pattern.definition_cost()
            + self.pattern.composite_cost() * self.frequency;
        atomic_total as isize - composite_total as isize
    }
}

/// Generated composite table for optimal style emission.
#[derive(Clone, Debug, Default)]
pub struct CompositeTable {
    /// Composites sorted by ID
    composites: Vec<Composite>,
    /// Quick lookup: pattern -> composite ID
    pattern_to_id: HashMap<StylePattern, u32>,
    /// Quick lookup: check if a set of utils has a composite
    utils_to_id: HashMap<Vec<u8>, u32>,
}

impl CompositeTable {
    /// Create an empty composite table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a composite table from collected patterns.
    ///
    /// Uses frequent itemset mining to find optimal composites.
    pub fn from_collector(collector: &PatternCollector) -> Self {
        let mut table = Self::new();
        let mut next_id = COMPOSITE_ID_START;

        // Start with exact patterns that meet frequency threshold
        let mut candidates: Vec<_> = collector
            .exact_patterns()
            .iter()
            .filter(|(p, &count)| count >= MIN_PATTERN_FREQUENCY && p.len() >= MIN_PATTERN_SIZE)
            .map(|(p, &count)| (p.clone(), count))
            .collect();

        // Sort by bytes_saved descending (greedy selection)
        candidates.sort_by(|(p1, c1), (p2, c2)| {
            let savings1 = (p1.atomic_cost() * c1) as isize
                - (p1.definition_cost() + p1.composite_cost() * c1) as isize;
            let savings2 = (p2.atomic_cost() * c2) as isize
                - (p2.definition_cost() + p2.composite_cost() * c2) as isize;
            savings2.cmp(&savings1)
        });

        // Add composites that provide positive savings
        for (pattern, frequency) in candidates {
            let composite = Composite {
                id: next_id,
                pattern: pattern.clone(),
                frequency,
            };

            if composite.bytes_saved() > 0 {
                table.pattern_to_id.insert(pattern.clone(), next_id);

                let mut sorted_utils = pattern.utils().to_vec();
                sorted_utils.sort_unstable();
                table.utils_to_id.insert(sorted_utils, next_id);

                table.composites.push(composite);
                next_id += 1;
            }
        }

        table
    }

    /// Look up composite ID for a pattern.
    pub fn get_composite_id(&self, utils: &[u8]) -> Option<u32> {
        let mut sorted = utils.to_vec();
        sorted.sort_unstable();
        self.utils_to_id.get(&sorted).copied()
    }

    /// Get all composites.
    pub fn composites(&self) -> &[Composite] {
        &self.composites
    }

    /// Check if table is empty.
    pub fn is_empty(&self) -> bool {
        self.composites.is_empty()
    }

    /// Get total bytes saved by using this composite table.
    pub fn total_bytes_saved(&self) -> isize {
        self.composites.iter().map(|c| c.bytes_saved()).sum()
    }

    /// Generate the composite table bytes for the wire protocol.
    pub fn to_bytes(&self) -> Vec<u8> {
        use crate::protocol::varint::write_varint;

        let mut bytes = Vec::new();

        // Count (varint)
        write_varint(&mut bytes, self.composites.len() as u32);

        for composite in &self.composites {
            // ID (varint)
            write_varint(&mut bytes, composite.id);
            // Util count (u8)
            bytes.push(composite.pattern.len() as u8);
            // Utils
            bytes.extend_from_slice(composite.pattern.utils());
        }

        bytes
    }
}

/// Result of pattern analysis with optimization recommendations.
#[derive(Clone, Debug)]
pub struct PatternAnalysis {
    /// Total patterns observed
    pub total_patterns: usize,
    /// Patterns that could be optimized
    pub optimizable_patterns: usize,
    /// Generated composite table
    pub composite_table: CompositeTable,
    /// Estimated bytes saved
    pub estimated_savings: isize,
    /// Original byte count (atomic emission)
    pub original_bytes: usize,
    /// Optimized byte count (with composites)
    pub optimized_bytes: usize,
}

impl PatternAnalysis {
    /// Calculate compression ratio (0.0 = no compression, 1.0 = 100% compression).
    pub fn compression_ratio(&self) -> f64 {
        if self.original_bytes == 0 {
            return 0.0;
        }
        1.0 - (self.optimized_bytes as f64 / self.original_bytes as f64)
    }
}

/// Collect style patterns from an element tree.
///
/// Walks the tree recursively and observes all style token combinations.
pub fn collect_from_element(collector: &mut PatternCollector, el: &crate::builder::ElementBuilder) {
    // Collect pattern from this element's style tokens
    let utils = el.get_style_utils();
    if !utils.is_empty() {
        collector.observe(utils);
    }

    // Also collect from style_props (converted to utils for pattern matching)
    // Note: We only composite style_utils, not style_props (they're more dynamic)

    // Recursively collect from children
    for child in el.children() {
        collect_from_element(collector, child);
    }

    // If this is a synced element, we'd need access to rendered content
    // That's handled at a higher level during BuildContext collection
}

/// Collect patterns from an element tree using default state for synced elements.
///
/// This is the main entry point for pattern collection.
pub fn collect_patterns(root: &crate::builder::ElementBuilder) -> PatternCollector {
    let mut collector = PatternCollector::new();
    collect_from_element(&mut collector, root);
    collector
}

/// Analyze patterns and generate optimized composite table.
pub fn analyze_patterns(collector: &PatternCollector) -> PatternAnalysis {
    let composite_table = CompositeTable::from_collector(collector);

    let total_patterns = collector.total_observations();
    let optimizable_patterns = collector
        .exact_patterns()
        .iter()
        .filter(|(p, &c)| c >= MIN_PATTERN_FREQUENCY && p.len() >= MIN_PATTERN_SIZE)
        .count();

    // Calculate original bytes (all atomic)
    let original_bytes: usize = collector
        .exact_patterns()
        .iter()
        .map(|(p, &count)| p.atomic_cost() * count)
        .sum();

    // Calculate optimized bytes
    let mut optimized_bytes = 0usize;

    // Add composite table overhead
    optimized_bytes += composite_table.to_bytes().len();

    // Add emission costs
    for (pattern, &count) in collector.exact_patterns() {
        if composite_table.get_composite_id(pattern.utils()).is_some() {
            // Use composite reference
            optimized_bytes += pattern.composite_cost() * count;
        } else {
            // Use atomic emission
            optimized_bytes += pattern.atomic_cost() * count;
        }
    }

    let estimated_savings = original_bytes as isize - optimized_bytes as isize;

    PatternAnalysis {
        total_patterns,
        optimizable_patterns,
        composite_table,
        estimated_savings,
        original_bytes,
        optimized_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // StylePattern Tests
    // ========================================================================

    #[test]
    fn test_pattern_new_sorts_and_dedupes() {
        let pattern = StylePattern::new([0x05, 0x02, 0x05, 0x01]);
        assert_eq!(pattern.utils(), &[0x01, 0x02, 0x05]);
    }

    #[test]
    fn test_pattern_equality() {
        let p1 = StylePattern::new([0x01, 0x02, 0x03]);
        let p2 = StylePattern::new([0x03, 0x01, 0x02]); // Different order
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_pattern_contains() {
        let large = StylePattern::new([0x01, 0x02, 0x03, 0x04]);
        let small = StylePattern::new([0x02, 0x03]);
        let other = StylePattern::new([0x05, 0x06]);

        assert!(large.contains(&small));
        assert!(large.contains(&large)); // Contains itself
        assert!(!small.contains(&large));
        assert!(!large.contains(&other));
    }

    #[test]
    fn test_pattern_costs() {
        let pattern = StylePattern::new([0x01, 0x02, 0x03, 0x04]);

        // 4 utils × 3 bytes = 12 bytes
        assert_eq!(pattern.atomic_cost(), 12);

        // Composite reference = 3 bytes
        assert_eq!(pattern.composite_cost(), 3);

        // Definition = 2 (id) + 1 (count) + 4 (utils) = 7 bytes
        assert_eq!(pattern.definition_cost(), 7);
    }

    // ========================================================================
    // PatternCollector Tests
    // ========================================================================

    #[test]
    fn test_collector_observe_exact_patterns() {
        let mut collector = PatternCollector::new();

        collector.observe(&[0x01, 0x02, 0x03]);
        collector.observe(&[0x01, 0x02, 0x03]); // Same pattern
        collector.observe(&[0x04, 0x05]);

        let patterns = collector.exact_patterns();

        let p1 = StylePattern::new([0x01, 0x02, 0x03]);
        let p2 = StylePattern::new([0x04, 0x05]);

        assert_eq!(patterns.get(&p1), Some(&2));
        assert_eq!(patterns.get(&p2), Some(&1));
    }

    #[test]
    fn test_collector_ignores_small_patterns() {
        let mut collector = PatternCollector::new();

        collector.observe(&[0x01]); // Too small
        collector.observe(&[0x01, 0x02]); // OK

        assert_eq!(collector.exact_patterns().len(), 1);
    }

    #[test]
    fn test_collector_tracks_pairs() {
        let mut collector = PatternCollector::new();

        collector.observe(&[0x01, 0x02, 0x03]);
        collector.observe(&[0x01, 0x02]);

        let pairs = collector.pair_counts();

        // (0x01, 0x02) appears in both observations
        assert_eq!(pairs.get(&(0x01, 0x02)), Some(&2));
        // (0x02, 0x03) appears only in first
        assert_eq!(pairs.get(&(0x02, 0x03)), Some(&1));
    }

    // ========================================================================
    // CompositeTable Tests
    // ========================================================================

    #[test]
    fn test_composite_bytes_saved() {
        let pattern = StylePattern::new([0x01, 0x02, 0x03, 0x04]);
        let composite = Composite {
            id: COMPOSITE_ID_START,
            pattern,
            frequency: 10,
        };

        // Atomic: 4 utils × 3 bytes × 10 = 120 bytes
        // Composite: definition(7) + reference(3) × 10 = 37 bytes
        // Savings: 120 - 37 = 83 bytes
        assert_eq!(composite.bytes_saved(), 83);
    }

    #[test]
    fn test_composite_bytes_saved_negative() {
        let pattern = StylePattern::new([0x01, 0x02]);
        let composite = Composite {
            id: COMPOSITE_ID_START,
            pattern,
            frequency: 1, // Only used once
        };

        // Atomic: 2 utils × 3 bytes × 1 = 6 bytes
        // Composite: definition(5) + reference(3) × 1 = 8 bytes
        // Savings: 6 - 8 = -2 bytes (worse!)
        assert!(composite.bytes_saved() < 0);
    }

    #[test]
    fn test_composite_table_from_collector() {
        let mut collector = PatternCollector::new();

        // Pattern used 5 times - should be composited
        for _ in 0..5 {
            collector.observe(&[0x01, 0x02, 0x03, 0x04]);
        }

        // Pattern used once - should NOT be composited
        collector.observe(&[0x10, 0x11]);

        let table = CompositeTable::from_collector(&collector);

        // Should have exactly one composite (the frequent pattern)
        assert_eq!(table.composites().len(), 1);
        assert_eq!(table.composites()[0].pattern.utils(), &[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(table.composites()[0].frequency, 5);
    }

    #[test]
    fn test_composite_table_lookup() {
        let mut collector = PatternCollector::new();

        for _ in 0..5 {
            collector.observe(&[0x01, 0x02, 0x03]);
        }

        let table = CompositeTable::from_collector(&collector);

        // Should find composite for this pattern (order independent)
        assert!(table.get_composite_id(&[0x03, 0x01, 0x02]).is_some());

        // Should not find composite for unknown pattern
        assert!(table.get_composite_id(&[0x10, 0x11]).is_none());
    }

    #[test]
    fn test_composite_table_to_bytes() {
        let mut collector = PatternCollector::new();

        for _ in 0..5 {
            collector.observe(&[0x01, 0x02]);
        }

        let table = CompositeTable::from_collector(&collector);
        let bytes = table.to_bytes();

        // Should produce valid bytes:
        // count_varint(1) + id_varint(256) + count(2) + utils(0x01, 0x02)
        assert!(!bytes.is_empty());

        // First byte should be count = 1
        assert_eq!(bytes[0], 1);
    }

    // ========================================================================
    // Pattern Analysis Tests
    // ========================================================================

    #[test]
    fn test_analyze_patterns_calculates_savings() {
        let mut collector = PatternCollector::new();

        // 10 elements use 4 utils each
        for _ in 0..10 {
            collector.observe(&[0x01, 0x02, 0x03, 0x04]);
        }

        let analysis = analyze_patterns(&collector);

        // Original: 10 × 4 × 3 = 120 bytes
        assert_eq!(analysis.original_bytes, 120);

        // Should have positive savings
        assert!(analysis.estimated_savings > 0);

        // Optimized should be less than original
        assert!(analysis.optimized_bytes < analysis.original_bytes);
    }

    #[test]
    fn test_analyze_patterns_compression_ratio() {
        let mut collector = PatternCollector::new();

        // Many uses of large pattern = high compression
        for _ in 0..50 {
            collector.observe(&[0x01, 0x02, 0x03, 0x04, 0x05]);
        }

        let analysis = analyze_patterns(&collector);

        // Should achieve significant compression
        assert!(analysis.compression_ratio() > 0.5);
    }

    #[test]
    fn test_analyze_patterns_no_savings_for_infrequent() {
        let mut collector = PatternCollector::new();

        // Each pattern only used once
        collector.observe(&[0x01, 0x02]);
        collector.observe(&[0x03, 0x04]);
        collector.observe(&[0x05, 0x06]);

        let analysis = analyze_patterns(&collector);

        // No composites should be generated
        assert!(analysis.composite_table.is_empty());

        // Optimized adds 1 byte overhead for empty composite table
        // Original: 3 patterns × 2 utils × 3 bytes = 18 bytes
        // Optimized: 18 + 1 (empty table) = 19 bytes
        assert_eq!(analysis.original_bytes, 18);
        assert_eq!(analysis.optimized_bytes, 19);
        assert!(analysis.estimated_savings < 0); // Slight overhead when no composites
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[test]
    fn test_realistic_component_library_scenario() {
        let mut collector = PatternCollector::new();

        // Simulate a component library with common patterns:

        // Button base (used 20 times): flex, center, gap-sm, px-md, py-sm
        for _ in 0..20 {
            collector.observe(&[0x02, 0x1A, 0x2A, 0x58, 0x5A]);
        }

        // Card base (used 15 times): flex, col, gap-md, p-lg
        for _ in 0..15 {
            collector.observe(&[0x02, 0x11, 0x2B, 0x54]);
        }

        // Stack row (used 30 times): flex, row, gap-md
        for _ in 0..30 {
            collector.observe(&[0x02, 0x10, 0x2B]);
        }

        // Stack col (used 25 times): flex, col, gap-md
        for _ in 0..25 {
            collector.observe(&[0x02, 0x11, 0x2B]);
        }

        let analysis = analyze_patterns(&collector);

        // Should create composites for all frequent patterns
        assert!(analysis.composite_table.composites().len() >= 3);

        // Should achieve good compression
        assert!(analysis.compression_ratio() > 0.4);

        println!("Component library analysis:");
        println!("  Original: {} bytes", analysis.original_bytes);
        println!("  Optimized: {} bytes", analysis.optimized_bytes);
        println!("  Savings: {} bytes ({:.1}%)",
            analysis.estimated_savings,
            analysis.compression_ratio() * 100.0
        );
    }

    #[test]
    fn test_collect_from_element_tree() {
        use crate::builder::el;
        use crate::protocol::El;
        use crate::style_tokens::St;

        // Build a tree with repeated style patterns
        let root = el(El::Div)
            .st([St::DisplayFlex, St::FlexCol, St::GapMd])
            .append([
                el(El::Div).st([St::DisplayFlex, St::FlexCol, St::GapMd]),
                el(El::Div).st([St::DisplayFlex, St::FlexCol, St::GapMd]),
                el(El::Div).st([St::DisplayFlex, St::ItemsCenter]),
            ]);

        let collector = collect_patterns(&root);

        // Should have observed patterns
        assert!(collector.total_observations() >= 3);

        // The flex+col+gap pattern appears 3 times
        let pattern = StylePattern::new([0x02, 0x11, 0x2B]); // flex, col, gap-md
        let count = collector.exact_patterns().get(&pattern).copied().unwrap_or(0);
        assert_eq!(count, 3, "Expected flex+col+gap pattern to appear 3 times");

        // Analyze patterns
        let analysis = analyze_patterns(&collector);

        // Should have at least one composite (the 3-use pattern)
        assert!(!analysis.composite_table.is_empty());
        assert!(analysis.estimated_savings > 0);
    }
}
