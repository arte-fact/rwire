//! The node configuration form: device/engine/model/context inputs, the dynamic
//! flag rows, the VRAM estimate panel, the effective-command preview, and the
//! pure helpers (context/port resolution, engine params) shared with the
//! dashboard and handlers.

// Rust guideline compliant 2026-02-21

use super::handlers::{
    add_flag, flag_ref, launch_node, remove_flag, save_node, set_engine, set_flag, set_flag_text,
    set_value, set_value_text, toggle_autofit, toggle_device, update_ctx, update_model,
    update_port, update_profile_name, update_vram_target,
};
use super::{CUSTOM, input_box, input_box_w, text_field_w};
use rwire::{El, ElementBuilder, Ev, St, Style, el};
use rwire_components::{Button, ButtonSize, Gap, Stack, StackJustify, Text};

use crate::convert;
use crate::models::DiscoveredModel;
use crate::snapshot::{
    DeviceInfo, EngineKind, FlagEntry, LlmNode, MaterialClass, ValueSpec, engines_for,
    flag_catalog, flag_spec,
};
use crate::vram;

/// The `Configuring` body: device editor + engine picker + model + Launch.
pub fn configuring_body(
    node: &LlmNode,
    item_ref: rwire::ItemRef<LlmNode>,
    models: &[DiscoveredModel],
    devices: &[DeviceInfo],
    body: &mut Vec<ElementBuilder>,
) {
    device_engine_section(node, item_ref, devices, body);

    let Some(engine) = node.engine else {
        return;
    };

    model_context_port_section(node, item_ref, models, body);

    // VRAM/RAM estimate + target slider + autofit toggle.
    body.push(vram_panel(node, models, item_ref));

    // Dynamic flag–value rows.
    flags_section(node, engine, item_ref, body);

    // Effective command preview: the exact command the launcher will run.
    body.push(command_preview(node, models));

    // Action bar: save-as-profile on the left, Launch on the right.
    body.push(action_bar(node, item_ref));
}

/// The device-chip editor followed by the engine picker.
fn device_engine_section(
    node: &LlmNode,
    item_ref: rwire::ItemRef<LlmNode>,
    devices: &[DeviceInfo],
    body: &mut Vec<ElementBuilder>,
) {
    // Editable device set — any device may be picked; choosing one from another
    // class switches the node's class (and resets the engine/model).
    body.push(field_label("Devices"));
    let chips: Vec<ElementBuilder> = devices
        .iter()
        .map(|d| {
            let on = node.devices.iter().any(|k| k == &d.key);
            let btn = if on {
                Button::primary(d.label.clone())
            } else {
                Button::secondary(d.label.clone())
            };
            btn.size(ButtonSize::Sm).build().data("dev", &d.key).on_ref(
                Ev::Click,
                toggle_device(),
                item_ref,
            )
        })
        .collect();
    body.push(
        el(El::Div)
            .st([St::DisplayFlex, St::FlexWrap, St::GapSm])
            .append(chips),
    );

    let engine_buttons: Vec<ElementBuilder> = engines_for(node.material)
        .iter()
        .map(|&eng| {
            let btn = if node.engine == Some(eng) {
                Button::primary(eng.label())
            } else {
                Button::secondary(eng.label())
            };
            btn.size(ButtonSize::Sm)
                .build()
                .data("engine", eng.tag())
                .on_ref(Ev::Click, set_engine(), item_ref)
        })
        .collect();
    body.push(Text::label("Engine".to_string()).build());
    body.push(Stack::row().gap(Gap::Sm).children(engine_buttons).build());
}

