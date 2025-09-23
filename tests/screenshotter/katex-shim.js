/*!
 * katex-shim.js
 * Expose a KaTeX-compatible JS API on top of katex-rs WASM exports.
 * - API: createShim(wasm) -> { render(tex, element, options), renderToString(tex, options) }
 * - Options mapping: JS plain object keys are forwarded for parsing in Rust (see parse_js_options).
 *   Supported keys: display/displayMode, throwOnError/noThrow, errorColor, macros, strict, trust,
 *   leqno, fleqn, minRuleThickness, colorIsTextColor, maxExpand, globalGroup, sizeMultiplier, color, output.
 */
export function createShim(wasm) {
  // Normalize/forward KaTeX-style options to a plain object parsed by Rust.
  function mapOptions(options) {
    const o = options || {};
    const out = {};

    // display or displayMode
    if (Object.prototype.hasOwnProperty.call(o, "displayMode")) out.displayMode = !!o.displayMode;
    if (Object.prototype.hasOwnProperty.call(o, "display")) out.display = !!o.display;

    // throwOnError / noThrow
    if (Object.prototype.hasOwnProperty.call(o, "throwOnError")) {
      out.throwOnError = !!o.throwOnError;
    } else if (Object.prototype.hasOwnProperty.call(o, "noThrow")) {
      out.noThrow = !!o.noThrow;
    }

    // simple scalars
    if (o.errorColor != null) out.errorColor = String(o.errorColor);
    if (o.color != null) out.color = String(o.color);
    if (Object.prototype.hasOwnProperty.call(o, "leqno")) out.leqno = !!o.leqno;
    if (Object.prototype.hasOwnProperty.call(o, "fleqn")) out.fleqn = !!o.fleqn;
    if (Object.prototype.hasOwnProperty.call(o, "colorIsTextColor")) out.colorIsTextColor = !!o.colorIsTextColor;
    if (Object.prototype.hasOwnProperty.call(o, "globalGroup")) out.globalGroup = !!o.globalGroup;

    // numeric
    if (Object.prototype.hasOwnProperty.call(o, "minRuleThickness")) out.minRuleThickness = Number(o.minRuleThickness);
    if (Object.prototype.hasOwnProperty.call(o, "maxExpand")) out.maxExpand = Number(o.maxExpand);
    if (Object.prototype.hasOwnProperty.call(o, "sizeMultiplier")) out.sizeMultiplier = Number(o.sizeMultiplier);

    // enums/strings
    if (Object.prototype.hasOwnProperty.call(o, "strict")) out.strict = o.strict; // bool or "warn" | "error" | "ignore"
    if (Object.prototype.hasOwnProperty.call(o, "trust")) out.trust = o.trust;   // bool only in v1
    if (Object.prototype.hasOwnProperty.call(o, "output") && o.output != null) out.output = String(o.output);

    // macros: plain object map
    if (o.macros && typeof o.macros === "object") out.macros = o.macros;

    return out;
  }

  function render(tex, element, options) {
    if (!element) throw new Error("katex.render: element is required");
    const jsOpts = mapOptions(options);
    try {
      if (typeof wasm.render_with_options === "function") {
        // Preferred path: let Rust render directly into the Element
        wasm.render_with_options(tex, element, jsOpts);
      } else if (typeof wasm.render_to_string_with_options === "function") {
        // Fallback: render to string then inject
        const html = wasm.render_to_string_with_options(tex, jsOpts);
        element.innerHTML = html;
      } else if (typeof wasm.render_to_string === "function") {
        const html = wasm.render_to_string(tex);
        element.innerHTML = html;
      } else {
        throw new Error("katex-rs WASM: no suitable render function exported");
      }
    } catch (e) {
      const noThrow = options && (options.noThrow === true || options.throwOnError === false);
      if (noThrow) {
        const color = (options && options.errorColor) || "#cc0000";
        element.innerHTML = '<span style="color:' + String(color) + '">' +
          String(e && e.message ? e.message : e) + "</span>";
        return;
      }
      throw e;
    }
  }

  function renderToString(tex, options) {
    const jsOpts = mapOptions(options);
    if (typeof wasm.render_to_string_with_options === "function") {
      return wasm.render_to_string_with_options(tex, jsOpts);
    }
    if (typeof wasm.render_to_string === "function") {
      return wasm.render_to_string(tex);
    }
    // Last-resort fallback via DOM (should rarely trigger)
    const span = document.createElement("span");
    render(tex, span, options);
    return span.innerHTML;
  }

  return { render, renderToString };
}