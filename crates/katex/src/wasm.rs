//! WebAssembly bindings that expose a KaTeX-compatible JavaScript API.
//!
//! The real KaTeX package for Node.js exports two primary entry points –
//! `render` and `renderToString` – as plain JavaScript functions that accept a
//! KaTeX-style options object.  Consumers expect to be able to call these
//! functions directly without constructing intermediate Rust objects.  The
//! previous bindings exported thin wrappers around [`Settings`], which made the
//! WASM build awkward to use and required a bespoke shim in the screenshotter
//! tests.
//!
//! This module mirrors the Node.js surface.  Options are accepted as plain
//! JavaScript objects, `ParseError`s are thrown just like in KaTeX.js, and the
//! exported names match the canonical camelCase spellings.  This allows the
//! generated `pkg/katex.js` bundle to be dropped into existing KaTeX tooling –
//! including the upstream screenshotter – without additional glue code.

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::prelude::*;

use std::sync::OnceLock;

use crate::macro_expander::MacroMap;
use crate::macros::MacroDefinition;
use crate::{
    ParseError,
    context::KatexContext,
    core,
    types::{OutputFormat, Settings, StrictMode, StrictSetting, TrustSetting},
};

/// Cached global [`KatexContext`].
fn get_context() -> &'static KatexContext {
    static CONTEXT: OnceLock<KatexContext> = OnceLock::new();
    CONTEXT.get_or_init(KatexContext::default)
}

/// Wrapper carrying the resolved settings together with metadata describing
/// whether the caller explicitly set the output format.
struct JsSettings {
    settings: Settings,
    output_specified: bool,
}

fn js_error(message: &str) -> JsValue {
    JsValue::from_str(message)
}

fn parse_js_bool(value: &JsValue, key: &str) -> Result<bool, JsValue> {
    value
        .as_bool()
        .ok_or_else(|| js_error(&format!("option '{key}' must be a boolean")))
}

fn parse_js_number(value: &JsValue, key: &str) -> Result<f64, JsValue> {
    value
        .as_f64()
        .ok_or_else(|| js_error(&format!("option '{key}' must be a number")))
}

fn parse_js_string(value: &JsValue, key: &str) -> Result<String, JsValue> {
    value
        .as_string()
        .ok_or_else(|| js_error(&format!("option '{key}' must be a string")))
}

fn parse_js_options(options: JsValue) -> Result<JsSettings, JsValue> {
    if options.is_undefined() || options.is_null() {
        return Ok(JsSettings {
            settings: Settings::default(),
            output_specified: false,
        });
    }

    if !options.is_object() {
        return Err(js_error("options must be a plain object"));
    }

    let obj: Object = options.into();
    let get = |key: &str| -> Result<JsValue, JsValue> {
        Reflect::get(&obj, &JsValue::from_str(key))
            .map_err(|_| js_error(&format!("failed to read option '{key}'")))
    };
    let opt_bool = |key: &str| -> Result<Option<bool>, JsValue> {
        let v = get(key)?;
        if v.is_undefined() || v.is_null() {
            Ok(None)
        } else {
            Ok(Some(parse_js_bool(&v, key)?))
        }
    };
    let opt_number = |key: &str| -> Result<Option<f64>, JsValue> {
        let v = get(key)?;
        if v.is_undefined() || v.is_null() {
            Ok(None)
        } else {
            Ok(Some(parse_js_number(&v, key)?))
        }
    };
    let opt_string = |key: &str| -> Result<Option<String>, JsValue> {
        let v = get(key)?;
        if v.is_undefined() || v.is_null() {
            Ok(None)
        } else {
            Ok(Some(parse_js_string(&v, key)?))
        }
    };

    let mut settings = Settings::default();
    let mut output_specified = false;

    if let Some(display_mode) = opt_bool("displayMode")? {
        settings.display_mode = display_mode;
    } else if let Some(display) = opt_bool("display")? {
        settings.display_mode = display;
    }

    match (opt_bool("throwOnError")?, opt_bool("noThrow")?) {
        (Some(throw_on_error), _) => settings.throw_on_error = throw_on_error,
        (None, Some(no_throw)) => settings.throw_on_error = !no_throw,
        (None, None) => {}
    }

    if let Some(color) = opt_string("errorColor")? {
        settings.error_color = color;
    }

    if let Some(color) = opt_string("color")? {
        settings.color = Some(color);
    }

    if let Some(leqno) = opt_bool("leqno")? {
        settings.leqno = leqno;
    }
    if let Some(fleqn) = opt_bool("fleqn")? {
        settings.fleqn = fleqn;
    }
    if let Some(color_is_text_color) = opt_bool("colorIsTextColor")? {
        settings.color_is_text_color = color_is_text_color;
    }
    if let Some(global_group) = opt_bool("globalGroup")? {
        settings.global_group = global_group;
    }

    if let Some(min_rule_thickness) = opt_number("minRuleThickness")? {
        if !(min_rule_thickness.is_finite() && min_rule_thickness >= 0.0) {
            return Err(js_error(
                "option 'minRuleThickness' must be a non-negative finite number",
            ));
        }
        settings.min_rule_thickness = min_rule_thickness;
    }

    if let Some(size_multiplier) = opt_number("sizeMultiplier")? {
        if !(size_multiplier.is_finite() && size_multiplier >= 0.0) {
            return Err(js_error(
                "option 'sizeMultiplier' must be a non-negative finite number",
            ));
        }
        settings.size_multiplier = size_multiplier;
    }

    if let Some(max_size) = opt_number("maxSize")? {
        if !(max_size.is_finite() && max_size >= 0.0) {
            return Err(js_error(
                "option 'maxSize' must be a non-negative finite number",
            ));
        }
        settings.max_size = max_size;
    }

    if let Some(max_expand) = opt_number("maxExpand")? {
        if !(max_expand.is_finite() && max_expand >= 0.0) {
            return Err(js_error(
                "option 'maxExpand' must be a finite non-negative integer",
            ));
        }
        if max_expand.fract() != 0.0 {
            return Err(js_error("option 'maxExpand' must be an integer"));
        }
        settings.max_expand = max_expand as usize;
    }

    let strict_value = get("strict")?;
    if !strict_value.is_undefined() && !strict_value.is_null() {
        if let Some(strict_bool) = strict_value.as_bool() {
            settings.strict = StrictSetting::Bool(strict_bool);
        } else if let Some(strict_string) = strict_value.as_string() {
            let strict_mode = match strict_string.to_lowercase().as_str() {
                "ignore" => StrictMode::Ignore,
                "warn" => StrictMode::Warn,
                "error" => StrictMode::Error,
                other => {
                    return Err(js_error(&format!(
                        "option 'strict' string not recognized: '{other}'; expected 'ignore' | 'warn' | 'error'",
                    )));
                }
            };
            settings.strict = StrictSetting::Mode(strict_mode);
        } else {
            return Err(js_error(
                "option 'strict' must be a boolean or one of: 'ignore' | 'warn' | 'error'",
            ));
        }
    }

    let trust_value = get("trust")?;
    if !trust_value.is_undefined() && !trust_value.is_null() {
        if let Some(trust_bool) = trust_value.as_bool() {
            settings.trust = TrustSetting::Bool(trust_bool);
        } else {
            return Err(js_error("option 'trust' currently supports only boolean"));
        }
    }

    if let Some(output) = opt_string("output")? {
        settings.output = match output.to_lowercase().as_str() {
            "html" => OutputFormat::Html,
            "mathml" => OutputFormat::Mathml,
            "htmlandmathml" => OutputFormat::HtmlAndMathml,
            other => {
                return Err(js_error(&format!(
                    "option 'output' must be one of 'html', 'mathml', 'htmlAndMathml'; received '{other}'",
                )));
            }
        };
        output_specified = true;
    }

    let macros_value = get("macros")?;
    if !macros_value.is_undefined() && !macros_value.is_null() {
        if Array::is_array(&macros_value) {
            return Err(js_error(
                "option 'macros' must be a plain object, not an array",
            ));
        }
        if !macros_value.is_object() {
            return Err(js_error("option 'macros' must be a plain object"));
        }

        let macros_obj: Object = macros_value.into();
        let keys = Object::keys(&macros_obj);
        let mut macros = MacroMap::default();
        for key in keys.iter() {
            let Some(name) = key.as_string() else {
                return Err(js_error("macros keys must be strings"));
            };
            let value = Reflect::get(&macros_obj, &JsValue::from_str(&name))
                .map_err(|_| js_error(&format!("failed to read macros['{name}']")))?;
            let Some(expansion) = value.as_string() else {
                return Err(js_error(&format!("macros['{name}'] must be a string")));
            };
            macros.insert(name, MacroDefinition::String(expansion));
        }
        *settings.macros.borrow_mut() = macros;
    }

    Ok(JsSettings {
        settings,
        output_specified,
    })
}