/// The model dropdown, context-length field, and port field.
fn model_context_port_section(
    node: &LlmNode,
    item_ref: rwire::ItemRef<LlmNode>,
    models: &[DiscoveredModel],
    body: &mut Vec<ElementBuilder>,
) {
    body.push(field_label("Model"));
    let compatible: Vec<&DiscoveredModel> = models
        .iter()
        .filter(|m| node.engine.is_some_and(|e| e.accepts_format(m.format)))
        .collect();
    if compatible.is_empty() {
        body.push(
            Text::caption("No compatible models found (set MODELS_DIR).".to_string())
                .muted()
                .build(),
        );
    } else {
        let mut options = vec![el(El::Option).attr("value", "").text("Select a model…")];
        for m in &compatible {
            let mut opt = el(El::Option).attr("value", &m.path).text(&m.name);
            if m.path == node.model {
                opt = opt.attr("selected", "selected");
            }
            options.push(opt);
        }
        body.push(
            input_box_w(&format!("model-{}", node.id), El::Select, Some("34rem"))
                .append(options)
                .on_ref(Ev::Change, update_model(), item_ref),
        );
    }

    body.push(field_label("Context length (blank = engine default)"));
    if node.autofit {
        // Autofit owns the context: show the fitted value read-only.
        let meta = node_model(node, models).and_then(|m| m.meta.as_ref());
        let fitted = effective_ctx(node, meta);
        body.push(
            input_box_w(&format!("ctx-{}", node.id), El::Input, Some("12rem"))
                .attr("type", "text")
                .attr("value", &format!("{fitted} (autofit)"))
                .attr("disabled", "disabled"),
        );
    } else {
        body.push(
            text_field_w(
                &format!("ctx-{}", node.id),
                "e.g. 8192",
                &node.ctx,
                Some("12rem"),
            )
            .on_ref(Ev::Input, update_ctx(), item_ref),
        );
    }

    body.push(field_label("Port"));
    body.push(
        text_field_w(
            &format!("port-{}", node.id),
            &format!(
                "auto ({})",
                8090 + u16::try_from(node.id).unwrap_or(u16::MAX)
            ),
            &node.port_cfg,
            Some("12rem"),
        )
        .on_ref(Ev::Input, update_port(), item_ref),
    );
}

/// The bottom action bar: save-as-profile on the left, Launch on the right,
/// separated from the form by a top border.
fn action_bar(node: &LlmNode, item_ref: rwire::ItemRef<LlmNode>) -> ElementBuilder {
    let ready = !node.model.is_empty();
    let launch = Button::primary("Launch").size(ButtonSize::Sm).build();
    let launch = if ready {
        launch.on_ref(Ev::Click, launch_node(), item_ref)
    } else {
        launch.st([St::Opacity50, St::CursorNotAllowed])
    };
    el(El::Div)
        .style(
            Style::new()
                .set("border-top", "1px solid var(--g)")
                .set("padding-top", "var(--S4)"),
        )
        .append([Stack::row()
            .justify(StackJustify::Between)
            .align_center()
            .children([
                Stack::row()
                    .gap(Gap::Sm)
                    .align_center()
                    .children([
                        text_field_w(
                            &format!("pname-{}", node.id),
                            "profile name",
                            &node.profile_name,
                            Some("11rem"),
                        )
                        .on_ref(Ev::Input, update_profile_name(), item_ref),
                        Button::secondary("Save")
                            .size(ButtonSize::Sm)
                            .build()
                            .on_ref(Ev::Click, save_node(), item_ref),
                    ])
                    .build(),
                launch,
            ])
            .build()])
}

/// A consistently styled field label.
pub fn field_label(text: &str) -> ElementBuilder {
    Text::label(text.to_string()).build()
}

/// A read-only, monospace preview of the exact command the launcher will run:
/// the engine executable, the managed args, and the user's flag rows. Mirrors
/// `launcher::build_command`; kept in sync by hand (the builder owns the truth).
pub fn command_preview(node: &LlmNode, models: &[DiscoveredModel]) -> ElementBuilder {
    let cmd = effective_command(node, models);
    let block = el(El::Pre)
        .st([
            St::FontMono,
            St::RoundedSm,
            St::PSm,
            St::WhitespacePreWrap,
            St::WordBreakAll,
            St::M0,
        ])
        .style(
            Style::new()
                .background("var(--b)")
                .border("1px solid var(--g)")
                .set("color", "var(--j)")
                .set("font-size", "0.78rem"),
        )
        .text(&cmd);
    Stack::column()
        .gap(Gap::Xs)
        .children([field_label("Effective command"), block])
        .build()
}

/// The port a node will serve on: the configured value, or auto `8090 + id`.
pub fn resolved_port(node: &LlmNode) -> u16 {
    node.port_cfg
        .trim()
        .parse::<u16>()
        .ok()
        .filter(|p| *p > 0)
        .unwrap_or_else(|| 8090 + u16::try_from(node.id).unwrap_or(u16::MAX))
}

