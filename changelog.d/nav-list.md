### Added: `NavList`, `NavGroup`, `NavRow` (dense grouped navigation)

- **`NavList::new().aria_label(…).group(…)`**: a `<nav>` of groups.
- **`NavGroup::new(label).count(n).badge(el).leading(el).folded(bool).on_toggle(h).rows([…])`**:
  a header button in spaced capitals (11 px, 600, `.06em`, the soft ink:
  `St::NavGroupHead`) with a chevron (`St::ChevronFold`, no turn under
  `prefers-reduced-motion`; `St::RotateNeg90` when folded), an optional
  leading element and badge, the count at the end; `aria-expanded` and
  `aria-controls` name the rows' `role="group"` container. A folded group
  renders its header only.
- **`NavRow::new(href).leading(el).title(strong, muted_suffix).trailing(el).meta(text, mono_suffix).current(bool).attention(bool)`**:
  a client-routed link on a `14px | 1fr | auto` grid (`St::GridNavRow`),
  padded by the density (`St::PadRow`), 7 px radius (`St::RoundedNavRow`),
  title 13.5 px (`St::TextNavTitle`), meta line 12 px (`St::TextNavMeta`,
  `St::ColNavMeta`) with a 10 px mono suffix (`St::TextNavHash`). The current
  row is tinted (`BgAccentSubtle`) with an accent hairline (`St::NavCurrent`)
  and `aria-current="page"`; `attention` makes its title bold.
