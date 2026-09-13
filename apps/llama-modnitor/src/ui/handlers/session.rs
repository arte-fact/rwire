//! Running-node session handlers: the Logs/Chat/Bench toggles, test-chat input
//! and send/clear, log refresh, and benchmark runs.

// Rust guideline compliant 2026-02-21

use super::{App, handler, launcher};

use crate::snapshot::ChatTurn;
use crate::ui::short_path;

/// Toggle the server-log block; pulls fresh logs when opening it.
#[handler]
pub fn toggle_logs(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    if let Some(node) = app.nodes.get_mut(idx) {
        node.view.logs = !node.view.logs;
        if node.view.logs {
            node.log_tail = launcher::log_tail(node.id, 8192);
        }
    }
}

/// Toggle the test-chat block.
#[handler]
pub fn toggle_chat(app: &mut App, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index()
        && let Some(node) = app.nodes.get_mut(idx)
    {
        node.view.chat = !node.view.chat;
    }
}

/// Reload a node's server-log tail.
#[handler]
pub fn refresh_logs(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    if let Some(node) = app.nodes.get_mut(idx) {
        node.log_tail = launcher::log_tail(node.id, 8192);
    }
}

/// Update the test-chat prompt input.
#[handler]
pub fn update_chat_input(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let Some(text) = ctx.text() else { return };
    if let Some(node) = app.nodes.get_mut(idx) {
        node.chat_input = text.to_string();
    }
}

/// Append the prompt to the conversation and send the full history; the reply
/// (replacing the pending assistant turn) arrives asynchronously.
#[handler]
pub fn send_chat(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    if let Some(node) = app.nodes.get_mut(idx) {
        let prompt = node.chat_input.trim().to_string();
        if prompt.is_empty() || node.port == 0 {
            return;
        }
        node.chat_log.push(ChatTurn {
            role: "user".into(),
            content: prompt,
        });
        node.chat_input.clear();
        // The history to send is the real turns so far (before the placeholder).
        let messages: Vec<(String, String)> = node
            .chat_log
            .iter()
            .map(|t| (t.role.clone(), t.content.clone()))
            .collect();
        node.chat_log.push(ChatTurn {
            role: "assistant".into(),
            content: "…".into(),
        });
        let model = short_path(&node.model).to_string();
        launcher::request_chat(node.id, node.port, model, messages);
    }
}

/// Clear a node's test-chat conversation.
#[handler]
pub fn clear_chat(app: &mut App, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index()
        && let Some(node) = app.nodes.get_mut(idx)
    {
        node.chat_log.clear();
        node.chat_input.clear();
    }
}

/// Toggle the benchmark block.
#[handler]
pub fn toggle_bench(app: &mut App, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index()
        && let Some(node) = app.nodes.get_mut(idx)
    {
        node.view.bench = !node.view.bench;
    }
}

/// Start a pp/tg benchmark run; results stream in asynchronously.
#[handler]
pub fn run_bench(app: &mut App, ctx: &EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    if let Some(node) = app.nodes.get_mut(idx) {
        if node.bench_running || node.port == 0 {
            return;
        }
        node.bench_running = true;
        node.bench_results.clear();
        launcher::request_bench(node.id, node.port);
    }
}
