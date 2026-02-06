# Design System Study: rwire Component Library

A comprehensive analysis of rwire's component library compared to modern design systems, with a roadmap for building an rwire documentation website.

## Table of Contents

1. **[Current Inventory](01-current-inventory.md)** — Complete audit of rwire's 28 components, their APIs, CSS budgets, and patterns
2. **[Comparison](02-comparison.md)** — Side-by-side comparison with shadcn/ui, Chakra UI, Mantine, and Ant Design
3. **[API Analysis](03-api-analysis.md)** — Composition patterns, API ergonomics, what to adopt and what to skip
4. **[Gap Analysis](04-gap-analysis.md)** — Missing components by category with priority tiers
5. **[Docs Site Components](05-docs-site-components.md)** — Components needed to build a full documentation website
6. **[Markdown Pipeline](06-markdown-pipeline.md)** — Architecture for parsing `.md` folders into rwire pages
7. **[Roadmap](07-roadmap.md)** — Phased implementation plan with CSS budgets

## Context

rwire is a server-rendered UI framework where all state and logic live on the server, with a ~1.5KB JavaScript runtime on the client that executes binary DOM opcodes. This architecture has unique implications for component design:

- **No client-side JS per component** — all interactivity is server round-trips or local opcode handlers
- **CSS is the primary cost** — component "weight" is measured in CSS bytes, not JS bundle size
- **Tree-shaking is CSS-level** — unused components contribute zero bytes to the capsule
- **Binary protocol** — element types are single-byte opcodes, not string tag names
- **Symbol interning** — CSS class names are interned once, then referenced by index

Current state: **28 components, ~12.5KB total CSS** (tree-shaken per page).

## Methodology

- Component counts and API surface comparisons based on official documentation as of early 2025
- CSS budget estimates based on rwire's existing component measurements and minified CSS analysis
- Priority tiers based on requirements for building a documentation site with rwire itself
