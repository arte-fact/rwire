### Added: `Field::textarea` in `Form`; validation rules reach the markup

- `Field::textarea(name)` renders a `<textarea name=… id=… rows=…>`, submitted
  with the form like any field, with the form's themed styling and a vertical
  resize handle.
  - Its initial `.value()` is its text.
  - It gets a stable id, `field-<name>` (or `.id(…)`): a re-render matches the
    live element by that id, so what the user typed stays in the right field
    even when fields before it appear or disappear. Give it a new id to empty
    it after a submit.
  - With a `.label()`, the wrapping `<label>` gets the id `<id>-label` (as
    does the label of any field given an id), so the wrapper is matched by id
    too and the typed text survives the same re-renders.
  - `.rows(n)` sets the visible lines (default 4).
- **Behaviour change: the validation rules now reach the markup.** Until now
  only `required` was emitted. `min_length`, `max_length`, `pattern` and the
  numeric `Min`/`Max` were recorded and silently ignored. They are now emitted
  as the native constraints (`minlength`, `maxlength`, `pattern`, `min`,
  `max`), and the browser enforces them on submit.
  - `pattern`, `minlength` and `maxlength` are now applied by the browser: a
    form that used to submit can now be blocked. A pattern that is invalid in
    the browser's `v`-flag regex mode is ignored by the browser.
  - `min`/`max` are emitted only on `Number` and `Range` fields (they hold a
    number; a date or time field's `min` would need a date string).
  - A textarea takes no `pattern`.
- `Field::id(…)` sets any field's id. Inputs still have none by default.
- **Breaking:** `FieldType` gains a `Textarea` variant. An exhaustive `match`
  on it needs an arm.
