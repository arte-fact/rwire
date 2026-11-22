### Added: `ChatEntry::delivery` — outbox states (R3, iteration 3)

- `Delivery::Sending | Sent | Unconfirmed | Failed` with a status line and an
  actions row under the body: `ChatEntry::delivery(state, status, actions)`
  or `ChatItem::delivery(ctx)` returning a `ChatDelivery`.
- `Sending` sets the entry at 60% opacity; `Unconfirmed` and `Failed` give
  the body a dashed hairline outline. The status line is `role=status`, or
  `role=alert` (error ink) when failed; the actions (Resend, Withdraw) render
  in a row under it. New `St` tokens: `BorderDashed`, `Opacity60`.
- Gallery: the Conversation demo shows a sending, an unconfirmed and a
  failed message. Measured at 1280/375 light/dark: sending opacity 0.6
  role=status no actions; unconfirmed/failed `border:1px dashed`, actions
  Resend+Withdraw, failed role=alert in the error ink.
