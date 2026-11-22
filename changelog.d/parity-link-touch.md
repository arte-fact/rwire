### Added: `Link::touch(href, text)`

- A navigation link that is a control (a sidebar footer's link): `Link::to`
  plus `data-touch`, so under a coarse pointer or below `Theme::touch_below` it
  becomes an `inline-flex` 44 px target (the touch rule now gives
  `a[data-touch]` `display:inline-flex;align-items:center`, at zero
  specificity). A link in running text stays `Link::to` and does not grow.
