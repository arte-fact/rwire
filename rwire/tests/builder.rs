//! Tests for the element builder API.

use rwire::builder::BuildContext;
use rwire::{el, El};

#[test]
fn test_el_builder_basic() {
    let elem = el(El::Div);
    assert_eq!(elem.el_type(), El::Div);
    assert!(elem.text_content().is_none());
    assert!(elem.class_name().is_none());
    assert!(elem.children().is_empty());
}

#[test]
fn test_el_builder_text() {
    let elem = el(El::Span).text("hello");
    assert_eq!(elem.text_content(), Some("hello"));
}

#[test]
fn test_el_builder_class() {
    let elem = el(El::Div).class("container");
    assert_eq!(elem.class_name(), Some("container"));
}

#[test]
fn test_el_builder_attrs() {
    let elem = el(El::Input)
        .attr("type", "text")
        .attr("placeholder", "Enter name");

    let attrs = elem.attributes();
    assert_eq!(attrs.len(), 2);
    assert_eq!(attrs[0], ("type".to_string(), "text".to_string()));
    assert_eq!(
        attrs[1],
        ("placeholder".to_string(), "Enter name".to_string())
    );
}

#[test]
fn test_el_builder_children() {
    let elem = el(El::Div).append([el(El::Span).text("one"), el(El::Span).text("two")]);

    assert_eq!(elem.children().len(), 2);
    assert_eq!(elem.children()[0].text_content(), Some("one"));
    assert_eq!(elem.children()[1].text_content(), Some("two"));
}

#[test]
fn test_el_builder_chaining() {
    let elem = el(El::Button)
        .text("Click me")
        .class("primary")
        .attr("disabled", "true");

    assert_eq!(elem.el_type(), El::Button);
    assert_eq!(elem.text_content(), Some("Click me"));
    assert_eq!(elem.class_name(), Some("primary"));
    assert_eq!(elem.attributes().len(), 1);
}

#[test]
fn test_el_builder_nested() {
    let elem = el(El::Div).class("outer").append([el(El::Div)
        .class("inner")
        .append([el(El::Span).text("deep")])]);

    assert_eq!(elem.class_name(), Some("outer"));
    assert_eq!(elem.children()[0].class_name(), Some("inner"));
    assert_eq!(
        elem.children()[0].children()[0].text_content(),
        Some("deep")
    );
}

#[test]
fn test_build_context_new() {
    let ctx = BuildContext::new();
    assert!(ctx.handlers().is_empty());
    assert!(ctx.used_elements().is_empty());
    assert!(ctx.used_events().is_empty());
}

#[test]
fn test_build_context_tracks_elements() {
    let elem = el(El::Div).append([el(El::Button), el(El::Span)]);

    let mut ctx = BuildContext::new();
    let state: () = ();
    ctx.collect_symbols(&elem, &state);

    let used = ctx.used_elements();
    assert!(used.contains(&El::Div.as_u8()));
    assert!(used.contains(&El::Button.as_u8()));
    assert!(used.contains(&El::Span.as_u8()));
    assert_eq!(used.len(), 3);
}

#[test]
fn test_build_context_emits_opcodes() {
    let elem = el(El::Div).class("test").text("hello");

    let mut ctx = BuildContext::new();
    let state: () = ();
    ctx.collect_symbols(&elem, &state);
    ctx.emit(&elem, &state);
    let bytes = ctx.finish();

    // Should contain: SYMBOLS, CREATE, SET_CLASS, SET_TEXT, APPEND, END
    assert!(!bytes.is_empty());
    // First byte should be SYMBOLS (0xF0)
    assert_eq!(bytes[0], 0xF0);
    // Last byte should be BATCH_END (0xFF)
    assert_eq!(bytes[bytes.len() - 1], 0xFF);
}

#[test]
fn test_build_context_symbol_interning() {
    // Same string used multiple times should be interned once
    let elem = el(El::Div)
        .class("shared")
        .append([el(El::Span).class("shared")]);

    let mut ctx = BuildContext::new();
    let state: () = ();
    ctx.collect_symbols(&elem, &state);
    ctx.emit(&elem, &state);
    let bytes = ctx.finish();

    // Count occurrences of "shared" in the symbol table
    let bytes_str = String::from_utf8_lossy(&bytes);
    let count = bytes_str.matches("shared").count();
    assert_eq!(count, 1, "Symbol 'shared' should be interned only once");
}

#[test]
fn test_element_builder_clone() {
    let elem = el(El::Div).class("test").text("hello");
    let cloned = elem.clone();

    assert_eq!(cloned.el_type(), El::Div);
    assert_eq!(cloned.class_name(), Some("test"));
    assert_eq!(cloned.text_content(), Some("hello"));
}

#[test]
fn test_is_synced() {
    let regular = el(El::Div);
    assert!(!regular.is_synced());
}

#[test]
fn test_all_element_types() {
    let elements = [
        (El::Div, "div"),
        (El::Span, "span"),
        (El::Button, "button"),
        (El::Input, "input"),
        (El::P, "p"),
        (El::H1, "h1"),
        (El::H2, "h2"),
        (El::A, "a"),
        (El::Form, "form"),
    ];

    for (el_type, _name) in elements {
        let elem = el(el_type);
        assert_eq!(elem.el_type(), el_type);
    }
}

#[test]
fn test_build_context_default() {
    let ctx = BuildContext::default();
    assert!(ctx.handlers().is_empty());
}

#[test]
fn test_complex_tree() {
    let form = el(El::Form).class("login-form").append([
        el(El::Div).class("field").append([
            el(El::Span).text("Username:"),
            el(El::Input).attr("type", "text").attr("name", "username"),
        ]),
        el(El::Div).class("field").append([
            el(El::Span).text("Password:"),
            el(El::Input)
                .attr("type", "password")
                .attr("name", "password"),
        ]),
        el(El::Button).text("Login").attr("type", "submit"),
    ]);

    let mut ctx = BuildContext::new();
    let state: () = ();
    ctx.collect_symbols(&form, &state);
    ctx.emit(&form, &state);

    // Check used elements before finish() consumes the context
    let used = ctx.used_elements();
    assert!(used.contains(&El::Form.as_u8()));
    assert!(used.contains(&El::Div.as_u8()));
    assert!(used.contains(&El::Span.as_u8()));
    assert!(used.contains(&El::Input.as_u8()));
    assert!(used.contains(&El::Button.as_u8()));

    let bytes = ctx.finish();

    // Verify structure was encoded
    assert!(!bytes.is_empty());
}
