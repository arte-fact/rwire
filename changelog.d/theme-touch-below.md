### Added: `Theme::touch_below(px)`

- Sets the viewport width below which interactive controls get 44 px touch
  targets under a fine pointer too (a coarse pointer gets them at any width).
  The default is the `sm` breakpoint, 640 px, as before; `sm` itself does not
  move. mcp-agents uses 760, the phone threshold of its old front.
- The touch-target rule moves from the base CSS to the theme CSS so it can
  follow the theme.
