//! Configuration-form handlers: device/engine/model/context/port inputs, VRAM
//! target, autofit, profile save/load, and node launch.

// Rust guideline compliant 2026-02-21

use super::{
    App, EngineKind, FlagEntry, MaterialClass, NodePhase, handler, launcher, refresh_budgets,
};

use crate::launcher::LaunchSpec;
use crate::profiles::{self, NodeProfile};
use crate::ui::{effective_ctx, node_model, resolved_port};

/// Toggle a device (`data-dev`) in a node's device set. Picking a device from a
/// different class switches the node's class and resets the engine/model/flags
/// (engine compatibility is class-specific); otherwise it toggles, keeping ≥1.
#[handler]
pub fn toggle_device(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let Some(key) = ctx.data("dev") else { return };
    let class = crate::snapshot::class_of_key(key);
    if let Some(node) = app.nodes.get_mut(idx) {
        if class != node.material {
            // Class switch: start a fresh device set + clear engine-specific config.
            node.material = class;
            node.devices = vec![key.to_string()];
            node.engine = None;
            node.model.clear();
            node.flags = vec![FlagEntry::default()];
        } else if class == MaterialClass::Cpu {
            node.devices = vec![key.to_string()];
        } else if let Some(pos) = node.devices.iter().position(|k| k == key) {
            if node.devices.len() > 1 {
                node.devices.remove(pos);
            }
        } else {
            node.devices.push(key.to_string());
        }
    }
    refresh_budgets(app);
}

/// Set a node's engine from its item reference + `data-engine`.
#[handler]
pub fn set_engine(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else {
        return;
    };
    let engine = ctx.data("engine").and_then(EngineKind::from_tag);
    if let (Some(node), Some(engine)) = (app.nodes.get_mut(idx), engine) {
        // Switching engines invalidates the model (format differs) and the flag
        // rows (the catalog is per-engine), so reset them — but keep one empty
        // "new row" so the + Add affordance is always present.
        if node.engine != Some(engine) {
            node.engine = Some(engine);
            node.model.clear();
            node.flags = vec![FlagEntry::default()];
        }
    }
}

/// Update a node's transient "save as" profile-name field.
#[handler]
pub fn update_profile_name(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let Some(text) = ctx.text() else { return };
    if let Some(node) = app.nodes.get_mut(idx) {
        node.profile_name = text.to_string();
    }
}

/// Save a node's configuration as a named on-disk profile, then refresh the
/// saved-profile list. Ignores a blank name.
#[handler]
pub fn save_node(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let Some(node) = app.nodes.get(idx) else {
        return;
    };
    let name = node.profile_name.trim();
    if name.is_empty() {
        return;
    }
    let profile = NodeProfile::from_node(name, node);
    if profiles::save(&profile).is_ok() {
        app.profiles = profiles::list();
    }
}

/// Load a saved profile (by name in the event text) into a fresh node.
#[handler]
pub fn load_profile(app: &mut App, ctx: &EventContext) {
    let Some(name) = ctx.text() else { return };
    if name.is_empty() {
        return;
    }
    if let Some(profile) = profiles::load(name) {
        let id = app.next_node_id;
        app.next_node_id += 1;
        app.nodes.push(profile.to_node(id));
        refresh_budgets(app);
    }
}

/// Update a node's model field from its item reference + input text.
#[handler]
pub fn update_model(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let Some(text) = ctx.text() else { return };
    if let Some(node) = app.nodes.get_mut(idx) {
        node.model = text.to_string();
    }
}

/// Update a node's context-length field from its item reference + input text.
#[handler]
pub fn update_ctx(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let Some(text) = ctx.text() else { return };
    if let Some(node) = app.nodes.get_mut(idx) {
        node.ctx = text.to_string();
    }
}

/// Update a node's configured-port field from its item reference + input text.
#[handler]
pub fn update_port(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let Some(text) = ctx.text() else { return };
    if let Some(node) = app.nodes.get_mut(idx) {
        node.port_cfg = text.to_string();
    }
}

/// Update a node's VRAM target percent from the slider (clamped 10–100).
#[handler]
pub fn update_vram_target(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let Some(text) = ctx.text() else { return };
    if let (Some(node), Ok(v)) = (app.nodes.get_mut(idx), text.trim().parse::<u8>()) {
        node.vram_target = v.clamp(10, 100);
    }
}

/// Toggle a node's autofit mode (context follows the VRAM target when on).
#[handler]
pub fn toggle_autofit(app: &mut App, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index()
        && let Some(node) = app.nodes.get_mut(idx)
    {
        node.autofit = !node.autofit;
    }
}

/// Launch (or relaunch) a node: allocate a port, mark it Starting, and ask the
/// supervisor to spawn the engine process.
#[handler]
pub fn launch_node(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    // When autofit is on, resolve the fitted context first (needs model meta,
    // an immutable borrow) before taking the mutable node borrow.
    let fitted = app.nodes.get(idx).filter(|n| n.autofit).map(|n| {
        let meta = node_model(n, &app.models).and_then(|m| m.meta.as_ref());
        effective_ctx(n, meta)
    });
    let Some(node) = app.nodes.get_mut(idx) else {
        return;
    };
    if node.engine.is_none() || node.model.is_empty() {
        return;
    }
    if let Some(f) = fitted {
        node.ctx = if f > 0 { f.to_string() } else { String::new() };
    }
    node.port = resolved_port(node);
    node.phase = NodePhase::Starting;
    if let Some(spec) = LaunchSpec::from_node(node) {
        launcher::request_launch(spec);
    }
    refresh_budgets(app);
}
