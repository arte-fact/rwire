//! Retained-mode scene graph for server-side sprite management.
//!
//! The server maintains a `Scene` of sprites. Each connection gets a `ClientView`
//! tracking what the client currently has. The diff engine computes minimal opcodes
//! to sync client state.

use std::collections::{HashMap, HashSet};

/// Sprite flags.
pub const FLAG_FLIP_X: u8 = 0x01;
pub const FLAG_VISIBLE: u8 = 0x02;

/// Layer flags.
pub const LAYER_CACHEABLE: u8 = 0x01;
pub const LAYER_VISIBLE: u8 = 0x02;
pub const LAYER_WORLD_SPACE: u8 = 0x04;

/// Per-sprite state tracked by the server.
#[derive(Clone, Debug)]
pub struct SpriteState {
    pub id: u16,
    pub layer: u8,
    pub sprite_id: u16,
    pub x: i16,
    pub y: i16,
    pub flags: u8,
    pub alpha: u8,
}

/// Layer definition.
#[derive(Clone, Debug)]
pub struct LayerDef {
    pub id: u8,
    pub flags: u8,
}

/// What the client currently has — maintained per connection.
pub struct ClientView {
    pub sprites: HashMap<u16, SpriteState>,
    pub minimap_sent: bool,
    pub layers_sent: HashSet<u8>,
    pub tile_chunks_sent: HashMap<u8, HashSet<(u16, u16)>>,
    pub last_camera: (i16, i16, u16),
}

impl ClientView {
    pub fn new() -> Self {
        Self {
            sprites: HashMap::new(),
            minimap_sent: false,
            layers_sent: HashSet::new(),
            tile_chunks_sent: HashMap::new(),
            last_camera: (0, 0, 256),
        }
    }

    /// Snapshot the current scene into the client view after a successful send.
    pub fn sync_from(&mut self, scene: &Scene) {
        self.sprites = scene.sprites.clone();
    }
}

impl Default for ClientView {
    fn default() -> Self {
        Self::new()
    }
}

/// Authoritative scene on the server. One per connection.
pub struct Scene {
    pub sprites: HashMap<u16, SpriteState>,
    next_id: u16,
    pub layers: Vec<LayerDef>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            sprites: HashMap::new(),
            next_id: 0,
            layers: Vec::new(),
        }
    }

    /// Add a layer definition.
    pub fn add_layer(&mut self, id: u8, flags: u8) {
        self.layers.push(LayerDef { id, flags });
    }

    /// Create a sprite and return its ID.
    pub fn create(
        &mut self,
        layer: u8,
        sprite_id: u16,
        x: i16,
        y: i16,
        flags: u8,
    ) -> u16 {
        let id = self.next_id;
        self.next_id += 1;
        self.sprites.insert(id, SpriteState {
            id,
            layer,
            sprite_id,
            x,
            y,
            flags,
            alpha: 255,
        });
        id
    }

    /// Create a sprite with a specific ID (for stable IDs mapped from game entities).
    pub fn create_with_id(
        &mut self,
        id: u16,
        layer: u8,
        sprite_id: u16,
        x: i16,
        y: i16,
        flags: u8,
    ) {
        self.sprites.insert(id, SpriteState {
            id,
            layer,
            sprite_id,
            x,
            y,
            flags,
            alpha: 255,
        });
        if id >= self.next_id {
            self.next_id = id + 1;
        }
    }

    /// Update sprite position, sprite_id, and flags.
    pub fn update(&mut self, id: u16, x: i16, y: i16, sprite_id: u16, flags: u8) {
        if let Some(s) = self.sprites.get_mut(&id) {
            s.x = x;
            s.y = y;
            s.sprite_id = sprite_id;
            s.flags = flags;
        }
    }

    /// Move sprite position only.
    pub fn move_sprite(&mut self, id: u16, x: i16, y: i16) {
        if let Some(s) = self.sprites.get_mut(&id) {
            s.x = x;
            s.y = y;
        }
    }

    /// Delete a sprite.
    pub fn delete(&mut self, id: u16) {
        self.sprites.remove(&id);
    }

    /// Set sprite alpha.
    pub fn set_alpha(&mut self, id: u16, alpha: u8) {
        if let Some(s) = self.sprites.get_mut(&id) {
            s.alpha = alpha;
        }
    }

    /// Get a sprite by ID.
    pub fn get(&self, id: u16) -> Option<&SpriteState> {
        self.sprites.get(&id)
    }

    /// Clear all sprites (e.g., on reconnect).
    pub fn clear(&mut self) {
        self.sprites.clear();
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
