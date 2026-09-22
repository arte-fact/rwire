//! A pannable, zoomable SVG map for rwire.
//!
//! The server renders an `<svg viewBox="…">` of whatever it likes (regions,
//! roads, a chart); the runtime extension shipped here lets the viewer drag
//! it with one pointer, pinch it with two and zoom it with the wheel,
//! writing the `viewBox` client-side. Once a gesture is over, the extension
//! copies the view into the value of an `<input data-map-view>` beside the
//! svg and dispatches `input`, so a handler bound with [`view_input`] learns
//! where the viewer looks and the next render agrees with the screen.
//!
//! ```ignore
//! use rwire_map::{parse_view, view_input, zoomed, MapExt, ServeMap, View};
//!
//! #[handler]
//! fn set_view(me: &mut Me, ctx: &EventContext) {
//!     me.view = ctx.text().and_then(parse_view);
//! }
//!
//! fn map(me: &Me) -> ElementBuilder {
//!     el(El::Div).append([
//!         el(El::Svg).map(EXTENT, me.view.unwrap_or(EXTENT)).append(paths),
//!         view_input("map-view", set_view()),
//!     ])
//! }
//!
//! Server::bind(addr)?.root(root).serve_map().run().await
//! ```

use std::borrow::Cow;

use rwire::{el, At, El, ElementBuilder, Ev, HandlerSpec, ServerWithRoot, St};

/// The extension, minified: built from `runtime/src/ext/map.ts` by the
/// runtime's `npm run sync`.
pub const MAP_JS: &str = include_str!("../assets/map.min.js");
/// Where the runtime loader asks for it once a batch hints `ext("map")`.
pub const MAP_JS_PATH: &str = "/_rw/ext/map.js";

/// A viewBox: x, y, width, height, in the svg's units.
pub type View = [f64; 4];

/// The `viewBox` attribute of a view, whole units.
pub fn view_box(v: View) -> String {
    format!("{:.0} {:.0} {:.0} {:.0}", v[0], v[1], v[2], v[3])
}

/// The view an extension reported ("x y w h"), if it is one.
pub fn parse_view(s: &str) -> Option<View> {
    let parts: Vec<f64> = s
        .split_whitespace()
        .filter_map(|p| p.parse().ok())
        .collect();
    match parts[..] {
        [x, y, w, h] if w > 0.0 && h > 0.0 => Some([x, y, w, h]),
        _ => None,
    }
}

/// `view` scaled by `f` (0.5 halves it, 2 doubles it) about `about` — its
/// own centre if `None` — within `extent`: `None` once it would show the
/// whole extent, so the caller can fall back to it.
pub fn zoomed(view: View, extent: View, f: f64, about: Option<(f64, f64)>) -> Option<View> {
    let [x, y, w, h] = view;
    let (w2, h2) = (w * f, h * f);
    if w2 >= extent[2] && h2 >= extent[3] {
        return None;
    }
    let (cx, cy) = about.unwrap_or((x + w / 2.0, y + h / 2.0));
    Some([cx - w2 / 2.0, cy - h2 / 2.0, w2, h2])
}

/// The svg the extension drives.
pub trait MapExt {
    /// Make this svg a map showing `view` of `extent`: the extension is
    /// hinted, the zoom bounded to the extent, and the browser told to
    /// leave the gestures alone.
    fn map(self, extent: View, view: View) -> Self;
}

impl MapExt for ElementBuilder {
    fn map(self, extent: View, view: View) -> Self {
        self.ext("map")
            .attr("data-map", "1")
            .attr("data-map-extent", &view_box(extent))
            .at_str(At::ViewBox, &view_box(view))
            .at_str(At::PreserveAspectRatio, "xMidYMid meet")
            .st([St::TouchNone])
    }
}

/// The hidden input the extension reports through, beside the svg (a child
/// of the same parent): `handler` is bound on its `input` event and reads
/// the view with [`parse_view`] from `ctx.text()`.
pub fn view_input(id: &str, handler: HandlerSpec) -> ElementBuilder {
    el(El::Input)
        .attr("id", id)
        .attr("data-map-view", "1")
        .attr("type", "hidden")
        .on(Ev::Input, handler)
}

/// The server that ships the extension.
pub trait ServeMap {
    fn serve_map(self) -> Self;
}

impl<F> ServeMap for ServerWithRoot<F>
where
    F: Fn() -> ElementBuilder + Send + Sync + Clone + 'static,
{
    fn serve_map(self) -> Self {
        self.asset(
            MAP_JS_PATH,
            "text/javascript; charset=utf-8",
            Cow::Borrowed(MAP_JS.as_bytes()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_is_parsed_and_printed_whole() {
        assert_eq!(
            parse_view("1.4 -2.6 800 600"),
            Some([1.4, -2.6, 800.0, 600.0])
        );
        assert_eq!(parse_view("1 2 0 600"), None);
        assert_eq!(parse_view("garbage"), None);
        assert_eq!(view_box([1.4, -2.6, 800.0, 600.0]), "1 -3 800 600");
    }

    #[test]
    fn zooming_keeps_the_centre_and_stops_at_the_extent() {
        let extent = [0.0, 0.0, 1000.0, 500.0];
        assert_eq!(
            zoomed([0.0, 0.0, 400.0, 200.0], extent, 0.5, None),
            Some([100.0, 50.0, 200.0, 100.0])
        );
        assert_eq!(
            zoomed([0.0, 0.0, 400.0, 200.0], extent, 0.5, Some((10.0, 10.0))),
            Some([-90.0, -40.0, 200.0, 100.0])
        );
        assert_eq!(zoomed([0.0, 0.0, 600.0, 300.0], extent, 2.0, None), None);
    }

    #[test]
    fn the_extension_is_hinted_on_the_svg() {
        let svg = el(El::Svg).map([0.0, 0.0, 10.0, 10.0], [2.0, 2.0, 4.0, 4.0]);
        let html = svg.to_static_html();
        assert!(html.contains("data-map-extent=\"0 0 10 10\""), "{html}");
        assert!(html.contains("viewBox=\"2 2 4 4\""), "{html}");
    }
}
