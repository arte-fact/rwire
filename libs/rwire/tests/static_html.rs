//! `ElementBuilder::to_static_html` — serializing an element tree to a
//! self-contained HTML string for pre-capsule pages (e.g. the auth login).

use rwire::style::Style;
use rwire::{At, Av, El, St, el};

#[test]
fn serializes_tag_text_and_children() {
    let tree = el(El::Div).class("box").append([el(El::Span).text("hi")]);
    assert_eq!(
        tree.to_static_html(),
        "<div class=\"box\"><span>hi</span></div>"
    );
}

#[test]
fn void_elements_have_no_closing_tag() {
    let html = el(El::Input).attr("name", "username").to_static_html();
    assert_eq!(html, "<input name=\"username\">");
    assert!(!html.contains("</input>"));
}

#[test]
fn escapes_text_and_attribute_values() {
    let html = el(El::Div)
        .attr("title", "a\"<&>")
        .text("<b>&")
        .to_static_html();
    assert_eq!(
        html,
        "<div title=\"a&quot;&lt;&amp;&gt;\">&lt;b&gt;&amp;</div>"
    );
}

#[test]
fn inline_style_passes_through_as_attribute() {
    let html = el(El::Div)
        .style(Style::new().display("none"))
        .to_static_html();
    assert!(
        html.starts_with("<div style=\"display:none"),
        "got: {html}"
    );
}

#[test]
fn utility_token_becomes_u_class() {
    let code = St::DisplayFlex.as_u16();
    let html = el(El::Div).st([St::DisplayFlex]).to_static_html();
    assert_eq!(html, format!("<div class=\"u{code}\"></div>"));
}

#[test]
fn typed_and_bool_attributes_render() {
    let html = el(El::Input)
        .at(At::Type, Av::Password)
        .bool_attr(At::Required)
        .to_static_html();
    assert_eq!(html, "<input type=\"password\" required>");
}

#[test]
fn attribute_order_is_class_then_string_then_typed() {
    let html = el(El::Input)
        .class("field")
        .attr("name", "password")
        .at(At::Type, Av::Password)
        .to_static_html();
    assert_eq!(
        html,
        "<input class=\"field\" name=\"password\" type=\"password\">"
    );
}
