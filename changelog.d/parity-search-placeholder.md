### Changed: the search placeholder is the faint ink

- `Input::search()`'s placeholder is `TextFaint` (`--j2`, AA in both modes),
  apart from the typed text; it was the soft ink, too close to the typed text
  in dark mode. Other inputs keep the soft ink.
