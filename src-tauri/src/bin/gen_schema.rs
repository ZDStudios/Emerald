//! Generates the three derived settings artefacts from `settings.rs`.
//!
//!   * `schema/focus-access.schema.json` — JSON Schema draft 2020-12
//!   * `src/lib/settings.gen.ts`         — TS types + defaults for the chrome
//!   * `docs/settings-schema.md`         — every option, enumerated
//!
//! Run with `pnpm schema`. `scripts/check-generated.sh` re-runs it and fails if
//! anything changed, so the docs cannot drift from the code.

use emerald_lib::settings::Settings;
use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("no repo root")?
        .to_path_buf();

    let schema = serde_json::to_value(schemars::schema_for!(Settings))?;
    let defaults = serde_json::to_value(Settings::default())?;

    let schema_path = root.join("schema/focus-access.schema.json");
    std::fs::create_dir_all(schema_path.parent().unwrap())?;
    std::fs::write(&schema_path, format!("{}\n", serde_json::to_string_pretty(&schema)?))?;

    let ts = emit_typescript(&schema, &defaults)?;
    std::fs::write(root.join("src/lib/settings.gen.ts"), ts)?;

    let md = emit_markdown(&schema, &defaults)?;
    std::fs::write(root.join("docs/settings-schema.md"), md)?;

    eprintln!("wrote schema/focus-access.schema.json, src/lib/settings.gen.ts, docs/settings-schema.md");
    Ok(())
}

// ---------------------------------------------------------------------------
// Schema walking helpers
// ---------------------------------------------------------------------------

fn defs(schema: &Value) -> &Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    schema
        .get("$defs")
        .and_then(Value::as_object)
        .unwrap_or_else(|| EMPTY.get_or_init(Map::new))
}

/// Follow a `$ref` to its definition, if the node is one.
fn resolve<'a>(node: &'a Value, defs: &'a Map<String, Value>) -> &'a Value {
    node.get("$ref")
        .and_then(Value::as_str)
        .and_then(|r| r.rsplit('/').next())
        .and_then(|name| defs.get(name))
        .unwrap_or(node)
}

fn ref_name(node: &Value) -> Option<&str> {
    node.get("$ref")
        .and_then(Value::as_str)
        .and_then(|r| r.rsplit('/').next())
}

