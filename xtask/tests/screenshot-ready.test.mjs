import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { setImmediate } from "node:timers/promises";
import test from "node:test";
import vm from "node:vm";
import { abortable, waitForImage, waitForRender, withRenderTimeout } from "../assets/screenshot-ready.mjs";

function deferred() {
  let resolve, reject;
  const promise = new Promise((res, rej) => { resolve = res; reject = rej; });
  return { promise, resolve, reject };
}

class Image extends EventTarget {
  complete = false;
  naturalWidth = 0;
  src = "/test.png";
}

test("already failed images reject instead of waiting for an event that has fired", async () => {
  const img = new Image();
  img.complete = true;
  await assert.rejects(waitForImage(img, new AbortController().signal), /Image failed to load/);
});

test("image errors are failures, not successful readiness", async () => {
  const img = new Image();
  const result = waitForImage(img, new AbortController().signal);
  img.dispatchEvent(new Event("error"));
  await assert.rejects(result, /Image failed to load/);
});

test("loaded images wait for decoding", async () => {
  const img = new Image();
  img.complete = true;
  img.naturalWidth = 100;
  const decoded = deferred();
  img.decode = () => decoded.promise;
  let ready = false;
  const result = waitForImage(img, new AbortController().signal).then(() => { ready = true; });
  await setImmediate();
  assert.equal(ready, false);
  decoded.resolve();
  await result;
});

function documentForLayout(fonts, images = [], widths = [100]) {
  let frames = 0;
  let layoutForced = false;
  const node = { getBoundingClientRect: () => ({ x: 0, y: 0, width: widths[Math.min(frames - 1, widths.length - 1)], height: 20 }) };
  return {
    body: { getBoundingClientRect: () => { layoutForced = true; return {}; } },
    get fonts() { assert.ok(layoutForced); return fonts; },
    querySelectorAll: selector => selector === "img" ? images : [node],
    frame: async signal => { signal.throwIfAborted(); frames++; await setImmediate(); },
    frames: () => frames,
  };
}

test("each render waits for lazy fonts and then three stable layout samples", async () => {
  const font = deferred();
  const doc = documentForLayout({ ready: font.promise, status: "loaded" }, [], [10, 50, 100, 100, 100]);
  const result = waitForRender(doc, new AbortController().signal, doc.frame);
  await setImmediate();
  assert.equal(doc.frames(), 0);
  font.resolve();
  await result;
  assert.equal(doc.frames(), 5);
});

test("font and image hangs have a bounded, explicit error", async () => {
  const pending = new Promise(() => {});
  const docs = [
    documentForLayout({ ready: pending, status: "loading" }),
    documentForLayout(null, [new Image()]),
  ];
  for (const doc of docs) {
    await assert.rejects(
      withRenderTimeout(signal => waitForRender(doc, signal, doc.frame), 20),
      /Render readiness timed out/,
    );
  }
});

const page = await readFile(new URL("../assets/screenshot.html", import.meta.url), "utf8");
const pageScript = page.match(/<script type="module">([\s\S]*?)<\/script>/)[1]
  .replace(/import \{[^}]+\} from "\.\/screenshot-ready\.mjs";/, "")
  .replace('import("./pkg/katex.js")', "Promise.resolve(mockWasm)")
  .replace('import("/katex.mjs")', "Promise.resolve(mockJs)");

