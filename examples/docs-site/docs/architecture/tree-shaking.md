---
title: Tree Shaking
description: How rwire minimizes the client runtime
order: 3
---
# Tree Shaking

rwire generates a minimal JavaScript runtime for each application. Only the code needed by your specific app is included.

## What Gets Tree-Shaken

- **Element types**: Only `createElement` calls for elements you actually use
- **Event types**: Only `addEventListener` bindings for events you handle
- **Style tokens**: Only CSS rules for tokens referenced in your components
- **Attribute tokens**: Only attribute name/value lookup entries you use

## How It Works

1. At server startup, `BuildContext::collect_symbols()` walks the root element tree
2. It records every element type, event type, and style token encountered
3. `capsule_gen::generate_capsule()` filters the lookup tables to only include used entries
4. The resulting JS runtime and CSS are embedded in the capsule HTML

## Size Budgets

| App Complexity | JS Runtime | CSS | Total |
|---------------|-----------|-----|-------|
| Counter (minimal) | ~1.5KB | ~2KB | ~3.5KB |
| Todo list | ~1.8KB | ~8KB | ~10KB |
| Design system (all components) | ~2.2KB | ~17KB | ~19KB |

Compare with typical SPA frameworks:

| Framework | JS Bundle | CSS |
|-----------|----------|-----|
| React + Router | 45KB+ | varies |
| Vue + Vuetify | 80KB+ | 50KB+ |
| rwire | ~2KB | ~17KB |
