### Fixed: contradictory `st()` tokens — the last call wins, the stylesheet order is fixed

- `.st([St::DisplayFlex])` then `.st([St::DisplayNone])` did not hide the
  element. Utility rules have equal specificity, so the stylesheet order
  decided, not the call order.
- **The last call wins.** `ElementBuilder::st` / `style_util` now drop the
  earlier tokens whose every CSS property the new token sets again. Shorthands
  are expanded: `padding` covers `padding-inline`, `inset` covers `top`,
  `border` covers `border-bottom`, `font` covers `font-size`.
- **Partial overlaps.** A token that sets only part of an earlier one's
  properties cannot replace it. Both stay, and the shared properties go to the
  higher token code. When that is the earlier token, the later call loses, and
  debug builds report it once per pair on stderr.
  - **Known case: hiding after a compound token.**
    `.st([St::FlexCenter]).st([St::DisplayNone])` still shows the element:
    `DisplayNone` covers only part of `FlexCenter`, and `FlexCenter` wins
    `display`. Debug builds report it; release builds say nothing. The
    `hidden` attribute does not help (the base CSS has no `[hidden]` rule). To
    hide safely, choose the display token
    (`if visible { FlexCenter } else { DisplayNone }`) or do not render the
    element.
- **The stylesheet order no longer depends on the page's history.** The
  runtime used to append each lazily delivered rule at the end of the
  stylesheet, so the order depended on which classes a connection met first.
  On a later page, a base rule could land after its own breakpoint rule and
  cancel it. Each `STYLE_DEF` rule now carries its place, and the runtime
  inserts it there:
  - utilities in code order, then pseudo-classes, then breakpoints;
  - **wire format change:** `[count, (order, rule_len, rule)…]`;
  - a rule the browser rejects is skipped. The old fallback rewrote the
    stylesheet's text, which dropped every rule inserted before it.
- **Components fixed.** The partial-override check found three real bugs, now
  fixed:
  - `FontInheritAll` (`font:inherit`, a higher code than `TextXs` / `TextSm` /
    `TextBase`) cancelled the font size of `Button`, `Chip`, `Input`,
    `Select`, `Textarea` and `Form` fields. The base reset already applies
    `font:inherit` to form controls, so the token is gone from them. **Button
    and chip sizes now change the font size as designed.**
  - The slider's `RoundedSm` never applied (`SliderTrack` sets the radius);
    it is removed.