function loadPage({ fonts = Promise.resolve(), init = Promise.resolve(), markup = tex => `<math>${tex}</math>` } = {}) {
  const renders = [];
  const nodes = Object.fromEntries(["pre", "math", "post"].map(id => [id, {
    innerHTML: "", textContent: "", setAttribute() {}, removeAttribute() {}, appendChild() {},
    getBoundingClientRect: () => ({ x: 0, y: 0, width: 100, height: 20 }),
    querySelector(selector) {
      return (selector === "math" && this.innerHTML.includes("<math>")) ||
        (selector === ".katex-error" && this.innerHTML.includes("katex-error")) ? {} : null;
    },
  }]));
  const window = { location: { search: "" }, dispatchEvent() {} };
  const api = mode => ({
    render(tex, node, opts) {
      renders.push({ mode, tex, output: opts.output, macros: structuredClone(opts.macros) });
      node.innerHTML = markup(tex);
      if (opts.macros) opts.macros.changedByRenderer = "true";
    },
  });
  const document = {
    fonts: { load: () => fonts, ready: Promise.resolve(), status: "loaded" },
    body: { getBoundingClientRect() {} },
    getElementById: id => nodes[id],
    querySelectorAll: selector => selector === "img" ? [] : Object.values(nodes),
    createTextNode: text => ({ text }),
    createElement: tag => ({ tag }),
  };
  vm.runInNewContext(pageScript, {
    window, document, URLSearchParams, structuredClone,
    mockWasm: { default: () => init, ...api("wasm") }, mockJs: api("js"),
    abortable, withRenderTimeout,
    waitForRender: (doc, signal) => waitForRender(doc, signal, async s => {
      s.throwIfAborted(); await setImmediate();
    }),
  });
  return { window, renders, nodes };
}

async function until(predicate) {
  for (let i = 0; i < 100; i++) {
    if (predicate()) return;
    await setImmediate();
  }
  assert.fail("page did not reach expected state");
}

test("slow initial fonts/WASM cannot overwrite the first Accents case with a blank page", async () => {
  const fonts = deferred();
  const init = deferred();
  const { window, renders, nodes } = loadPage({ fonts: fonts.promise, init: init.promise });
  assert.equal(typeof window.runCase, "function");
  assert.equal(window.__initialSetupDone, false);
  const first = window.runCase({ tex: "Accents", output: "mathml" });
  await setImmediate();
  assert.equal(renders.length, 0);
  fonts.resolve();
  await setImmediate();
  assert.equal(window.__initialSetupDone, false);
  init.resolve();
  assert.equal((await first).state, "rendered");
  assert.equal(window.__initialSetupDone, true);
  assert.deepEqual(renders.map(r => r.tex), ["", "Accents"]);
  assert.equal(nodes.math.innerHTML, "<math>Accents</math>");
  assert.equal(window.__ready, true);
});

test("initialization failures report status without declaring setup complete", async () => {
  const fonts = deferred();
  const { window } = loadPage({ fonts: fonts.promise });
  fonts.reject(new Error("font download failed"));
  await until(() => window.__status.state === "error");
  assert.equal(window.__initialSetupDone, false);
  assert.match(window.__status.message, /font download failed/);
  await assert.rejects(window.runCase({tex: "Accents"}), /initialization failed/);
});

test("MathML readiness rejects missing math but accepts intentional noThrow error markup", async () => {
  const { window } = loadPage({ markup: tex => tex ? '<span class="katex-error">invalid</span>' : '<math></math>' });
  await until(() => window.__initialSetupDone);
  const invalid = await window.runCase({ tex: "invalid", output: "mathml" });
  assert.equal(invalid.state, "error");
  const expectedError = await window.runCase({ tex: "invalid", output: "mathml", noThrow: true });
  assert.equal(expectedError.state, "rendered");
});

test("queued WASM/JS renders preserve MathML mode and isolate macro mutations", async () => {
  const { window, renders, nodes } = loadPage();
  await until(() => window.__initialSetupDone);
  const payload = { tex: "first", output: "mathml", macros: { original: "1" } };
  const first = window.runCase(payload);
  const second = window.renderWithImpl("js", { ...payload, tex: "second" });
  await Promise.all([first, second]);
  assert.deepEqual(renders.slice(1).map(r => [r.mode, r.tex, r.output]), [
    ["wasm", "first", "mathml"], ["js", "second", "mathml"],
  ]);
  assert.deepEqual(payload.macros, { original: "1" });
  assert.deepEqual(renders[2].macros, { original: "1" });
  assert.equal(nodes.math.innerHTML, "<math>second</math>");
});
