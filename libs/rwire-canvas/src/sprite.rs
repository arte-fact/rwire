//! Sprite and texture ID types.

/// Texture identifier (references an entry in the texture table).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(pub u8);

/// Sprite identifier (references an entry in the sprite table).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpriteId(pub u16);

/// A source rectangle within a texture (for sprite sheets).
#[derive(Clone, Copy, Debug)]
pub struct SpriteRect {
    pub texture: TextureId,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

/// Definition of a sprite sheet with uniform frame layout.
#[derive(Clone, Debug)]
pub struct SpriteSheet {
    pub texture: TextureId,
    pub frame_w: u16,
    pub frame_h: u16,
    pub columns: u16,
    pub first_sprite_id: SpriteId,
    pub frame_count: u16,
}

impl SpriteSheet {
    /// Get the SpriteId for a given frame index.
    pub fn frame(&self, index: u16) -> SpriteId {
        SpriteId(self.first_sprite_id.0 + index.min(self.frame_count - 1))
    }

    /// Generate all sprite rects for this sheet (for building the sprite table).
    pub fn rects(&self) -> Vec<SpriteRect> {
        (0..self.frame_count)
            .map(|i| {
                let col = i % self.columns;
                let row = i / self.columns;
                SpriteRect {
                    texture: self.texture,
                    x: col * self.frame_w,
                    y: row * self.frame_h,
                    w: self.frame_w,
                    h: self.frame_h,
                }
            })
            .collect()
    }
}
