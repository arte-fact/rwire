### Changed: rwire-markdown no longer loads remote images, and keeps fewer link schemes

- **Remote images are off by default.** An image whose URL is absolute
  `http(s)://` or protocol-relative (`//host/…`) is rendered as its alt text
  instead of being loaded: a remote image tells its server who reads the page
  and when. Same-origin images and raster `data:image/*` are kept. To load
  them again (trusted content only): `Markdown::remote_images(true)`, or
  `parse_markdown_with_options` with `ParseOptions { remote_images: true, .. }`.
- **Links** keep their `href` only when relative, an anchor, `http:`,
  `https:` or `mailto:`; `tel:` (allowed until now) and every other scheme
  are rendered as text. `rwire::url_safety::is_allowed_link` follows; the new
  `url_safety::is_remote` tells a remote URL. The rule for attributes the
  application writes (`is_executable_attr`) and the runtime are unchanged.