/// Build the human-readable effective command string for [`command_preview`].
pub fn effective_command(node: &LlmNode, models: &[DiscoveredModel]) -> String {
    let port = resolved_port(node);
    let mut parts: Vec<String> = Vec::new();

    // Device-mask env prefix (what the supervisor sets on the child).
    if node.material != MaterialClass::Cpu {
        let mask = node
            .device_ordinals()
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let var = if node.material == MaterialClass::Nvidia {
            "CUDA_VISIBLE_DEVICES"
        } else {
            "HIP_VISIBLE_DEVICES"
        };
        parts.push(format!("{var}={mask}"));
    }

    // Effective context: the autofit value when enabled, else the typed one.
    let meta = node_model(node, models).and_then(|m| m.meta.as_ref());
    let eff = effective_ctx(node, meta);
    let ctx = if eff > 0 {
        eff.to_string()
    } else {
        String::new()
    };
    let ctx = ctx.as_str();
    match node.engine {
        Some(EngineKind::LlamaCpp) => {
            parts.push("llama-server".into());
            parts.push(format!("-m {}", short_path(&node.model)));
            parts.push("--host 0.0.0.0".into());
            parts.push(format!("--port {port}"));
            parts.push("--metrics".into());
            parts.push(format!(
                "-ngl {}",
                if node.material == MaterialClass::Cpu {
                    0
                } else {
                    99
                }
            ));
            if !ctx.is_empty() {
                parts.push(format!("-c {ctx}"));
            }
        }
        Some(EngineKind::Flambeau) => {
            parts.push("flambeau serve".into());
            parts.push(format!("--model {}", short_path(&node.model)));
            parts.push(format!("--port {port}"));
            if !ctx.is_empty() {
                parts.push(format!("--ctx-cap {ctx}"));
            }
        }
        Some(EngineKind::Vllm) => {
            parts.push(format!("vllm serve {}", short_path(&node.model)));
            parts.push("--host 0.0.0.0".into());
            parts.push(format!("--port {port}"));
            if !ctx.is_empty() {
                parts.push(format!("--max-model-len {ctx}"));
            }
        }
        None => {}
    }

    // User flag rows (bare flag when no value), as the launcher appends them.
    for f in &node.flags {
        let flag = f.flag.trim();
        if flag.is_empty() {
            continue;
        }
        let value = f.value.trim();
        if value.is_empty() {
            parts.push(flag.to_string());
        } else {
            parts.push(format!("{flag} {value}"));
        }
    }

    // One parameter per line, shell-style with trailing backslash continuations.
    parts.join(" \\\n  ")
}

/// The KV-cache element size (bytes) implied by the node's `--cache-type-k` /
/// `--kv-cache-dtype` / `--kv` flag, defaulting to f16 (2 bytes).
pub fn node_cache_bytes(node: &LlmNode) -> f64 {
    let v = node
        .flags
        .iter()
        .find(|f| {
            matches!(
                f.flag.as_str(),
                "--cache-type-k" | "--kv-cache-dtype" | "--kv"
            )
        })
        .map_or("", |f| f.value.as_str());
    vram::cache_elt_bytes(v)
}

/// Build the engine-specific estimation parameters from a node's flags + device.
pub fn node_engine_params(node: &LlmNode) -> vram::EngineParams {
    let flag_f64 = |name: &str| {
        node.flags
            .iter()
            .find(|f| f.flag == name)
            .and_then(|f| f.value.trim().parse::<f64>().ok())
    };
    vram::EngineParams {
        engine: node.engine.unwrap_or(EngineKind::LlamaCpp),
        cache_elt_bytes: node_cache_bytes(node),
        capacity_mb: convert::u64_f64(node.device_vram_mb),
        gpu_util: flag_f64("--gpu-memory-utilization").unwrap_or(0.9),
        extra_mb: flag_f64("--prefix-cache-max-gb").unwrap_or(0.0) * 1024.0,
    }
}

/// VRAM/RAM budget for autofit: the target fraction of capacity, minus VRAM
/// already used on the selected devices.
pub fn autofit_budget_mb(node: &LlmNode) -> f64 {
    let target = convert::u64_f64(node.device_vram_mb) * f64::from(node.vram_target) / 100.0;
    (target - convert::u64_f64(node.device_used_mb)).max(0.0)
}

