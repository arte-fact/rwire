### Added: complete "Conversation" gallery with stable testids (R3, iteration 7)

- The Conversation demo now shows every screen-2 / screen-4 state: a user
  bubble, a long Markdown answer (headings, list, quote, table, long code
  block that scrolls inside its `pre`), a tool call ok and one ko, a pending
  and a resolved decision card, a sending / unconfirmed / failed message, a
  writing row, the day divider and note chrome, and the docked composer.
- `TypingIndicator` now carries `data-testid="typing-indicator"`; together
  with the other `data-testid`s of the conversation components
  (`chat-thread`, `chat-column`, `chat-entry`, `chat-avatar`, `chat-bubble`,
  `chat-prose`, `chat-time`, `chat-delivery`, `chat-delivery-actions`,
  `markdown`, `markdown-code`, `tool-call`, `tool-summary`, `tool-status`,
  `tool-output`, `decision-card` and children, `composer-dock`,
  `composer-card`, `composer-send`, `composer-hint`, `chat-divider`,
  `chat-note`, `page-header` and children, `badge-status`) the app can pin
  its parity selectors on stable hooks.
- Captures at 1280 and 375 px, light and dark, live in the group's
  `medias-r3/` folder.
