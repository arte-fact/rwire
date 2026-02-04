# rwire Design System Documentation

Interactive showcase of all rwire components - a living style guide demonstrating every component in the rwire UI library.

## Running the Example

```bash
cargo run -p design-system
```

Then open http://127.0.0.1:9000 in your browser.

## What's Inside

The design system documentation is organized into three sections accessible via tab navigation:

### 1. Form Components

- **Label** - Form labels with optional required indicator
- **Input** - Text inputs with various types (text, email, password, search) and sizes
- **Textarea** - Multi-line text input with customizable rows
- **Checkbox** - Boolean checkbox with optional label association
- **Radio** - Radio buttons for mutually exclusive options
- **Switch** - Toggle switch for boolean states
- **Select** - Dropdown select with options
- **FormField** - Composition component wrapping inputs with labels, help text, and validation errors

### 2. Data Display Components

- **Avatar** - User avatars with fallback text initials
- **Progress** - Progress bars showing task completion (interactive demo)
- **Spinner** - Loading indicators with size variants
- **Table** - Data tables with headers and rows
- **Badge** - Status indicators and labels with intent colors

### 3. Feedback & Navigation Components

- **Alert** - Alert messages with different intent levels (Info, Success, Warning, Error)
- **Breadcrumb** - Navigation breadcrumb trails
- **Tabs** - Tab navigation for content sections
- **Pagination** - Page navigation for lists and tables (interactive demo)

## Interactive Features

Several components demonstrate interactive state management:

- **Checkbox** - Toggle to see state update
- **Radio Buttons** - Select different plans
- **Switch** - Toggle notifications on/off
- **Progress Bar** - Increase/decrease with buttons
- **Pagination** - Navigate between pages

## Component Usage Examples

Each component card shows:
1. Component title and description
2. Live rendered component examples
3. Visual demonstration of variants and states

## Code Structure

```
src/main.rs
├── Header - Title and description
├── Navigation - Section tabs
└── Content - Dynamic section rendering
    ├── Form Components Section
    ├── Data Display Section
    └── Feedback & Navigation Section
```

All components use rwire's reactive state management with `#[handler]` and `#[renderer]` macros.

## Key Patterns Demonstrated

- **Builder Pattern**: All components use fluent builder API
- **Type-Safe Variants**: Enum-based variants prevent typos
- **Reactive State**: Only modified sections re-render
- **Composition**: Complex UIs built from simple components
- **Layout**: Stack and Card components for consistent spacing

## Component CSS

All components use design tokens for consistency:
- Color: `var(--rw-accent-9)`, `var(--rw-red-8)`, etc.
- Spacing: `var(--rw-space-2)`, `var(--rw-space-4)`, etc.
- Typography: `var(--rw-text-sm)`, `var(--rw-font-medium)`, etc.
- Radius: `var(--rw-radius-md)`, etc.

CSS is tree-shaken automatically - only used components' CSS is included.