/// The context length the node will actually use: the auto-fitted value when
/// autofit is on (needs model metadata), otherwise the typed value.
pub fn effective_ctx(node: &LlmNode, meta: Option<&crate::modelmeta::ModelMeta>) -> u32 {
    if let (true, Some(meta)) = (node.autofit, meta) {
        return vram::fit_ctx(meta, autofit_budget_mb(node), node_engine_params(node));
    }
    node.ctx.trim().parse().unwrap_or(0)
}

/// Look up the discovered model backing a node, if any.
pub fn node_model<'a>(
    node: &LlmNode,
    models: &'a [DiscoveredModel],
) -> Option<&'a DiscoveredModel> {
    models.iter().find(|m| m.path == node.model)
}

/// Render the VRAM/RAM estimate panel: a usage bar (estimate vs selected-device
/// capacity) with a target marker, a breakdown, the target slider, and the
/// autofit toggle.
pub fn vram_panel(
    node: &LlmNode,
    models: &[DiscoveredModel],
    item_ref: rwire::ItemRef<LlmNode>,
) -> ElementBuilder {
    let unit = if node.material == MaterialClass::Cpu {
        "RAM"
    } else {
        "VRAM"
    };
    let mut rows: Vec<ElementBuilder> = vec![Text::label(format!("{unit} estimate")).build()];

    let meta = node_model(node, models).and_then(|m| m.meta.as_ref());
    let capacity = convert::u64_f64(node.device_vram_mb);
    let used = convert::u64_f64(node.device_used_mb);
    match meta {
        Some(meta) if capacity > 0.0 => {
            let params = node_engine_params(node);
            let ctx = effective_ctx(node, Some(meta));
            let usage = vram::estimate(meta, ctx, params);
            let node_total = usage.total_mb();
            let combined = used + node_total;
            let pct = (combined / capacity * 100.0).round();
            // vLLM reserves a pre-allocated pool rather than a ctx-sized KV cache.
            let mid_label = if params.engine == EngineKind::Vllm {
                "pool"
            } else {
                "KV"
            };
            rows.push(vram_bar(used, node_total, capacity, node.vram_target));
            rows.push(
                Text::caption(format!(
                    "{} / {} · {pct:.0}%   used {} · weights {} · {mid_label} {} · buf {}",
                    fmt_gib_mb(combined),
                    fmt_gib_mb(capacity),
                    fmt_gib_mb(used),
                    fmt_gib_mb(usage.weights),
                    fmt_gib_mb(usage.kv),
                    fmt_gib_mb(usage.buffer),
                ))
                .muted()
                .build(),
            );
        }
        Some(_) => rows.push(
            Text::caption("Select hardware for an estimate.".to_string())
                .muted()
                .build(),
        ),
        None => rows.push(
            Text::caption("Select a model with readable metadata.".to_string())
                .muted()
                .build(),
        ),
    }

    // Target slider (doubles as the bar's budget line and the autofit target).
    rows.push(
        Stack::row()
            .gap(Gap::Sm)
            .align_center()
            .children([
                Text::caption(format!("Target {}%", node.vram_target))
                    .muted()
                    .build(),
                input_box(&format!("vt-{}", node.id), El::Input)
                    .attr("type", "range")
                    .attr("min", "10")
                    .attr("max", "100")
                    .attr("step", "5")
                    .attr("value", &node.vram_target.to_string())
                    .st([St::Flex1])
                    .style(Style::new().max_width("18rem"))
                    .on_ref(Ev::Input, update_vram_target(), item_ref),
                {
                    let label = if node.autofit {
                        "Autofit: on"
                    } else {
                        "Autofit: off"
                    };
                    let btn = if node.autofit {
                        Button::primary(label)
                    } else {
                        Button::secondary(label)
                    };
                    btn.size(ButtonSize::Sm)
                        .build()
                        .on_ref(Ev::Click, toggle_autofit(), item_ref)
                },
            ])
            .build(),
    );

    Stack::column().gap(Gap::Xs).children(rows).build()
}

