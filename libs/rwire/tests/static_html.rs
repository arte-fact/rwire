//! `ElementBuilder::to_static_html` — serializing an element tree to a
//! self-contained HTML string for pre-capsule pages (e.g. the auth login).

use rwire::style::Style;
use rwire::{el, At, Av, El, St};

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
    assert!(html.starts_with("<div style=\"display:none"), "got: {html}");
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
fn collect_style_keys_walks_tree_and_pseudo_groups() {
    use rwire::StyleKey;
    use std::collections::BTreeSet;
    let tree = el(El::Div)
        .st([St::BgApp])
        .append([el(El::Input).st([St::WFull]).focus([St::BorderAccent])]);
    let mut keys = BTreeSet::new();
    tree.collect_style_keys(&mut keys);
    assert!(keys.contains(&StyleKey::Util(St::BgApp.as_u16())));
    assert!(keys.contains(&StyleKey::Util(St::WFull.as_u16())));
    assert!(
        keys.iter()
            .any(|k| matches!(k, StyleKey::Pseudo(_, st) if *st == St::BorderAccent.as_u16())),
        "focus pseudo for BorderAccent should be collected"
    );
}

#[test]
fn render_static_page_inlines_theme_and_utility_css() {
    let code = St::BgApp.as_u16();
    let tree = el(El::Div).st([St::BgApp]).text("hi");
    let html = rwire::render_static_page(&rwire::CapsuleConfig::new(), "claw", &tree);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<title>claw</title>"));
    assert!(
        html.contains(":root{"),
        "theme :root vars should be inlined"
    );
    assert!(
        html.contains(&format!(".u{code}{{")),
        "utility rule for the used token should be inlined"
    );
    assert!(html.contains(&format!("class=\"u{code}\"")));
    assert!(html.contains(">hi</div>"));
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

#[test]
fn synced_regions_render_default_state_and_collect_styles() {
    use rwire::builder::ElementBuilder;
    use rwire::style_tokens::StyleKey;
    use rwire::{renderer, State};

    #[derive(State, Default)]
    #[storage(memory)]
    struct S {
        n: i32,
    }

    #[renderer]
    fn count(state: &S) -> ElementBuilder {
        el(El::Span).st([St::TextSm]).text(&state.n.to_string())
    }

    let tree = el(El::Div).st([St::PMd]).append([count()]);
    let html = tree.to_static_html();
    assert!(html.contains(">0<"), "default state rendered: {html}");

    let rules = tree.static_style_rules();
    assert!(rules.contains(&StyleKey::Util(St::PMd as u16)));
    assert!(
        rules.contains(&StyleKey::Util(St::TextSm as u16)),
        "styles inside the synced region are collected"
    );
}

/// Like the runtime, the static HTML drops URL attributes that would run code,
/// whether set as a raw attribute or a typed one; other URLs and attributes stay.
#[test]
fn executable_urls_are_dropped() {
    let html = el(El::Div)
        .append([
            el(El::A)
                .at_str(At::Href, "JaVa\tScRiPt:alert(1)")
                .text("a"),
            el(El::A)
                .attr("href", "javascript&colon;alert(1)")
                .text("b"),
            el(El::Img).attr("src", "data:image/svg+xml,<svg onload=alert(1)>"),
            el(El::Form).attr("action", "vbscript:x"),
            el(El::Button).attr("formaction", "javascript:x"),
            el(El::A)
                .at_str(At::Href, "magnet:?xt=urn:btih:x")
                .text("c"),
            el(El::Img).attr("src", "data:image/png;base64,AA"),
            el(El::Img).attr("src", "data:image;base64,BB"),
            el(El::A).attr("title", "javascript: the good parts"),
        ])
        .to_static_html();
    assert!(!html.contains("script:alert"), "{html}");
    assert!(!html.contains("svg"), "{html}");
    assert!(!html.contains("vbscript"), "{html}");
    assert!(!html.contains("formaction"), "{html}");
    assert!(html.contains("href=\"magnet:?xt=urn:btih:x\""), "{html}");
    assert!(html.contains("src=\"data:image/png;base64,AA\""), "{html}");
    assert!(html.contains("src=\"data:image;base64,BB\""), "{html}");
    assert!(
        html.contains("title=\"javascript: the good parts\""),
        "{html}"
    );
}

/// E2 criterion (renderer wrappers): a synced region is wrapped in a `<span>`
/// in the static paint as in the live page (`CREATE_SYNCED` makes a
/// class-less `<span id="__synced_N">`), and an unstyled wrapper is
/// `display:contents` in both, so a region inside a grid or a flex row adds
/// no box of its own: the two renders lay out the same boxes.
#[test]
fn a_region_wrapper_adds_no_box_in_the_static_paint_as_live() {
    #[derive(Default)]
    struct Cells;
    let region = rwire::ElementBuilder::synced::<Cells, _>(|_| el(El::Div).text("cell"));
    let grid = el(El::Div)
        .st([St::DisplayGrid])
        .append([region, el(El::Div).text("next")]);
    let html = grid.to_static_html();
    assert!(
        html.contains("<span data-rwr><div>cell</div></span>"),
        "{html}"
    );
    // The base CSS gives both wrappers, unstyled, no box.
    let base = rwire::generate_base_css();
    assert!(
        base.contains("[id^=__synced_]:not([class]),[data-rwr]:not([class]){display:contents}"),
        "{base}"
    );
    // A styled wrapper (`render_x().st([St::Flex1])`) keeps its box: it has a class.
    let styled = rwire::ElementBuilder::synced::<Cells, _>(|_| el(El::Div)).st([St::Flex1]);
    assert!(styled
        .to_static_html()
        .starts_with("<span data-rwr class=\"u"));
}

/// A region's wrapper carries the same classes in the static paint and live:
/// its style tokens, pseudo-classes and breakpoints, applied in both; its own
/// `.class()` and attributes in neither (the live `CREATE_SYNCED` wrapper
/// never had them).
#[test]
fn a_region_wrapper_has_the_same_classes_static_and_live() {
    use rwire::builder::BuildContext;
    use rwire::protocol::opcodes::{CREATE_SYNCED, STYLE_BREAKPOINT, STYLE_PSEUDO};
    use rwire::Bp;

    #[derive(Default)]
    struct Cells;
    let region = || {
        rwire::ElementBuilder::synced::<Cells, _>(|_| el(El::Div).text("cell"))
            .st([St::DisplayFlex])
            .breakpoint(Bp::Md, [St::Flex1])
            .hover([St::BgHover])
            .class("x")
            .attr("data-k", "v")
    };

    // Static: the token classes, not the region's own class or attributes.
    let html = region().to_static_html();
    let flex1 = St::Flex1.as_u16();
    let hover = St::BgHover.as_u16();
    let md = Bp::Md.as_u8();
    let open = &html[..html.find('>').unwrap()];
    assert!(open.contains(&format!("b{md}u{flex1}")), "{html}");
    assert!(open.contains(&format!("h0u{hover}")), "{html}");
    assert!(!open.contains(" x") && !open.contains("\"x"), "{html}");
    assert!(!open.contains("data-k"), "{html}");

    // Live: the wrapper (ref 0, the first CREATE_SYNCED) gets the same breakpoint
    // and pseudo groups.
    let root = region();
    let mut ctx = BuildContext::new();
    let cells = Cells;
    let mut states: std::collections::HashMap<
        std::any::TypeId,
        &(dyn std::any::Any + Send + Sync),
    > = std::collections::HashMap::new();
    states.insert(std::any::TypeId::of::<Cells>(), &cells);
    ctx.collect_symbols_multi(&root, &states);
    ctx.emit_multi(&root);
    let bytes = ctx.finish();
    assert!(bytes.contains(&CREATE_SYNCED));
    // [op, wrapper ref 0, code, 1 token, the token's varint]
    let has = |op: u8, code: u8, st: u16| {
        let mut want = vec![op, 0, code, 1];
        rwire::protocol::varint::write_varint(&mut want, u32::from(st));
        bytes.windows(want.len()).any(|w| w == &want[..])
    };
    assert!(
        has(STYLE_BREAKPOINT, md, flex1),
        "live wrapper: no md breakpoint"
    );
    assert!(has(STYLE_PSEUDO, 0, hover), "live wrapper: no hover");
}

/// A `<style>` (or `<script>`) element's text is raw text: not entity-escaped
/// (the shell's child combinators, `>`, must survive the static paint), only
/// kept from closing the element early.
#[test]
fn style_text_is_raw_in_the_static_paint() {
    let html = el(El::Style).text("a>b{c:d}</style><b>").to_static_html();
    assert!(html.contains("a>b{c:d}"), "{html}");
    assert!(!html.contains("&gt;"), "{html}");
    assert!(!html.contains("</style><b>"), "{html}");
}

/// A link that is a control (a sidebar footer's link) gets the touch target;
/// a link in running text does not.
#[test]
fn a_touch_link_carries_the_touch_marker() {
    let html = rwire::router::Link::touch("/", "Old page").to_static_html();
    assert!(html.contains("data-touch"), "{html}");
    assert!(html.contains("data-route"), "{html}");
    assert!(!rwire::router::Link::to("/", "x")
        .to_static_html()
        .contains("data-touch"));
    let css = rwire::theme::generate_theme_css(&rwire::theme::Theme::new());
    let block = css.split_once("@media (pointer:coarse)").unwrap().1;
    assert!(
        block.contains(":where(a[data-touch]){display:inline-flex;align-items:center}"),
        "{block}"
    );
}

/// The brand mark: a 26 px square with a 7 px radius; the logout arrow points
/// into a door on the right (the previous front's).
#[test]
fn brand_square_and_logout_icon() {
    assert_eq!(St::Sq26.css(), "width:26px;height:26px;flex-shrink:0");
    assert_eq!(St::Rounded7.css(), "border-radius:7px");
    assert_eq!(
        rwire::Icon::LogOut.svg_path(),
        "M15 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3M10 17l5-5-5-5M15 12H4"
    );
}
