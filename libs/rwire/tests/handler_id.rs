//! Stable handler ids.
//!
//! The `#[handler]` macro assigns each handler a deterministic, nonzero id
//! (FNV-1a of its module path + name) so a binding resolves to the same handler
//! across renders, process restarts, and reconnects — the basis for reusing DOM
//! nodes without misrouting events.

// Rust guideline compliant 2026-02-21

use rwire::{handler, stable_handler_id, State};

#[derive(State, Default)]
struct Counter {
    n: u32,
}

#[handler]
fn inc(s: &mut Counter) {
    s.n += 1;
}

#[handler]
fn dec(s: &mut Counter) {
    s.n = s.n.saturating_sub(1);
}

#[test]
fn macro_assigns_stable_nonzero_id() {
    let a = inc().handler_id();
    let b = inc().handler_id();
    assert_ne!(a, 0, "the macro must assign a nonzero id");
    assert_eq!(
        a, b,
        "the same handler must yield the same id across calls/renders"
    );
    assert_ne!(
        inc().handler_id(),
        dec().handler_id(),
        "distinct handlers need distinct ids"
    );
}

#[test]
fn stable_handler_id_is_deterministic_distinct_and_nonzero() {
    assert_eq!(stable_handler_id("m", "f"), stable_handler_id("m", "f"));
    assert_ne!(stable_handler_id("m", "f"), stable_handler_id("m", "g"));
    assert_ne!(stable_handler_id("a", "f"), stable_handler_id("b", "f"));
    assert_ne!(stable_handler_id("any", "name"), 0);
}

#[test]
fn handler_ids_survive_the_wire_varint() {
    // Handler ids travel as 3-byte varints; an id beyond VARINT_MAX would be
    // truncated on the wire and never resolve back. Both the macro-assigned id
    // and the raw helper must round-trip unchanged.
    use rwire::protocol::varint::{read_varint, write_varint};
    let check = |id: u32| {
        let mut buf = Vec::new();
        write_varint(&mut buf, id);
        assert_eq!(
            read_varint(&buf),
            Some((id, buf.len())),
            "id {id} must round-trip"
        );
    };
    check(inc().handler_id());
    check(dec().handler_id());
    for (m, n) in [
        ("a::b::c", "do_thing"),
        ("x", "y"),
        ("very::long::module::path", "handler"),
    ] {
        check(stable_handler_id(m, n));
    }
}