/// A stacked usage bar: a grey "already used" segment, then this node's
/// estimate (green within target / amber up to capacity / red over), plus a
/// target tick. All amounts in MiB; `target_pct` in percent of capacity.
pub fn vram_bar(used_mb: f64, node_mb: f64, capacity_mb: f64, target_pct: u8) -> ElementBuilder {
    let cap = capacity_mb.max(1.0);
    let used_pct = (used_mb / cap).clamp(0.0, 1.0);
    let node_pct = (node_mb / cap).clamp(0.0, 1.0 - used_pct);
    let target = f64::from(target_pct) / 100.0;
    let combined = used_mb + node_mb;
    let color = if combined > capacity_mb {
        "var(--O10)" // over capacity
    } else if combined > capacity_mb * target {
        "var(--A10)" // over target, within capacity
    } else {
        "var(--G10)" // within target
    };
    let used_seg = el(El::Div).st([St::PositionAbsolute]).style(
        Style::new()
            .set("left", "0")
            .set("width", &format!("{:.1}%", used_pct * 100.0))
            .set("height", "10px")
            .background("var(--g)"),
    );
    let node_seg = el(El::Div).st([St::PositionAbsolute]).style(
        Style::new()
            .set("left", &format!("{:.1}%", used_pct * 100.0))
            .set("width", &format!("{:.1}%", node_pct * 100.0))
            .set("height", "10px")
            .background(color),
    );
    let tick = el(El::Div).st([St::PositionAbsolute]).style(
        Style::new()
            .set("left", &format!("{:.1}%", target * 100.0))
            .set("top", "-2px")
            .set("width", "2px")
            .set("height", "14px")
            .background("var(--k)"),
    );
    el(El::Div)
        .st([
            St::RoundedSm,
            St::WFull,
            St::PositionRelative,
            St::OverflowHidden,
        ])
        .style(Style::new().set("height", "10px").background("var(--c)"))
        .append([used_seg, node_seg, tick])
}

/// Format a MiB amount as GiB with one decimal.
pub fn fmt_gib_mb(mb: f64) -> String {
    format!("{:.1} GiB", mb / 1024.0)
}

/// Render the dynamic flag–value rows. The last row is the always-present "new
/// row" carrying the **+ Add** button (instead of remove); the rest carry **×**.
pub fn flags_section(
    node: &LlmNode,
    engine: EngineKind,
    item_ref: rwire::ItemRef<LlmNode>,
    body: &mut Vec<ElementBuilder>,
) {
    body.push(Text::label("Flags".to_string()).build());
    let last = node.flags.len().saturating_sub(1);
    for (ri, entry) in node.flags.iter().enumerate() {
        let is_last = ri == last;
        body.push(flag_row(
            node.id,
            engine,
            ri,
            entry,
            &node.flags,
            is_last,
            item_ref,
        ));
    }
}

/// One flag row: a flag cell (catalog dropdown or custom text) stacked above a
/// value cell adapted to the chosen flag, with a remove button.
pub fn flag_row(
    node_id: u64,
    engine: EngineKind,
    ri: usize,
    entry: &FlagEntry,
    all_flags: &[FlagEntry],
    is_last: bool,
    item_ref: rwire::ItemRef<LlmNode>,
) -> ElementBuilder {
    // Pack (node index, row index) into one ItemRef so change/input events —
    // which don't carry the element dataset — can still locate the row.
    let rref = flag_ref(item_ref.index(), ri);

    // Flag cell: dropdown of the catalog (grouped, minus flags used by other
    // rows) + "Custom…"; or a text input when the row is in custom mode.
    let flag_cell = if entry.custom_flag {
        text_field_w(
            &format!("flag-{node_id}-{ri}"),
            "--custom-flag",
            &entry.flag,
            None,
        )
        .on_ref(Ev::Input, set_flag_text(), rref)
    } else {
        flag_dropdown(node_id, engine, ri, entry, all_flags, rref)
    };

    // Trailing control: the last row is the "new row" → + Add (append another
    // row); every other row → × (remove this row).
    let trailing = if is_last {
        Button::ghost("+ Add")
            .size(ButtonSize::Sm)
            .build()
            .on_ref(Ev::Click, add_flag(), item_ref)
    } else {
        Button::ghost("×")
            .size(ButtonSize::Sm)
            .build()
            .on_ref(Ev::Click, remove_flag(), rref)
    };

    // Inline row: flag, value, and trailing button on one line; wraps on narrow.
    el(El::Div)
        .st([St::DisplayFlex, St::FlexWrap, St::ItemsCenter, St::GapSm])
        .append([
            el(El::Div)
                .style(Style::new().set("flex", "1 1 17rem").min_width("0"))
                .append([flag_cell]),
            el(El::Div)
                .style(Style::new().set("flex", "1 1 12rem").min_width("0"))
                .append([value_cell(node_id, engine, ri, entry, rref)]),
            trailing,
        ])
}

