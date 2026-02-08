# Phase 2: Website Components

**Goal**: Build the three new components identified in the gap analysis — Footer, CopyButton, and LogoCloud.

**Depends on**: Phase 1 (Grid tokens, new icons)

---

## Step 2.1 — Footer Component

A multi-column footer with logo, link groups, social icons, and copyright. Standard pattern across all analyzed framework websites.

### API Design

```rust
Footer::new()
    .logo(
        Link::to("/", "rwire")
            .st([St::TextLg, St::FontBold, St::NoDecoration])
    )
    .tagline("Server-side UI with a binary protocol")
    .column(FooterColumn::new("Resources")
        .link("Documentation", "/docs")
        .link("Examples", "/examples")
        .link("GitHub", "https://github.com/user/rwire"))
    .column(FooterColumn::new("Community")
        .link("Discord", "https://discord.gg/...")
        .link("Discussions", "https://github.com/.../discussions"))
    .column(FooterColumn::new("Legal")
        .link("License", "/docs/license")
        .link("Privacy", "/privacy"))
    .copyright("2026 rwire contributors")
    .build()
```

### Structure

```
<footer>
  ┌─────────────────────────────────────────────┐
  │  Logo + Tagline                              │
  ├──────────┬──────────┬──────────┬────────────┤
  │ Resources│ Community│  Legal   │ (responsive│
  │ · Docs   │ · Discord│ · License│  columns)  │
  │ · Examples│ · Discuss│ · Privacy│            │
  │ · GitHub │          │          │            │
  ├─────────────────────────────────────────────┤
  │  Divider                                     │
  ├─────────────────────────────────────────────┤
  │  (c) 2026 rwire  ·  Built with rwire        │
  └─────────────────────────────────────────────┘
```

### Tokens Used

- Layout: `St::DisplayGrid`, `St::GridCols3`, `St::GapXl`, `St::PtXl`, `St::PbLg`
- Colors: `St::BgSurface`, `St::TextMuted`, `St::BorderT`
- Links: `St::NoDecoration`, `St::TextMuted` + hover `St::TextDefault`

### Implementation Notes

- Footer links use `Link::to()` for internal links, raw `<a>` with `target="_blank"` for external
- Responsive: grid collapses to single column on mobile via `St::GridColsAuto`
- Divider between link columns and copyright

**Files**: `rwire/src/components/footer.rs`, `rwire/src/components/mod.rs`

---

## Step 2.2 — CopyButton Component

A clipboard-copy button with visual feedback. Used in install commands and code blocks.

### API Design

```rust
// Standalone
CopyButton::new("cargo add rwire")
    .size(ButtonSize::Sm)
    .build()

// Integrated in a command line display
el(El::Div)
    .st([St::DisplayFlex, St::ItemsCenter, St::GapSm,
         St::BgCodeBlock, St::RoundedMd, St::PxMd, St::PySm])
    .append([
        el(El::Span).st([St::FontMono, St::TextSm]).text("$ cargo add rwire"),
        CopyButton::new("cargo add rwire").build(),
    ])
```

### Behavior

1. Default state: clipboard icon
2. Click: copy text to clipboard via `navigator.clipboard.writeText()`
3. Success: checkmark icon + "Copied!" tooltip (2 seconds)
4. Reset: return to clipboard icon

### Implementation Strategy

CopyButton needs client-side logic (clipboard API + timeout feedback). Two approaches:

**Option A — Local handler (preferred)**:
Use `#[storage(local)]` state for the "copied" boolean. The `#[handler]` copies text and toggles state. A `setTimeout` resets it.

**Option B — Inline JS via data attributes**:
Encode the copy text in a `data-copy` attribute. Add a small JS snippet to the capsule that handles `click` on `[data-copy]` elements. This keeps it self-contained without server round-trips.

Option B is simpler and avoids server involvement for a purely client-side action. The capsule already supports inline behavior for route links — the same pattern works for copy buttons.

### Capsule JS Addition

```javascript
// In capsule runtime — click handler for [data-copy] elements
document.addEventListener('click', e => {
    const el = e.target.closest('[data-copy]');
    if (!el) return;
    navigator.clipboard.writeText(el.dataset.copy);
    el.classList.add('copied');
    setTimeout(() => el.classList.remove('copied'), 2000);
});
```

**Files**: `rwire/src/components/copy_button.rs`, `rwire/src/capsule_gen.rs` (optional inline JS), `rwire/src/components/mod.rs`

---

## Step 2.3 — LogoCloud Component

A responsive grid of logos for social proof. Lower priority — can be deferred if no logos available yet.

### API Design

```rust
LogoCloud::new()
    .title("Built for Rust teams")
    .logo("Company A", "/logos/a.svg")
    .logo("Company B", "/logos/b.svg")
    .logo("Company C", "/logos/c.svg")
    .grayscale(true)
    .build()
```

### Structure

```
┌─────────────────────────────────┐
│     "Built for Rust teams"      │  ← title (optional)
├────┬────┬────┬────┬────┬───────┤
│ A  │ B  │ C  │ D  │ E  │  F   │  ← responsive grid
└────┴────┴────┴────┴────┴───────┘
```

### Tokens Used

- Grid: `St::DisplayGrid`, `St::GridColsAuto`, `St::GapLg`
- Images: `St::Grayscale`, `St::OpacityHalf` + hover `St::Opacity75` + remove `St::Grayscale`
- Centering: `St::ItemsCenter`, `St::JustifyCenter`

**Files**: `rwire/src/components/logo_cloud.rs`, `rwire/src/components/mod.rs`

---

## Verification Checklist

- [ ] Footer renders with logo, 3 columns, copyright
- [ ] Footer links: internal use `Link::to()`, external use `<a target="_blank">`
- [ ] Footer responsive: collapses on small screens
- [ ] CopyButton: clicking copies text to clipboard
- [ ] CopyButton: visual feedback (icon change or class toggle)
- [ ] LogoCloud: renders grid of images with optional grayscale
- [ ] `cargo clippy --workspace` — zero warnings
- [ ] `cargo test --workspace` — all pass
