//! Canvas frame encoder.
//!
//! Fluent builder API for constructing binary canvas frame messages.
//! Each method appends opcodes + arguments to an internal byte buffer.

use bytes::{BufMut, Bytes, BytesMut};

use super::opcodes::*;

/// Binary canvas frame builder.
///
/// Accumulates canvas draw opcodes into a byte buffer, then produces
/// a compact binary message via `finish()`.
pub struct CanvasBuffer {
    buf: BytesMut,
}

impl CanvasBuffer {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(2048),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: BytesMut::with_capacity(cap),
        }
    }

    /// Consume the buffer and return the final bytes.
    pub fn finish(self) -> Bytes {
        self.buf.freeze()
    }

    /// Current byte length.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    fn put_i16(&mut self, v: i16) {
        self.buf.put_i16(v);
    }

    fn put_u16(&mut self, v: u16) {
        self.buf.put_u16(v);
    }

    // ========================================================================
    // Frame control
    // ========================================================================

    /// Mark the start of a frame with a tick number (for interpolation).
    pub fn frame_begin(&mut self, tick: u32) -> &mut Self {
        self.buf.put_u8(FRAME_BEGIN);
        self.buf.put_u32(tick);
        self
    }

    /// Mark the end of a frame — triggers render on the client.
    pub fn frame_end(&mut self) -> &mut Self {
        self.buf.put_u8(FRAME_END);
        self
    }

    /// Clear the entire canvas.
    pub fn clear(&mut self) -> &mut Self {
        self.buf.put_u8(CLEAR);
        self
    }

    // ========================================================================
    // Transform stack
    // ========================================================================

    pub fn save(&mut self) -> &mut Self {
        self.buf.put_u8(SAVE);
        self
    }

    pub fn restore(&mut self) -> &mut Self {
        self.buf.put_u8(RESTORE);
        self
    }

    /// Translate the canvas origin.
    pub fn translate(&mut self, x: i16, y: i16) -> &mut Self {
        self.buf.put_u8(TRANSLATE);
        self.put_i16(x);
        self.put_i16(y);
        self
    }

    /// Scale the canvas. Values are fixed-point i16: actual_scale = value / 256.
    /// E.g., 256 = 1.0x, 512 = 2.0x, 128 = 0.5x, -256 = -1.0x (flip).
    pub fn scale(&mut self, sx: i16, sy: i16) -> &mut Self {
        self.buf.put_u8(SCALE_XY);
        self.put_i16(sx);
        self.put_i16(sy);
        self
    }

    /// Scale uniformly. Convenience for `scale(s, s)`.
    pub fn scale_uniform(&mut self, s: f32) -> &mut Self {
        let v = (s * 256.0) as i16;
        self.scale(v, v)
    }

    /// Rotate the canvas. Angle in radians (encoded as u16: value / 10430.378).
    pub fn rotate(&mut self, angle_rad: f32) -> &mut Self {
        self.buf.put_u8(ROTATE);
        self.put_u16((angle_rad * 10430.378) as u16);
        self
    }

    // ========================================================================
    // Draw primitives
    // ========================================================================

    pub fn fill_rect(&mut self, x: i16, y: i16, w: u16, h: u16) -> &mut Self {
        self.buf.put_u8(FILL_RECT);
        self.put_i16(x);
        self.put_i16(y);
        self.put_u16(w);
        self.put_u16(h);
        self
    }

    pub fn stroke_rect(&mut self, x: i16, y: i16, w: u16, h: u16) -> &mut Self {
        self.buf.put_u8(STROKE_RECT);
        self.put_i16(x);
        self.put_i16(y);
        self.put_u16(w);
        self.put_u16(h);
        self
    }

    pub fn begin_path(&mut self) -> &mut Self {
        self.buf.put_u8(BEGIN_PATH);
        self
    }

    pub fn move_to(&mut self, x: i16, y: i16) -> &mut Self {
        self.buf.put_u8(MOVE_TO);
        self.put_i16(x);
        self.put_i16(y);
        self
    }

    pub fn line_to(&mut self, x: i16, y: i16) -> &mut Self {
        self.buf.put_u8(LINE_TO);
        self.put_i16(x);
        self.put_i16(y);
        self
    }

    /// Draw an arc. Angles encoded as u16 (radians × 10430.378).
    pub fn arc(&mut self, cx: i16, cy: i16, radius: u16, start_rad: f32, end_rad: f32) -> &mut Self {
        self.buf.put_u8(ARC);
        self.put_i16(cx);
        self.put_i16(cy);
        self.put_u16(radius);
        self.put_u16((start_rad * 10430.378) as u16);
        self.put_u16((end_rad * 10430.378) as u16);
        self
    }

    /// Full circle arc (0 to 2π).
    pub fn arc_full(&mut self, cx: i16, cy: i16, radius: u16) -> &mut Self {
        self.arc(cx, cy, radius, 0.0, std::f32::consts::TAU)
    }

    pub fn fill(&mut self) -> &mut Self {
        self.buf.put_u8(FILL);
        self
    }

    pub fn stroke(&mut self) -> &mut Self {
        self.buf.put_u8(STROKE);
        self
    }

    pub fn close_path(&mut self) -> &mut Self {
        self.buf.put_u8(CLOSE_PATH);
        self
    }

    /// Rounded rectangle with fill.
    pub fn round_rect(&mut self, x: i16, y: i16, w: u16, h: u16, radius: u8) -> &mut Self {
        self.buf.put_u8(ROUND_RECT);
        self.put_i16(x);
        self.put_i16(y);
        self.put_u16(w);
        self.put_u16(h);
        self.buf.put_u8(radius);
        self
    }

    // ========================================================================
    // Image / sprite drawing
    // ========================================================================

    /// Draw a region of a texture to a destination rectangle.
    /// Parameters map directly to the Canvas 2D API's 9-argument drawImage().
    #[allow(clippy::too_many_arguments)]
    pub fn draw_image(
        &mut self,
        texture: u8,
        sx: u16, sy: u16, sw: u16, sh: u16,
        dx: i16, dy: i16, dw: u16, dh: u16,
    ) -> &mut Self {
        self.buf.put_u8(DRAW_IMAGE);
        self.buf.put_u8(texture);
        self.put_u16(sx);
        self.put_u16(sy);
        self.put_u16(sw);
        self.put_u16(sh);
        self.put_i16(dx);
        self.put_i16(dy);
        self.put_u16(dw);
        self.put_u16(dh);
        self
    }

    /// Draw a full texture at a destination position and size.
    pub fn draw_image_simple(
        &mut self,
        texture: u8,
        dx: i16, dy: i16, dw: u16, dh: u16,
    ) -> &mut Self {
        self.buf.put_u8(DRAW_IMAGE_SIMPLE);
        self.buf.put_u8(texture);
        self.put_i16(dx);
        self.put_i16(dy);
        self.put_u16(dw);
        self.put_u16(dh);
        self
    }

    /// Draw a pre-defined sprite (from sprite table) at a position.
    pub fn draw_sprite(&mut self, sprite_id: u16, dx: i16, dy: i16) -> &mut Self {
        self.buf.put_u8(DRAW_SPRITE);
        self.put_u16(sprite_id);
        self.put_i16(dx);
        self.put_i16(dy);
        self
    }

    // ========================================================================
    // Text
    // ========================================================================

    pub fn fill_text(&mut self, x: i16, y: i16, text: &str) -> &mut Self {
        let bytes = text.as_bytes();
        let len = bytes.len().min(255) as u8;
        self.buf.put_u8(FILL_TEXT);
        self.put_i16(x);
        self.put_i16(y);
        self.buf.put_u8(len);
        self.buf.put_slice(&bytes[..len as usize]);
        self
    }

    pub fn stroke_text(&mut self, x: i16, y: i16, text: &str) -> &mut Self {
        let bytes = text.as_bytes();
        let len = bytes.len().min(255) as u8;
        self.buf.put_u8(STROKE_TEXT);
        self.put_i16(x);
        self.put_i16(y);
        self.buf.put_u8(len);
        self.buf.put_slice(&bytes[..len as usize]);
        self
    }

    // ========================================================================
    // Style state
    // ========================================================================

    pub fn set_fill_rgba(&mut self, r: u8, g: u8, b: u8, a: u8) -> &mut Self {
        self.buf.put_u8(SET_FILL_RGBA);
        self.buf.put_u8(r);
        self.buf.put_u8(g);
        self.buf.put_u8(b);
        self.buf.put_u8(a);
        self
    }

    pub fn set_fill_rgb(&mut self, r: u8, g: u8, b: u8) -> &mut Self {
        self.set_fill_rgba(r, g, b, 255)
    }

    pub fn set_fill_idx(&mut self, color_idx: u8) -> &mut Self {
        self.buf.put_u8(SET_FILL_IDX);
        self.buf.put_u8(color_idx);
        self
    }

    pub fn set_stroke_rgba(&mut self, r: u8, g: u8, b: u8, a: u8) -> &mut Self {
        self.buf.put_u8(SET_STROKE_RGBA);
        self.buf.put_u8(r);
        self.buf.put_u8(g);
        self.buf.put_u8(b);
        self.buf.put_u8(a);
        self
    }

    pub fn set_stroke_rgb(&mut self, r: u8, g: u8, b: u8) -> &mut Self {
        self.set_stroke_rgba(r, g, b, 255)
    }

    pub fn set_stroke_idx(&mut self, color_idx: u8) -> &mut Self {
        self.buf.put_u8(SET_STROKE_IDX);
        self.buf.put_u8(color_idx);
        self
    }

    /// Set global alpha (0-255 mapped to 0.0-1.0).
    pub fn set_alpha(&mut self, alpha: u8) -> &mut Self {
        self.buf.put_u8(SET_ALPHA);
        self.buf.put_u8(alpha);
        self
    }

    /// Set line width (quarter-pixels: value / 4).
    pub fn set_line_width(&mut self, width_quarter_px: u8) -> &mut Self {
        self.buf.put_u8(SET_LINE_WIDTH);
        self.buf.put_u8(width_quarter_px);
        self
    }

    /// Set line dash pattern. Each segment in pixels.
    pub fn set_line_dash(&mut self, segments: &[u8]) -> &mut Self {
        self.buf.put_u8(SET_LINE_DASH);
        self.buf.put_u8(segments.len() as u8);
        for &s in segments {
            self.buf.put_u8(s);
        }
        self
    }

    /// Clear line dash (solid line).
    pub fn clear_line_dash(&mut self) -> &mut Self {
        self.set_line_dash(&[])
    }

    /// Set font from the pre-defined font table.
    pub fn set_font(&mut self, font_idx: u8) -> &mut Self {
        self.buf.put_u8(SET_FONT);
        self.buf.put_u8(font_idx);
        self
    }

    /// Set text alignment: 0=left, 1=center, 2=right.
    pub fn set_text_align(&mut self, align: u8) -> &mut Self {
        self.buf.put_u8(SET_TEXT_ALIGN);
        self.buf.put_u8(align);
        self
    }

    /// Set text baseline: 0=top, 1=middle, 2=bottom, 3=alphabetic.
    pub fn set_text_baseline(&mut self, baseline: u8) -> &mut Self {
        self.buf.put_u8(SET_TEXT_BASELINE);
        self.buf.put_u8(baseline);
        self
    }

    /// Set image smoothing (anti-aliasing for scaled images).
    pub fn set_smoothing(&mut self, enabled: bool) -> &mut Self {
        self.buf.put_u8(SET_SMOOTHING);
        self.buf.put_u8(if enabled { 1 } else { 0 });
        self
    }

    /// Set composite operation: 0=source-over, 1=multiply, 2=screen.
    pub fn set_composite(&mut self, mode: u8) -> &mut Self {
        self.buf.put_u8(SET_COMPOSITE);
        self.buf.put_u8(mode);
        self
    }

    // ========================================================================
    // Metadata tables (sent once at connection start)
    // ========================================================================

    /// Define the texture table. Each entry: (id, http_path).
    /// Client loads images from these paths and stores by ID.
    pub fn texture_table(&mut self, textures: &[(u8, &str)]) -> &mut Self {
        self.buf.put_u8(TEXTURE_TABLE);
        self.buf.put_u8(textures.len() as u8);
        for &(id, path) in textures {
            self.buf.put_u8(id);
            let bytes = path.as_bytes();
            self.buf.put_u8(bytes.len() as u8);
            self.buf.put_slice(bytes);
        }
        self
    }

    /// Define the font table. Each entry: CSS font string (e.g., "bold 14px sans-serif").
    pub fn font_table(&mut self, fonts: &[&str]) -> &mut Self {
        self.buf.put_u8(FONT_TABLE);
        self.buf.put_u8(fonts.len() as u8);
        for font in fonts {
            let bytes = font.as_bytes();
            self.buf.put_u8(bytes.len() as u8);
            self.buf.put_slice(bytes);
        }
        self
    }

    /// Define the sprite table. Each entry: (texture_id, sx, sy, sw, sh).
    /// Sprites are referenced by their index (u16) in DRAW_SPRITE.
    pub fn sprite_table(&mut self, sprites: &[(u8, u16, u16, u16, u16)]) -> &mut Self {
        self.buf.put_u8(SPRITE_TABLE);
        self.put_u16(sprites.len() as u16);
        for &(tex, sx, sy, sw, sh) in sprites {
            self.buf.put_u8(tex);
            self.put_u16(sx);
            self.put_u16(sy);
            self.put_u16(sw);
            self.put_u16(sh);
        }
        self
    }

    /// Define the indexed color table. Colors referenced by u8 index.
    pub fn color_table(&mut self, colors: &[(u8, u8, u8, u8)]) -> &mut Self {
        self.buf.put_u8(COLOR_TABLE);
        self.buf.put_u8(colors.len() as u8);
        for &(r, g, b, a) in colors {
            self.buf.put_u8(r);
            self.buf.put_u8(g);
            self.buf.put_u8(b);
            self.buf.put_u8(a);
        }
        self
    }

    /// Batch update entities. Each entry: (entity_id, x, y, sprite_id, flags).
    ///
    /// Flags: bit 0 = flip_x, bit 1 = visible, bit 2 = alive.
    /// The client maintains an entity table and draws all visible entities.
    pub fn entity_batch(&mut self, entities: &[(u16, i16, i16, u16, u8)]) -> &mut Self {
        self.buf.put_u8(ENTITY_BATCH);
        self.put_u16(entities.len() as u16);
        for &(id, x, y, sprite, flags) in entities {
            self.put_u16(id);
            self.put_i16(x);
            self.put_i16(y);
            self.put_u16(sprite);
            self.buf.put_u8(flags);
        }
        self
    }
}

