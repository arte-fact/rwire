### Fixed: `<style>` and `<script>` text in the static paint

- `ElementBuilder::to_static_html` (and the SSR paint) entity-escaped the text
  of a `<style>` or `<script>` element, which is raw text: `a>b` became
  `a&gt;b` and the rule was lost. It is now written as is, only kept from
  closing the element early (`</` becomes `<\/`).
- In a `<script>`, `<!--` is also written `<\!--` (the same string to
  JavaScript), so the text cannot put the HTML parser in its "script data
  escaped" state and swallow the rest of the document. `<style>` text is left
  as is (no such state; `<\!--` would change the CSS outside a string).
