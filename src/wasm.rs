//! WebAssembly bindings for KaTeX Rust implementation
//!
//! This module provides JavaScript-compatible APIs that mirror the original
//! KaTeX.js library, allowing seamless integration with web applications.

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::macro_expander::MacroMap;
use crate::macros::MacroDefinition;
use crate::{
    context::KatexContext,
    core,
    types::{OutputFormat, Settings, StrictMode, StrictSetting, TrustSetting},
};

/// Global KaTeX context for WASM
use std::sync::OnceLock;

static KATEX_CONTEXT: OnceLock<KatexContext> = OnceLock::new();

/// Initialize the global KaTeX context

fn get_context() -> &'static KatexContext {
    KATEX_CONTEXT.get_or_init(KatexContext::default)
}

/// Render LaTeX expression to a DOM element

#[wasm_bindgen]
pub fn render(
    tex: &str,
    element: &web_sys::Node,
    options: Option<Settings>,
) -> Result<(), JsValue> {
    let ctx = get_context();
    let mut settings = options.unwrap_or_default();
    settings.output = OutputFormat::HtmlAndMathml;

    core::render(ctx, tex, element, &settings).map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Render LaTeX expression to HTML string

#[wasm_bindgen]
pub fn render_to_string(tex: &str, options: Option<Settings>) -> Result<String, JsValue> {
    let ctx = get_context();
    let mut settings = options.unwrap_or_default();
    settings.output = OutputFormat::HtmlAndMathml;

    core::render_to_string(ctx, tex, &settings).map_err(|e| JsValue::from_str(&format!("{e}")))
}

fn parse_js_options(js: &JsValue) -> Result<Settings, JsValue> {
    if js.is_undefined() || js.is_null() {
        return Ok(Settings::default());
    }
    if !js.is_object() {
        return Err(JsValue::from_str("options must be a plain object"));
    }
    let obj: Object = Object::from(js.clone());

    let get = |key: &str| -> Result<JsValue, JsValue> {
        Reflect::get(&obj, &JsValue::from_str(key))
            .map_err(|_| JsValue::from_str(&format!("failed to read option '{}'", key)))
    };
    let opt_bool = |key: &str| -> Result<Option<bool>, JsValue> {
        let v = get(key)?;
        if v.is_undefined() || v.is_null() {
            Ok(None)
        } else {
            v.as_bool()
                .map(Some)
                .ok_or_else(|| JsValue::from_str(&format!("option '{}' must be a boolean", key)))
        }
    };
    let opt_string = |key: &str| -> Result<Option<String>, JsValue> {
        let v = get(key)?;
        if v.is_undefined() || v.is_null() {
            Ok(None)
        } else {
            v.as_string()
                .map(Some)
                .ok_or_else(|| JsValue::from_str(&format!("option '{}' must be a string", key)))
        }
    };
    let opt_f64 = |key: &str| -> Result<Option<f64>, JsValue> {
        let v = get(key)?;
        if v.is_undefined() || v.is_null() {
            Ok(None)
        } else {
            v.as_f64()
                .map(Some)
                .ok_or_else(|| JsValue::from_str(&format!("option '{}' must be a number", key)))
        }
    };

    let mut settings = Settings::default();

    // display / displayMode
    if let Some(dm) = opt_bool("displayMode")? {
        settings.display_mode = dm;
    } else if let Some(d) = opt_bool("display")? {
        settings.display_mode = d;
    }

    // throwOnError / noThrow
    match (opt_bool("throwOnError")?, opt_bool("noThrow")?) {
        (Some(toe), _) => {
            settings.throw_on_error = toe;
        }
        (None, Some(no_throw)) => {
            settings.throw_on_error = !no_throw;
        }
        (None, None) => {
            // keep default (true)
        }
    }

    // errorColor
    if let Some(color) = opt_string("errorColor")? {
        settings.error_color = color;
    }

    // macros: { string -> string }
    let macros_val = get("macros")?;
    if !macros_val.is_undefined() && !macros_val.is_null() {
        if Array::is_array(&macros_val) {
            return Err(JsValue::from_str(
                "option 'macros' must be a plain object, not an array",
            ));
        }
        if !macros_val.is_object() {
            return Err(JsValue::from_str("option 'macros' must be an object"));
        }
        let macros_obj: Object = Object::from(macros_val);
        let keys = Object::keys(&macros_obj);
        let mut m: MacroMap = MacroMap::default();
        for key in keys.iter() {
            let k = key
                .as_string()
                .ok_or_else(|| JsValue::from_str("macros keys must be strings"))?;
            let val = Reflect::get(&macros_obj, &JsValue::from_str(&k))
                .map_err(|_| JsValue::from_str(&format!("failed to read macros['{}']", k)))?;
            let s = val
                .as_string()
                .ok_or_else(|| JsValue::from_str(&format!("macros['{}'] must be a string", k)))?;
            m.insert(k, MacroDefinition::String(s));
        }
        {
            let mut target = settings.macros.borrow_mut();
            *target = m;
        }
    }

    // strict
    let strict_val = get("strict")?;
    if !strict_val.is_undefined() && !strict_val.is_null() {
        if let Some(b) = strict_val.as_bool() {
            settings.strict = StrictSetting::Bool(b);
        } else if let Some(s) = strict_val.as_string() {
            match s.to_lowercase().as_str() {
                "ignore" => settings.strict = StrictSetting::Mode(StrictMode::Ignore),
                "warn" => settings.strict = StrictSetting::Mode(StrictMode::Warn),
                "error" => settings.strict = StrictSetting::Mode(StrictMode::Error),
                other => {
                    return Err(JsValue::from_str(&format!(
                        "option 'strict' string not recognized: '{}'; expected 'ignore' | 'warn' | 'error'",
                        other
                    )));
                }
            }
        } else {
            return Err(JsValue::from_str(
                "option 'strict' must be a boolean or one of: 'ignore' | 'warn' | 'error'",
            ));
        }
    }

    // trust
    let trust_val = get("trust")?;
    if !trust_val.is_undefined() && !trust_val.is_null() {
        if let Some(b) = trust_val.as_bool() {
            settings.trust = TrustSetting::Bool(b);
        } else {
            return Err(JsValue::from_str(
                "option 'trust' currently supports only boolean",
            ));
        }
    }

    // output format
    let output_val = get("output")?;
    if !output_val.is_undefined() && !output_val.is_null() {
        if let Some(s) = output_val.as_string() {
            settings.output = match s.to_lowercase().as_str() {
                "html" => OutputFormat::Html,
                "mathml" => OutputFormat::Mathml,
                "htmlandmathml" => OutputFormat::HtmlAndMathml,
                _ => OutputFormat::Html, // Default to HTML for screenshot compatibility
            };
        } else {
            return Err(JsValue::from_str("option 'output' must be a string"));
        }
    }

    // simple booleans
    if let Some(b) = opt_bool("leqno")? {
        settings.leqno = b;
    }
    if let Some(b) = opt_bool("fleqn")? {
        settings.fleqn = b;
    }
    if let Some(b) = opt_bool("colorIsTextColor")? {
        settings.color_is_text_color = b;
    }
    if let Some(b) = opt_bool("globalGroup")? {
        settings.global_group = b;
    }

    // numbers
    if let Some(n) = opt_f64("minRuleThickness")? {
        if !n.is_finite() || n.is_sign_negative() {
            return Err(JsValue::from_str(
                "option 'minRuleThickness' must be a non-negative finite number",
            ));
        }
        settings.min_rule_thickness = n;
    }
    if let Some(n) = opt_f64("sizeMultiplier")? {
        if !n.is_finite() || n.is_sign_negative() {
            return Err(JsValue::from_str(
                "option 'sizeMultiplier' must be a non-negative finite number",
            ));
        }
        settings.size_multiplier = n;
    }
    if let Some(n) = opt_f64("maxExpand")? {
        if !n.is_finite() {
            return Err(JsValue::from_str(
                "option 'maxExpand' must be a finite non-negative integer",
            ));
        }
        if n < 0.0 {
            return Err(JsValue::from_str("option 'maxExpand' must be non-negative"));
        }
        if (n.fract()).abs() > 0.0 {
            return Err(JsValue::from_str("option 'maxExpand' must be an integer"));
        }
        settings.max_expand = n as usize;
    }

    // color (math color)
    if let Some(c) = opt_string("color")? {
        settings.color = Some(c);
    }

    Ok(settings)
}

/// Render LaTeX expression to a DOM element with options specified as a JS
/// object
#[wasm_bindgen]
pub fn render_with_options(
    tex: &str,
    element: web_sys::Element,
    js_options: JsValue,
) -> Result<(), JsValue> {
    let ctx = get_context();
    let settings = parse_js_options(&js_options)?;

    let node: web_sys::Node = element.unchecked_into();
    core::render(ctx, tex, &node, &settings).map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Render LaTeX expression to HTML string with options specified as a JS object
#[wasm_bindgen]
pub fn render_to_string_with_options(tex: &str, js_options: JsValue) -> Result<String, JsValue> {
    let ctx = get_context();
    let settings = parse_js_options(&js_options)?;

    core::render_to_string(ctx, tex, &settings).map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Render LaTeX expression to MathML string
#[wasm_bindgen]
pub fn render_to_mathml(tex: &str, options: Option<Settings>) -> Result<String, JsValue> {
    let ctx = get_context();
    let mut settings = options.unwrap_or_default();
    settings.output = OutputFormat::Mathml;

    core::render_to_string(ctx, tex, &settings).map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Render LaTeX expression to HTML string (HTML-only output)
#[wasm_bindgen]
pub fn render_to_html(tex: &str, options: Option<Settings>) -> Result<String, JsValue> {
    let ctx = get_context();
    let mut settings = options.unwrap_or_default();
    settings.output = OutputFormat::Html;

    core::render_to_string(ctx, tex, &settings).map_err(|e| JsValue::from_str(&format!("{e}")))
}
