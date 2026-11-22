### Fixed: `<style>` and `<script>` text in the static paint

- `ElementBuilder::to_static_html` (and the SSR paint) entity-escaped the text
  of a `<style>` or `<script>` element, which is raw text: `a>b` became
  `a&gt;b` and the rule was lost. It is now written as is, only kept from
  closing the element early (`</` becomes `<\/`).