fn description(node: &Value) -> String {
    node.get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// String-enum variants as `(value, description)`, for both shapes schemars
/// emits: bare `enum` arrays, and `oneOf` lists of documented `const`s.
fn enum_variants(node: &Value) -> Option<Vec<(String, String)>> {
    if let Some(list) = node.get("oneOf").and_then(Value::as_array) {
        // schemars mixes the two forms in one `oneOf` when only some variants
        // carry doc comments: documented ones become `{const, description}`,
        // the rest are collapsed into a single `{enum: [...]}` entry. Missing
        // the second form silently drops undocumented variants from the
        // generated union, so both are collected here.
        let mut variants = Vec::new();
        for entry in list {
            if let Some(c) = entry.get("const").and_then(Value::as_str) {
                variants.push((c.to_string(), description(entry)));
            } else if let Some(values) = entry.get("enum").and_then(Value::as_array) {
                let shared = description(entry);
                for v in values.iter().filter_map(Value::as_str) {
                    variants.push((v.to_string(), shared.clone()));
                }
            }
        }
        if !variants.is_empty() {
            return Some(variants);
        }
    }
    if let Some(list) = node.get("enum").and_then(Value::as_array) {
        let variants: Vec<_> = list
            .iter()
            .filter_map(|v| v.as_str().map(|s| (s.to_string(), String::new())))
            .collect();
        if !variants.is_empty() {
            return Some(variants);
        }
    }
    None
}

fn range_note(node: &Value) -> Option<String> {
    let lo = node.get("minimum").and_then(Value::as_f64);
    let hi = node.get("maximum").and_then(Value::as_f64);
    match (lo, hi) {
        (Some(a), Some(b)) => Some(format!("{} – {}", num(a), num(b))),
        (Some(a), None) => Some(format!("≥ {}", num(a))),
        (None, Some(b)) => Some(format!("≤ {}", num(b))),
        (None, None) => None,
    }
}

fn num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// Root object first, then every `$def` that is an object with properties, in
/// dependency order so TS interfaces never forward-reference.
fn object_defs(schema: &Value) -> Vec<(String, Value)> {
    let d = defs(schema);
    let mut out: Vec<(String, Value)> = d
        .iter()
        .filter(|(_, v)| v.get("properties").is_some())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.push(("Settings".into(), schema.clone()));
    out
}

// ---------------------------------------------------------------------------
// TypeScript emitter
// ---------------------------------------------------------------------------

fn emit_typescript(schema: &Value, defaults: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let d = defs(schema);
    let mut out = String::new();
    out.push_str(
        "// GENERATED FILE — do not edit.\n\
         // Source of truth: src-tauri/src/settings.rs. Regenerate with `pnpm schema`.\n\n",
    );

    // String-union types for every enum.
    let mut enum_names: Vec<&String> = d
        .iter()
        .filter(|(_, v)| enum_variants(v).is_some())
        .map(|(k, _)| k)
        .collect();
    enum_names.sort();
    for name in enum_names {
        let node = &d[name];
        let variants = enum_variants(node).unwrap();
        let desc = description(node);
        if !desc.is_empty() {
            let _ = writeln!(out, "/** {desc} */");
        }
        let union = variants
            .iter()
            .map(|(v, _)| format!("'{v}'"))
            .collect::<Vec<_>>()
            .join(" | ");
        let _ = writeln!(out, "export type {name} = {union};");
        // A runtime-usable list, so the settings UI can render options without
        // a second hand-maintained array.
        let values = variants
            .iter()
            .map(|(v, _)| format!("'{v}'"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "export const {name}Values: readonly {name}[] = [{values}] as const;");
        // Per-variant help text, used verbatim by the Focus & Access panel.
        let _ = writeln!(out, "export const {name}Help: Record<{name}, string> = {{");
        for (v, vd) in &variants {
            let _ = writeln!(out, "  '{}': {},", v, json_str(vd));
        }
        out.push_str("};\n\n");
    }

    // Interfaces.
    for (name, node) in object_defs(schema) {
        let desc = description(&node);
        if !desc.is_empty() {
            let _ = writeln!(out, "/** {desc} */");
        }
        let _ = writeln!(out, "export interface {name} {{");
        let props = node
            .get("properties")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut keys: Vec<&String> = props.keys().collect();
        keys.sort();
        for key in keys {
            let prop = &props[key];
            let pdesc = description(prop);
            if !pdesc.is_empty() {
                let _ = writeln!(out, "  /** {pdesc} */");
            }
            let _ = writeln!(out, "  {}: {};", key, ts_type(prop, d));
        }
        out.push_str("}\n\n");
    }

    // Defaults, straight from `Settings::default()`.
    out.push_str("/** Exactly `Settings::default()` from settings.rs. */\n");
    let _ = writeln!(
        out,
        "export const DEFAULT_SETTINGS: Settings = {};\n",
        serde_json::to_string_pretty(defaults)?
    );

    Ok(out)
}

fn ts_type(prop: &Value, d: &Map<String, Value>) -> String {
    if let Some(name) = ref_name(prop) {
        // Enums and objects both surface as their generated TS name.
        if d.contains_key(name) {
            return name.to_string();
        }
    }
    let node = resolve(prop, d);
    if enum_variants(node).is_some() {
        if let Some(name) = ref_name(prop) {
            return name.to_string();
        }
    }
    match node.get("type").and_then(Value::as_str) {
        Some("string") => "string".into(),
        Some("boolean") => "boolean".into(),
        Some("integer") | Some("number") => "number".into(),
        Some("array") => {
            let items = node.get("items").map(|i| ts_type(i, d)).unwrap_or_else(|| "unknown".into());
            format!("{items}[]")
        }
        Some("object") => "Record<string, unknown>".into(),
        _ => "unknown".into(),
    }
}

fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

// ---------------------------------------------------------------------------
// Markdown emitter
// ---------------------------------------------------------------------------

fn emit_markdown(schema: &Value, defaults: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let d = defs(schema);
    let mut out = String::new();
    out.push_str(
        "<!-- GENERATED FILE — do not edit. Source: src-tauri/src/settings.rs. \
         Regenerate with `pnpm schema`. -->\n\n\
         # Settings schema\n\n\
         Every option Emerald persists, generated from the Rust types that define them.\n\
         Nothing here is aspirational: if an option is listed, it is read by running code.\n\n\
         Settings live in a single JSON file, hand-editable, at:\n\n\
         | Platform | Path |\n| --- | --- |\n\
         | Linux | `~/.config/app.emerald.browser/settings.json` |\n\
         | macOS | `~/Library/Application Support/app.emerald.browser/settings.json` |\n\
         | Windows | `%APPDATA%\\app.emerald.browser\\settings.json` |\n\n\
         Out-of-range values are clamped on load, not rejected. Unknown keys are an\n\
         error, so a typo surfaces instead of silently doing nothing. A file that\n\
         fails to parse is moved aside to `settings.json.bak` and defaults are used.\n\n",
    );

    let counted = count_options(schema);
    let _ = writeln!(
        out,
        "**{counted} options** across {} sections.\n",
        section_order().len()
    );

    for (path, title, blurb) in section_order() {
        let _ = writeln!(out, "## {title}\n");
        if !blurb.is_empty() {
            let _ = writeln!(out, "{blurb}\n");
        }
        let node = node_at(schema, d, path);
        let default_node = value_at(defaults, path);
        let _ = writeln!(out, "| Option | Type | Default | Range | Meaning |");
        let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
        let props = node
            .and_then(|n| n.get("properties"))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut keys: Vec<&String> = props.keys().collect();
        keys.sort();
        for key in keys {
            let prop = &props[key];
            let resolved = resolve(prop, d);
            if resolved.get("properties").is_some() {
                continue; // nested section, has its own table
            }
            let ty = if let Some(variants) = enum_variants(resolved) {
                variants
                    .iter()
                    .map(|(v, _)| format!("`{v}`"))
                    .collect::<Vec<_>>()
                    .join(" \\| ")
            } else {
                format!("`{}`", ts_type(prop, d))
            };
            let default = default_node
                .and_then(|n| n.get(key))
                .map(render_default)
                .unwrap_or_else(|| "—".into());
            let range = range_note(resolved).unwrap_or_else(|| "—".into());
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {} | {} |",
                key,
                ty,
                default,
                range,
                description(prop)
            );
        }
        out.push('\n');

        // Per-variant meanings for any enum used in this section.
        let mut wrote_header = false;
        let mut keys: Vec<&String> = props.keys().collect();
        keys.sort();
        for key in keys {
            let prop = &props[key];
            let resolved = resolve(prop, d);
            let Some(variants) = enum_variants(resolved) else {
                continue;
            };
            if variants.iter().all(|(_, dsc)| dsc.is_empty()) {
                continue;
            }
            if !wrote_header {
                let _ = writeln!(out, "<details><summary>What the choices mean</summary>\n");
                wrote_header = true;
            }
            let _ = writeln!(out, "**`{key}`**\n");
            for (v, dsc) in variants {
                let _ = writeln!(out, "- `{v}` — {dsc}");
            }
            out.push('\n');
        }
        if wrote_header {
            out.push_str("</details>\n\n");
        }
    }

    out.push_str(
        "## Not settings, on purpose\n\n\
         Some behaviour is not configurable because a configuration point implies a\n\
         mode where it is off, and those modes should not exist:\n\n\
         - **Nothing in Emerald's chrome flashes, blinks, pulses, or breathes.** Not in\n\
           any theme, not at any animation speed, not to get your attention. There is no\n\
           setting because there is no code path that does it.\n\
         - **No telemetry, no crash reporting, no update ping, no phone-home.** Emerald\n\
           opens exactly the connections you ask it to. There is no opt-out because there\n\
           is nothing to opt out of.\n\
         - **Hue semantics are fixed.** The accent means \"active, or yours\"; red means\n\
           destructive; yellow means needs-attention. `sensory_profile` changes saturation\n\
           and contrast, never which colour means what, so muscle memory survives the\n\
           switch.\n",
    );

    Ok(out)
}

fn render_default(v: &Value) -> String {
    match v {
        Value::String(s) if s.is_empty() => "*(empty)*".into(),
        Value::String(s) => format!("`{s}`"),
        Value::Bool(b) => format!("`{b}`"),
        Value::Null => "—".into(),
        // f32 defaults widen to f64 on the way through JSON, so 0.35 arrives
        // as 0.3499999940395355. Printing that in the documentation would be
        // both ugly and misleading about the precision on offer.
        Value::Number(n) if !n.is_i64() && !n.is_u64() => match n.as_f64() {
            Some(f) => format!("`{}`", trim_float(f)),
            None => format!("`{n}`"),
        },
        other => format!("`{other}`"),
    }
}

/// Shortest decimal that round-trips through f32, up to 4 places.
fn trim_float(f: f64) -> String {
    for places in 0..=4 {
        let s = format!("{f:.places$}");
        if s.parse::<f64>().map(|p| (p - f).abs() < 1e-6).unwrap_or(false) {
            return s;
        }
    }
    format!("{f:.4}")
}

fn section_order() -> Vec<(&'static [&'static str], &'static str, &'static str)> {
    vec![
        (
            &["focus_access", "attention"],
            "Focus & Access → Attention",
            "Decluttering, focus mode, and the motion budget. Written with ADHD in mind, \
             though the tab ceiling is the single biggest memory lever in the browser and \
             everyone benefits from it.",
        ),
        (
            &["focus_access", "predictability"],
            "Focus & Access → Predictability",
            "Nothing moves, plays, or reorders without being asked. Written with autism in \
             mind. Note that `no_layout_shift` and `stable_tab_order` default to on — \
             Emerald treats a calm baseline as the product, not as an accessibility mode \
             you have to find.",
        ),
        (
            &["focus_access", "reading"],
            "Focus & Access → Reading",
            "Typography, spacing and reading aids. Written with dyslexia in mind. The \
             spacing floors from WCAG 1.4.12 (Text Spacing) are reachable and exceedable: \
             line height 1.5×, letter spacing 0.12em, word spacing 0.16em, paragraph \
             spacing 2×. Applies to reader mode always, and to every page when \
             `apply_to_all_pages` is on.",
        ),
        (
            &["focus_access", "input"],
            "Focus & Access → Input",
            "Larger targets, dictation, and never losing typed text. Written with \
             dysgraphia in mind. `min_target_px` defaults to 44, which is WCAG 2.2 \
             Target Size (Enhanced, AAA) rather than the 24px AA minimum.",
        ),
        (&["appearance"], "Appearance", "Colour, type, and chrome layout."),
        (
            &["performance"],
            "Performance",
            "Memory and process behaviour. `memory_budget_mb` and the idle-tab policies are \
             the only things that cause Emerald to run a recurring timer; with all of them \
             off it does no periodic work whatsoever.",
        ),
        (
            &["privacy"],
            "Privacy",
            "What leaves the machine. The short answer is nothing that you did not ask for.",
        ),
        (
            &["extensions"],
            "Extensions",
            "Chrome extensions run on Windows, where Emerald's engine is WebView2, and do              not run on macOS or Linux, whose engines have no Chrome extension system.              These options are still read and stored everywhere — installing on a platform              that cannot load them is allowed and simply does nothing until you run Emerald              somewhere that can. There is no Chrome Web Store integration; see              `docs/architecture.md` §7.5.",
        ),
    ]
}

fn node_at<'a>(
    schema: &'a Value,
    d: &'a Map<String, Value>,
    path: &[&str],
) -> Option<&'a Value> {
    let mut cur = schema;
    for seg in path {
        cur = resolve(cur.get("properties")?.get(*seg)?, d);
    }
    Some(cur)
}

fn value_at<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = root;
    for seg in path {
        cur = cur.get(*seg)?;
    }
    Some(cur)
}

fn count_options(schema: &Value) -> usize {
    let d = defs(schema);
    section_order()
        .iter()
        .filter_map(|(path, _, _)| {
            let node = node_at(schema, d, path)?;
            let props = node.get("properties")?.as_object()?;
            Some(
                props
                    .values()
                    .filter(|p| resolve(p, d).get("properties").is_none())
                    .count(),
            )
        })
        .sum()
}
