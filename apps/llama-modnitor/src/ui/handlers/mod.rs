//! Event handlers for the dashboard UI: node lifecycle, confirmation flow, and
//! device-budget recompute. Config inputs, flag rows, and the chat/bench/logs
//! session controls live in the [`config`], [`flags`], and [`session`]
//! submodules and are re-exported here.

// Rust guideline compliant 2026-02-21

use rwire::{EventContext, handler};

use crate::launcher;
use crate::snapshot::{
    App, EngineKind, FlagEntry, HardwareSnapshot, LlmNode, MaterialClass, NodePhase, NodeView,
};

mod config;
mod flags;
mod session;
pub use config::*;
pub use flags::*;
pub use session::*;

#[handler]
pub fn toggle_select(app: &mut App, ctx: &EventContext) {
    if let Some(key) = ctx.data("key") {
        app.selection.toggle(key);
    }
}

/// Create a node from the current selection, then clear the selection.
#[handler]
pub fn add_node(app: &mut App) {
    if let Some(class) = app.selection.locked_class() {
        let id = app.next_node_id;
        app.next_node_id += 1;
        let devices = app.selection.keys.clone();
        let device_vram_mb = device_capacity_mb(&app.hw, class, &devices);
        let device_used_mb = device_used_mb(&app.hw, class, &devices);
        app.nodes.push(LlmNode {
            id,
            material: class,
            devices,
            engine: None,
            model: String::new(),
            ctx: String::new(),
            port_cfg: String::new(),
            autofit: false,
            vram_target: 90,
            device_vram_mb,
            device_used_mb,
            // Start with one empty row: it is the always-present "new row".
            flags: vec![FlagEntry::default()],
            profile_name: String::new(),
            phase: NodePhase::Configuring,
            port: 0,
            view: NodeView::default(),
            log_tail: String::new(),
            chat_input: String::new(),
            chat_log: Vec::new(),
            bench_results: Vec::new(),
            bench_running: false,
            pending: None,
        });
        app.selection.keys.clear();
        refresh_budgets(app);
    }
}

/// Total VRAM/RAM capacity (MiB) of the selected devices: summed GPU VRAM, or
/// system RAM for a CPU node.
pub fn device_capacity_mb(hw: &HardwareSnapshot, material: MaterialClass, keys: &[String]) -> u64 {
    if material == MaterialClass::Cpu {
        return hw.mem.total_kib / 1024;
    }
    hw.gpus
        .iter()
        .filter(|g| keys.iter().any(|k| *k == g.key()))
        .map(|g| g.metrics.vram_total)
        .sum()
}

/// Currently-used VRAM/RAM (MiB) on the selected devices.
pub fn device_used_mb(hw: &HardwareSnapshot, material: MaterialClass, keys: &[String]) -> u64 {
    if material == MaterialClass::Cpu {
        return hw.mem.used_kib / 1024;
    }
    hw.gpus
        .iter()
        .filter(|g| keys.iter().any(|k| *k == g.key()))
        .map(|g| g.metrics.vram_used)
        .sum()
}

/// Refresh every node's device capacity + used-VRAM snapshot from live hardware.
/// Called after node actions (the moments the device set or its load changes),
/// avoiding a per-poll re-render of the node column.
pub fn refresh_budgets(app: &mut App) {
    let hw = &app.hw;
    for node in &mut app.nodes {
        node.device_vram_mb = device_capacity_mb(hw, node.material, &node.devices);
        node.device_used_mb = device_used_mb(hw, node.material, &node.devices);
    }
}

/// Duplicate a node: clone its configuration into a fresh `Configuring` node.
#[handler]
pub fn duplicate_node(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let Some(src) = app.nodes.get(idx) else {
        return;
    };
    let id = app.next_node_id;
    app.next_node_id += 1;
    let mut copy = src.clone();
    copy.id = id;
    copy.phase = NodePhase::Configuring;
    copy.port = 0;
    copy.profile_name = String::new();
    app.nodes.push(copy);
    refresh_budgets(app);
}

/// Delete a node, first stopping its engine process so it can't leak (a deleted
/// running node would otherwise keep serving and holding VRAM).
#[handler]
pub fn delete_node(app: &mut App, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index()
        && idx < app.nodes.len()
    {
        let removed = app.nodes.remove(idx);
        if matches!(removed.phase, NodePhase::Running | NodePhase::Starting) {
            launcher::request_stop(removed.id);
        }
        app.node_stats.remove(&removed.id);
        refresh_budgets(app);
    }
}

/// Kill an orphan engine process (`data-pid`) that no node manages.
#[handler]
pub fn kill_orphan(app: &mut App, ctx: &EventContext) {
    let _ = app;
    if let Some(pid) = ctx.data("pid").and_then(|s| s.parse::<u32>().ok()) {
        launcher::request_kill(pid);
    }
}

/// Mark an action (`data-action`) as awaiting confirmation.
#[handler]
pub fn ask_confirm(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let Some(action) = ctx.data("action") else {
        return;
    };
    if let Some(node) = app.nodes.get_mut(idx) {
        node.pending = Some(action.to_string());
    }
}

/// Dismiss a pending confirmation.
#[handler]
pub fn cancel_confirm(app: &mut App, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index()
        && let Some(node) = app.nodes.get_mut(idx)
    {
        node.pending = None;
    }
}

/// Execute the pending confirmed action (stop / configure).
#[handler]
pub fn confirm_action(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let action = app.nodes.get_mut(idx).and_then(|n| n.pending.take());
    match action.as_deref() {
        Some("stop") => do_stop(app, idx),
        Some("configure") => do_edit(app, idx),
        _ => {}
    }
    refresh_budgets(app);
}

/// Stop a node's process and mark it Stopped.
pub fn do_stop(app: &mut App, idx: usize) {
    if let Some(node) = app.nodes.get_mut(idx) {
        let id = node.id;
        node.phase = NodePhase::Stopped;
        launcher::request_stop(id);
    }
}

/// Return a node to the configuration form, stopping it first if live.
pub fn do_edit(app: &mut App, idx: usize) {
    if let Some(node) = app.nodes.get_mut(idx) {
        let id = node.id;
        let was_live = matches!(node.phase, NodePhase::Running | NodePhase::Starting);
        node.phase = NodePhase::Configuring;
        if was_live {
            launcher::request_stop_silent(id);
        }
    }
}
