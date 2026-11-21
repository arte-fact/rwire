//! Every catalog component, in every variant and both bool states, builds
//! without a later `.st()` token losing to an earlier one (debug builds count
//! them; see `ElementBuilder::st`). Such a pair is a style that never
//! applies: `FontInheritAll` then `TextXs` kept the inherited font size.

use rwire_components::catalog::catalog;

#[test]
#[cfg(debug_assertions)]
fn no_component_has_a_later_token_that_loses() {
    use std::sync::atomic::Ordering;
    let before = rwire::builder::PARTIAL_OVERRIDES.load(Ordering::Relaxed);
    for entry in catalog() {
        let sizes: Vec<usize> = entry.variants.iter().map(|v| v.values.len()).collect();
        for (axis, &n) in sizes.iter().enumerate() {
            for k in 0..n {
                let mut variants = vec![0usize; sizes.len()];
                variants[axis] = k;
                for b in [false, true] {
                    let bools = vec![b; entry.bool_props.len()];
                    let _ = (entry.build_demo)(&variants, &bools).to_static_html();
                }
            }
        }
        let _ = (entry.build_demo)(&vec![0; sizes.len()], &vec![false; entry.bool_props.len()])
            .to_static_html();
    }
    let after = rwire::builder::PARTIAL_OVERRIDES.load(Ordering::Relaxed);
    assert_eq!(
        after - before,
        0,
        "see the `rwire: st(...)` lines on stderr"
    );
}
