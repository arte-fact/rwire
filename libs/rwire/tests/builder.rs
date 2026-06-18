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

    let bytes = ctx.finish();

    // Verify structure was encoded
    assert!(!bytes.is_empty());
}

#[test]
fn test_text_encoding_analysis() {

    // Build a tree with repeated words
    let page = el(El::Div).append([
        el(El::Span).text("Hello world"),
        el(El::Span).text("Hello there"),
        el(El::Span).text("world peace"),
    ]);

    let mut ctx = BuildContext::new();
    let state: () = ();
    ctx.collect_symbols(&page, &state);

    // Build word table and text encodings
    ctx.build_word_table();

    // Check word table was built with repeated words
    let word_table = ctx.word_table();
    assert!(!word_table.is_empty(), "Word table should have common words");
    // "Hello" and "world" each appear 2 times, so they should be in the table
    assert!(
        word_table.contains(&"Hello".to_string()),
        "Word table should contain 'Hello'"
    );
    assert!(
        word_table.contains(&"world".to_string()),
        "Word table should contain 'world'"
    );
}

#[test]
fn test_integer_text_encoding() {
    use rwire::builder::TextEncoding;

    // Build a tree with integer text
    let counter = el(El::Div).append([
        el(El::Span).text("42"),
        el(El::Span).text("-10"),
        el(El::Span).text("0"),
    ]);

    let mut ctx = BuildContext::new();
    let state: () = ();
    ctx.collect_symbols(&counter, &state);
    ctx.build_word_table();

    // Check that integer texts get Int encoding
    let enc_42 = ctx.get_text_encoding("42");
    let enc_neg = ctx.get_text_encoding("-10");
    let enc_zero = ctx.get_text_encoding("0");

    assert!(
        matches!(enc_42, Some(TextEncoding::Int(42))),
        "Expected Int(42), got {:?}",
        enc_42
    );
    assert!(
        matches!(enc_neg, Some(TextEncoding::Int(-10))),
        "Expected Int(-10), got {:?}",
        enc_neg
    );
    assert!(
        matches!(enc_zero, Some(TextEncoding::Int(0))),
        "Expected Int(0), got {:?}",
        enc_zero
    );
}