impl Default for CanvasBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_begin_end() {
        let mut buf = CanvasBuffer::new();
        buf.frame_begin(42).frame_end();
        let bytes = buf.finish();
        assert_eq!(bytes[0], FRAME_BEGIN);
        assert_eq!(bytes[5], FRAME_END);
        assert_eq!(bytes.len(), 6); // 1 + 4 + 1
    }

    #[test]
    fn test_fill_rect() {
        let mut buf = CanvasBuffer::new();
        buf.fill_rect(100, -50, 200, 64);
        let bytes = buf.finish();
        assert_eq!(bytes[0], FILL_RECT);
        assert_eq!(bytes.len(), 9); // 1 + 2 + 2 + 2 + 2
    }

    #[test]
    fn test_draw_image() {
        let mut buf = CanvasBuffer::new();
        buf.draw_image(1, 0, 0, 64, 64, 100, 200, 64, 64);
        let bytes = buf.finish();
        assert_eq!(bytes[0], DRAW_IMAGE);
        assert_eq!(bytes[1], 1); // texture id
        assert_eq!(bytes.len(), 18); // 1 + 1 + 4×u16_src + 2×i16_dst + 2×u16_dstsz
    }

    #[test]
    fn test_draw_sprite() {
        let mut buf = CanvasBuffer::new();
        buf.draw_sprite(42, 100, 200);
        let bytes = buf.finish();
        assert_eq!(bytes[0], DRAW_SPRITE);
        assert_eq!(bytes.len(), 7); // 1 + 2 + 2 + 2
    }

    #[test]
    fn test_set_fill_rgba() {
        let mut buf = CanvasBuffer::new();
        buf.set_fill_rgba(255, 0, 128, 200);
        let bytes = buf.finish();
        assert_eq!(bytes[0], SET_FILL_RGBA);
        assert_eq!(bytes[1], 255);
        assert_eq!(bytes[2], 0);
        assert_eq!(bytes[3], 128);
        assert_eq!(bytes[4], 200);
    }

    #[test]
    fn test_fill_text() {
        let mut buf = CanvasBuffer::new();
        buf.fill_text(10, 20, "hello");
        let bytes = buf.finish();
        assert_eq!(bytes[0], FILL_TEXT);
        assert_eq!(bytes[5], 5); // string length
        assert_eq!(&bytes[6..11], b"hello");
        assert_eq!(bytes.len(), 11); // 1 + 2 + 2 + 1 + 5
    }

    #[test]
    fn test_texture_table() {
        let mut buf = CanvasBuffer::new();
        buf.texture_table(&[(0, "sprites/units.png"), (1, "sprites/tiles.png")]);
        let bytes = buf.finish();
        assert_eq!(bytes[0], TEXTURE_TABLE);
        assert_eq!(bytes[1], 2); // count
    }

    #[test]
    fn test_entity_batch() {
        let mut buf = CanvasBuffer::new();
        buf.entity_batch(&[
            (0, 100, 200, 5, 0b110), // id=0, visible+alive
            (1, 300, 400, 8, 0b111), // id=1, flip+visible+alive
        ]);
        let bytes = buf.finish();
        assert_eq!(bytes[0], ENTITY_BATCH);
        assert_eq!(u16::from_be_bytes([bytes[1], bytes[2]]), 2); // count
        assert_eq!(bytes.len(), 3 + 2 * 9); // header + 2 × 9 bytes per entity
    }

    #[test]
    fn test_transform_stack() {
        let mut buf = CanvasBuffer::new();
        buf.save()
            .translate(-100, -200)
            .scale_uniform(2.0)
            .fill_rect(0, 0, 64, 64)
            .restore();
        let bytes = buf.finish();
        assert_eq!(bytes[0], SAVE);
        assert_eq!(bytes[1], TRANSLATE);
        assert_eq!(bytes[6], SCALE_XY);
        // scale 2.0 = 512 as i16
        assert_eq!(i16::from_be_bytes([bytes[7], bytes[8]]), 512);
    }

    #[test]
    fn test_sprite_table() {
        let mut buf = CanvasBuffer::new();
        buf.sprite_table(&[
            (0, 0, 0, 64, 64),     // sprite 0: tex 0, (0,0,64,64)
            (0, 64, 0, 64, 64),    // sprite 1: tex 0, (64,0,64,64)
            (1, 0, 0, 192, 192),   // sprite 2: tex 1, (0,0,192,192)
        ]);
        let bytes = buf.finish();
        assert_eq!(bytes[0], SPRITE_TABLE);
        assert_eq!(u16::from_be_bytes([bytes[1], bytes[2]]), 3); // count
        assert_eq!(bytes.len(), 3 + 3 * 9); // header + 3 × (1 + 4×u16)
    }

    #[test]
    fn test_complete_frame() {
        let mut buf = CanvasBuffer::new();
        buf.frame_begin(1)
            .clear()
            .set_fill_rgb(26, 26, 38) // background
            .fill_rect(0, 0, 960, 640)
            .save()
            .translate(-500, -300)
            .scale_uniform(1.5)
            // Draw some terrain
            .draw_sprite(0, 0, 0)
            .draw_sprite(1, 64, 0)
            .draw_sprite(0, 128, 0)
            // Draw units via entity batch
            .entity_batch(&[
                (0, 100, 200, 10, 0b110),
                (1, 300, 150, 12, 0b110),
            ])
            // HP bar
            .set_fill_rgba(51, 51, 51, 200)
            .fill_rect(76, 175, 48, 6)
            .set_fill_rgb(51, 204, 51)
            .fill_rect(76, 175, 36, 6)
            .restore()
            .frame_end();

        let bytes = buf.finish();
        // Just verify it's reasonable size
        assert!(bytes.len() < 200, "Frame too large: {} bytes", bytes.len());
        assert!(bytes.len() > 50, "Frame too small: {} bytes", bytes.len());
        // First byte is FRAME_BEGIN, last is FRAME_END
        assert_eq!(bytes[0], FRAME_BEGIN);
        assert_eq!(bytes[bytes.len() - 1], FRAME_END);
    }
}
