//! Verifies the `#[renderer]` macro infers field-level dependencies, so a
//! region only re-renders when a field it actually reads changes — and falls
//! back to "always" when the state param is used opaquely.

use rwire::builder::extract_renderers;
use rwire::state::ChangeSet;
use rwire::{el, renderer, El, ElementBuilder, State};

#[derive(State, Default)]
#[storage(memory)]
struct Two {
    a: i32,
    b: i32,
}

#[renderer]
fn reads_a(s: &Two) -> ElementBuilder {
    el(El::Span).text(&s.a.to_string())
}

#[renderer]
fn reads_b(s: &Two) -> ElementBuilder {
    el(El::Span).text(&s.b.to_string())
}

// Passes the whole state to a helper — the macro can't track fields, so it must
// conservatively depend on everything.
#[renderer]
fn opaque(s: &Two) -> ElementBuilder {
    helper(s)
}

fn helper(s: &Two) -> ElementBuilder {
    el(El::Span).text(&s.a.to_string())
}

fn deps_of(el: &ElementBuilder) -> rwire::RendererDeps {
    extract_renderers(el)[0].deps()
}

#[test]
fn renderer_depends_only_on_fields_it_reads() {
    let changed_a = ChangeSet::from_fields(&[Two::FIELD_A]);
    let changed_b = ChangeSet::from_fields(&[Two::FIELD_B]);

    let a = deps_of(&reads_a());
    assert!(
        a.needs_update(changed_a),
        "reads_a must update when a changes"
    );
    assert!(
        !a.needs_update(changed_b),
        "reads_a must NOT update when only b changes"
    );

    let b = deps_of(&reads_b());
    assert!(b.needs_update(changed_b));
    assert!(!b.needs_update(changed_a));
}

#[test]
fn opaque_state_use_falls_back_to_always() {
    let only_b = ChangeSet::from_fields(&[Two::FIELD_B]);
    // `opaque` only textually reads `a` (via helper) but passes the whole state,
    // so it must conservatively re-render on any change.
    assert!(deps_of(&opaque()).needs_update(only_b));
}
