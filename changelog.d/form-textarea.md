### Added: `Field::textarea` in `Form`; validation rules reach the markup

- `Field::textarea(name)` renders a `<textarea name=… id=… rows=…>`, submitted
  with the form like any field, with the form's themed styling and a vertical
  resize handle.
  - Its initial `.value()` is its text.
  - It gets a stable id, `field-<name>` (or `.id(…)`): a re-render matches the
    live element by that id, so what the user typed stays in the right field
    even when fields before it appear or disappear. Give it a new id to empty
    it after a submit.
  - `.rows(n)` sets the visible lines (default 4).
- **Behaviour change: the validation rules now reach the markup.** Until now
  only `required` was emitted. `min_length`, `max_length`, `pattern` and the
  numeric `Min`/`Max` were recorded and silently ignored. They are now emitted
  as the native constraints (`minlength`, `maxlength`, `pattern`, `min`,
  `max`) wherever HTML defines them, and the browser enforces them on submit.
  A textarea takes no `pattern`, and a text input takes no `min`/`max`.
- `Field::id(…)` sets any field's id. Inputs still have none by default.
- **Breaking:** `FieldType` gains a `Textarea` variant. An exhaustive `match`
  on it needs an arm.
