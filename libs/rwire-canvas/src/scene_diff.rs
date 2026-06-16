//! Scene diff engine — computes minimal opcodes to sync client from prev → current state.

use crate::protocol::encoder::CanvasBuffer;
use crate::scene::{ClientView, Scene, SpriteState};

/// Compute the minimal set of opcodes to bring `client_view` up to date with `scene`.
/// After calling this, the caller should call `client_view.sync_from(scene)`.
pub fn diff_sprites(scene: &Scene, client_view: &ClientView, buf: &mut CanvasBuffer) {
    // Deletions: in client but not in current scene
    for id in client_view.sprites.keys() {
        if !scene.sprites.contains_key(id) {
            buf.sprite_delete(*id);
        }
    }

    let mut moves: Vec<(u16, i16, i16)> = Vec::new();
    let mut full_updates: Vec<(u16, i16, i16, u16, u8)> = Vec::new();

    // Creates and updates
    for (id, sprite) in &scene.sprites {
        match client_view.sprites.get(id) {
            None => {
                // New sprite — full create
                buf.sprite_create(
                    sprite.id,
                    sprite.layer,
                    sprite.sprite_id,
                    sprite.x,
                    sprite.y,
                    sprite.flags,
                );
                if sprite.alpha != 255 {
                    buf.sprite_alpha(sprite.id, sprite.alpha);
                }
            }
            Some(prev) => {
                let pos_changed = prev.x != sprite.x || prev.y != sprite.y;
                let frame_changed = prev.sprite_id != sprite.sprite_id;
                let flags_changed = prev.flags != sprite.flags;
                let alpha_changed = prev.alpha != sprite.alpha;

                if !pos_changed && !frame_changed && !flags_changed && !alpha_changed {
                    continue; // No change
                }

                if pos_changed && !frame_changed && !flags_changed {
                    // Position-only (most common case)
                    moves.push((sprite.id, sprite.x, sprite.y));
                } else if frame_changed && !pos_changed && !flags_changed {
                    // Frame-only (animation change)
                    buf.sprite_frame(sprite.id, sprite.sprite_id);
                } else if pos_changed || frame_changed || flags_changed {
                    // Full update needed
                    full_updates.push((
                        sprite.id,
                        sprite.x,
                        sprite.y,
                        sprite.sprite_id,
                        sprite.flags,
                    ));
                }

                if alpha_changed {
                    buf.sprite_alpha(sprite.id, sprite.alpha);
                }
            }
        }
    }

    // Emit batched moves (chunks of 255)
    for chunk in moves.chunks(255) {
        buf.sprite_move_batch(chunk);
    }

    // Emit batched full updates
    for chunk in full_updates.chunks(255) {
        buf.sprite_update_batch(chunk);
    }
}

/// Diff and emit camera opcode only if changed.
pub fn diff_camera(
    cx: i16,
    cy: i16,
    zoom: u16,
    client_view: &ClientView,
    buf: &mut CanvasBuffer,
) -> bool {
    let (pcx, pcy, pz) = client_view.last_camera;
    if cx != pcx || cy != pcy || zoom != pz {
        buf.camera(cx, cy, zoom);
        true
    } else {
        false
    }
}

/// Emit layer creation opcodes for any layers not yet sent to the client.
pub fn diff_layers(scene: &Scene, client_view: &mut ClientView, buf: &mut CanvasBuffer) {
    for layer in &scene.layers {
        if !client_view.layers_sent.contains(&layer.id) {
            buf.layer_create(layer.id, layer.flags);
            client_view.layers_sent.insert(layer.id);
        }
    }
}

/// Check if a sprite changed compared to the client view.
fn _sprite_changed(current: &SpriteState, prev: &SpriteState) -> bool {
    current.x != prev.x
        || current.y != prev.y
        || current.sprite_id != prev.sprite_id
        || current.flags != prev.flags
        || current.alpha != prev.alpha
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;

    #[test]
    fn test_diff_creates_new_sprites() {
        let mut scene = Scene::new();
        scene.create(0, 10, 100, 200, 0x02);
        scene.create(0, 11, 300, 400, 0x02);

        let client = ClientView::new();
        let mut buf = CanvasBuffer::new();
        diff_sprites(&scene, &client, &mut buf);

        let bytes = buf.finish();
        // Should have two SPRITE_CREATE opcodes (0x70)
        let creates: Vec<_> = bytes.iter().enumerate()
            .filter(|(_, &b)| b == 0x70)
            .collect();
        assert_eq!(creates.len(), 2);
    }

    #[test]
    fn test_diff_deletes_removed_sprites() {
        let scene = Scene::new(); // empty scene

        let mut client = ClientView::new();
        client.sprites.insert(5, crate::scene::SpriteState {
            id: 5, layer: 0, sprite_id: 10, x: 100, y: 200, flags: 0x02, alpha: 255,
        });

        let mut buf = CanvasBuffer::new();
        diff_sprites(&scene, &client, &mut buf);

        let bytes = buf.finish();
        // Should have one SPRITE_DELETE opcode (0x71)
        assert!(bytes.iter().any(|&b| b == 0x71));
    }

    #[test]
    fn test_diff_position_only_batched() {
        let mut scene = Scene::new();
        let id = scene.create(0, 10, 150, 250, 0x02); // moved from (100,200)

        let mut client = ClientView::new();
        client.sprites.insert(id, crate::scene::SpriteState {
            id, layer: 0, sprite_id: 10, x: 100, y: 200, flags: 0x02, alpha: 255,
        });

        let mut buf = CanvasBuffer::new();
        diff_sprites(&scene, &client, &mut buf);

        let bytes = buf.finish();
        // Should have SPRITE_MOVE_BATCH (0x76), not individual SPRITE_UPDATE
        assert!(bytes.iter().any(|&b| b == 0x76));
    }

    #[test]
    fn test_diff_no_changes() {
        let mut scene = Scene::new();
        let id = scene.create(0, 10, 100, 200, 0x02);

        let mut client = ClientView::new();
        client.sprites.insert(id, crate::scene::SpriteState {
            id, layer: 0, sprite_id: 10, x: 100, y: 200, flags: 0x02, alpha: 255,
        });

        let mut buf = CanvasBuffer::new();
        diff_sprites(&scene, &client, &mut buf);

        let bytes = buf.finish();
        assert!(bytes.is_empty(), "No opcodes should be emitted for unchanged sprites");
    }
}
