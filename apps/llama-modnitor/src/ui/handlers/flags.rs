//! Dynamic flag-row handlers and the composite item-ref packing that carries the
//! (node, row) location through change/input events.

// Rust guideline compliant 2026-02-21

use super::{App, EventContext, FlagEntry, LlmNode, handler};

use crate::ui::CUSTOM;

/// Stride for packing a `(node_index, row_index)` pair into the single varint
/// an [`rwire::ItemRef`] carries. Change/input events (unlike clicks) don't send
/// the element dataset, so the row index must ride in the `on_ref` param bytes —
/// which `EventContext::item_index` decodes — alongside the node index.
pub const FLAG_STRIDE: usize = 4096;

/// Build the composite `ItemRef` for a flag row's change/input handlers.
pub fn flag_ref(node_index: usize, row_index: usize) -> rwire::ItemRef<LlmNode> {
    rwire::ItemRef::new(node_index * FLAG_STRIDE + row_index)
}

/// Split a packed composite index into `(node_index, row_index)`.
pub const fn unpack_flag(packed: usize) -> (usize, usize) {
    (packed / FLAG_STRIDE, packed % FLAG_STRIDE)
}

/// Decode `(node_index, row_index)` from a composite handler param.
pub fn flag_loc(ctx: &EventContext) -> Option<(usize, usize)> {
    Some(unpack_flag(ctx.item_index()?))
}

/// Append a blank flag row to a node.
#[handler]
pub fn add_flag(app: &mut App, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index()
        && let Some(node) = app.nodes.get_mut(idx)
    {
        node.flags.push(FlagEntry::default());
    }
}

/// Remove a flag row (node + row index packed in the handler param).
#[handler]
pub fn remove_flag(app: &mut App, ctx: &EventContext) {
    let Some((idx, ri)) = flag_loc(ctx) else {
        return;
    };
    if let Some(node) = app.nodes.get_mut(idx)
        && ri < node.flags.len()
    {
        node.flags.remove(ri);
    }
}

/// Set a flag row's flag from the catalog dropdown. "Custom…" switches the row
/// to free-text mode; any concrete choice resets the value (its options change).
#[handler]
pub fn set_flag(app: &mut App, ctx: &EventContext) {
    let Some((idx, ri)) = flag_loc(ctx) else {
        return;
    };
    let Some(choice) = ctx.text() else { return };
    if let Some(node) = app.nodes.get_mut(idx)
        && let Some(entry) = node.flags.get_mut(ri)
    {
        entry.value.clear();
        entry.raw_value = false;
        if choice == CUSTOM {
            entry.custom_flag = true;
            entry.flag.clear();
        } else {
            entry.custom_flag = false;
            entry.flag = choice.to_string();
        }
    }
}

/// Set a flag row's flag from the custom text input.
#[handler]
pub fn set_flag_text(app: &mut App, ctx: &EventContext) {
    let Some((idx, ri)) = flag_loc(ctx) else {
        return;
    };
    let Some(text) = ctx.text() else { return };
    if let Some(node) = app.nodes.get_mut(idx)
        && let Some(entry) = node.flags.get_mut(ri)
    {
        entry.flag = text.to_string();
    }
}

/// Set a flag row's value from the value dropdown. "Custom…" switches the value
/// cell to free-text mode.
#[handler]
pub fn set_value(app: &mut App, ctx: &EventContext) {
    let Some((idx, ri)) = flag_loc(ctx) else {
        return;
    };
    let Some(choice) = ctx.text() else { return };
    if let Some(node) = app.nodes.get_mut(idx)
        && let Some(entry) = node.flags.get_mut(ri)
    {
        if choice == CUSTOM {
            entry.raw_value = true;
            entry.value.clear();
        } else {
            entry.raw_value = false;
            entry.value = choice.to_string();
        }
    }
}

/// Set a flag row's value from a free-text input.
#[handler]
pub fn set_value_text(app: &mut App, ctx: &EventContext) {
    let Some((idx, ri)) = flag_loc(ctx) else {
        return;
    };
    let Some(text) = ctx.text() else { return };
    if let Some(node) = app.nodes.get_mut(idx)
        && let Some(entry) = node.flags.get_mut(ri)
    {
        entry.value = text.to_string();
    }
}

/// Shared node-card shell: a title with an optional temperature badge, then the
/// supplied body rows. Keeps the CPU and GPU cards visually consistent.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_ref_round_trips_node_and_row() {
        // The packed ItemRef must decode back to the same (node, row) so that
        // change/input events (which carry the param, not the dataset) edit the
        // right flag row.
        for (node, row) in [(0, 0), (0, 5), (3, 0), (3, 7), (12, 4095)] {
            let packed = flag_ref(node, row).index();
            assert_eq!(unpack_flag(packed), (node, row));
        }
    }
}