/// The flag-selection `<select>`: catalog flags grouped by `FlagSpec::group`,
/// then a "Custom…" entry. A leading placeholder shows when no flag is chosen.
pub fn flag_dropdown(
    node_id: u64,
    engine: EngineKind,
    ri: usize,
    entry: &FlagEntry,
    all_flags: &[FlagEntry],
    rref: rwire::ItemRef<LlmNode>,
) -> ElementBuilder {
    // Flags already chosen in other rows are hidden so each flag is added once.
    let taken = |flag: &str| {
        all_flags
            .iter()
            .enumerate()
            .any(|(i, f)| i != ri && f.flag == flag)
    };

    let mut placeholder = el(El::Option).attr("value", "").text("Select a flag…");
    if entry.flag.is_empty() {
        placeholder = placeholder.attr("selected", "selected");
    }
    let mut options = vec![placeholder];

    // Group available flags with disabled separator options (El has no
    // <optgroup>); skip groups that end up empty.
    let mut current_group = "";
    for spec in flag_catalog(engine) {
        if taken(spec.flag) {
            continue;
        }
        if spec.group != current_group {
            current_group = spec.group;
            options.push(
                el(El::Option)
                    .attr("value", "")
                    .attr("disabled", "disabled")
                    .text(&format!("── {current_group} ──")),
            );
        }
        let mut opt = el(El::Option).attr("value", spec.flag).text(spec.label);
        if spec.flag == entry.flag {
            opt = opt.attr("selected", "selected");
        }
        options.push(opt);
    }

    options.push(el(El::Option).attr("value", CUSTOM).text("Custom…"));

    input_box_w(&format!("flag-{node_id}-{ri}"), El::Select, None)
        .append(options)
        .on_ref(Ev::Change, set_flag(), rref)
}

/// The value cell adapted to the chosen flag's [`ValueSpec`]: a dropdown of
/// suggested values (with "Custom…"), a free text input, or "(no value)".
pub fn value_cell(
    node_id: u64,
    engine: EngineKind,
    ri: usize,
    entry: &FlagEntry,
    rref: rwire::ItemRef<LlmNode>,
) -> ElementBuilder {
    let id = format!("val-{node_id}-{ri}");

    let vw = Some("22rem");
    // A custom flag (or no flag chosen) has no schema → free text.
    if entry.flag.is_empty() || entry.custom_flag {
        return text_field_w(&id, "value", &entry.value, vw).on_ref(
            Ev::Input,
            set_value_text(),
            rref,
        );
    }

    match flag_spec(engine, &entry.flag).map(|s| s.value) {
        Some(ValueSpec::None) => Text::caption("(no value)".to_string()).muted().build(),
        Some(ValueSpec::Choice(_)) if entry.raw_value => {
            text_field_w(&id, "custom value", &entry.value, vw).on_ref(
                Ev::Input,
                set_value_text(),
                rref,
            )
        }
        Some(ValueSpec::Choice(values)) => {
            let mut placeholder = el(El::Option).attr("value", "").text("Select…");
            if entry.value.is_empty() {
                placeholder = placeholder.attr("selected", "selected");
            }
            let mut opts = vec![placeholder];
            for &v in values {
                let mut opt = el(El::Option).attr("value", v).text(v);
                if v == entry.value {
                    opt = opt.attr("selected", "selected");
                }
                opts.push(opt);
            }
            opts.push(el(El::Option).attr("value", CUSTOM).text("Custom…"));
            input_box_w(&id, El::Select, vw)
                .append(opts)
                .on_ref(Ev::Change, set_value(), rref)
        }
        // Unknown flag (typed but matches nothing) → free text.
        Option::None => {
            text_field_w(&id, "value", &entry.value, vw).on_ref(Ev::Input, set_value_text(), rref)
        }
    }
}

/// Shorten a model path to its file name for compact display.
pub fn short_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
