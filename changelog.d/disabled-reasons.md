### Added: disabled states that say why (`Composer::disabled`, `Button::title`)

- `Composer::disabled(reason)` disables the field and the send button. The
  reason shows in the empty field (its placeholder, so it is read before any
  click) and as both tooltips. No submit or draft handler is bound, and the
  keyboard hint is hidden.
- `Button::title(…)` sets the native tooltip, and `Button::attr(k, v)` any
  other attribute, on the button element itself.
- **Behaviour change:** a disabled `Button` no longer sets
  `pointer-events:none`, so its tooltip shows and its not-allowed cursor
  appears. The `disabled` attribute still blocks the click, and a disabled
  button gets no hover styling.
- **Fails loudly:** an attribute, class, style or text set on what
  `Button::on_click` / `submit` returns (the action-reflection wrapper,
  `display:contents`) never reached the button. Debug builds now panic at that
  call, naming the method; release builds still ignore it.
  - Mechanism: `ElementBuilder::boxless(what)` marks such wrappers.
  - Guarded setters: `attr`, `data`, `id`, `at`, `at_str`, `bool_attr`,
    `class`, `text`, `style`, `st`, `pseudo` / `hover`.
- `At::Title` (`title`) joins the attribute tokens.
