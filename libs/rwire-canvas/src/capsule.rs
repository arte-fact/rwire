//! Canvas capsule HTML generator.
//!
//! Produces a minimal HTML page with a `<canvas>` element and the
//! canvas runtime JS. Analogous to rwire's `generate_styled_capsule()`.

use crate::runtime::CANVAS_RUNTIME_JS;

/// Generate the canvas game capsule HTML.
///
/// The HTML includes:
/// - A full-viewport `<canvas>` element
/// - The canvas runtime JS (opcode dispatcher, input, WebSocket)
/// - Dark background styling and pixel-art rendering
pub fn generate_canvas_capsule(width: u16, height: u16, title: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,user-scalable=no">
<title>{title}</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
html,body{{width:100%;height:100%;overflow:hidden;background:#1a1a2e}}
canvas{{display:block;width:100%;height:100%;object-fit:contain;image-rendering:pixelated;image-rendering:crisp-edges}}
</style>
</head><body>
<canvas id="game-canvas" width="{width}" height="{height}"></canvas>
<script>
{CANVAS_RUNTIME_JS}
</script>
</body></html>"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capsule_contains_canvas() {
        let html = generate_canvas_capsule(960, 640, "Test");
        assert!(html.contains("<canvas"));
        assert!(html.contains("id=\"game-canvas\""));
        assert!(html.contains("width=\"960\""));
        assert!(html.contains("height=\"640\""));
    }

    #[test]
    fn test_capsule_contains_runtime() {
        let html = generate_canvas_capsule(960, 640, "Test");
        assert!(html.contains("WebSocket"));
        assert!(html.contains("getContext"));
    }
}