fn map_parse_error(error: ParseError) -> JsValue {
    JsValue::from(error)
}

fn element_from_js(element: JsValue) -> Result<web_sys::Element, JsValue> {
    if element.is_null() || element.is_undefined() {
        return Err(js_error("katex.render: element is required"));
    }

    element
        .dyn_into::<web_sys::Element>()
        .map_err(|_| js_error("katex.render: element must be a DOM Element"))
}

fn normalize_settings(parsed: JsSettings, default_output: Option<OutputFormat>) -> Settings {
    let mut settings = parsed.settings;
    if !parsed.output_specified
        && let Some(output) = default_output
    {
        settings.output = output;
    }
    settings
}

/// Exported as `katex.render`.
#[wasm_bindgen(js_name = render)]
pub fn render(tex: &str, element: JsValue, options: JsValue) -> Result<(), JsValue> {
    let element = element_from_js(element)?;
    let parsed = parse_js_options(options)?;
    let settings = normalize_settings(parsed, Some(OutputFormat::HtmlAndMathml));

    let node: web_sys::Node = element.unchecked_into();
    core::render(get_context(), tex, &node, &settings).map_err(map_parse_error)
}

/// Exported as `katex.renderToString`.
#[wasm_bindgen(js_name = renderToString)]
pub fn render_to_string(tex: &str, options: JsValue) -> Result<String, JsValue> {
    let parsed = parse_js_options(options)?;
    let settings = normalize_settings(parsed, Some(OutputFormat::HtmlAndMathml));
    core::render_to_string(get_context(), tex, &settings).map_err(map_parse_error)
}

/// Exported as `katex.renderToHTML`.
#[wasm_bindgen(js_name = renderToHTML)]
pub fn render_to_html(tex: &str, options: JsValue) -> Result<String, JsValue> {
    let parsed = parse_js_options(options)?;
    let mut settings = parsed.settings;
    settings.output = OutputFormat::Html;
    core::render_to_string(get_context(), tex, &settings).map_err(map_parse_error)
}

/// Exported as `katex.renderToMathML`.
#[wasm_bindgen(js_name = renderToMathML)]
pub fn render_to_mathml(tex: &str, options: JsValue) -> Result<String, JsValue> {
    let parsed = parse_js_options(options)?;
    let mut settings = parsed.settings;
    settings.output = OutputFormat::Mathml;
    core::render_to_string(get_context(), tex, &settings).map_err(map_parse_error)
}

/// Exported as `katex.version`.
#[wasm_bindgen(js_name = version)]
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Hook to install better panic messages in WASM.
#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}
