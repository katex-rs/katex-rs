#!/usr/bin/env node
"use strict";

/**
 * Screenshot runner for katex-rs
 *
 * Features:
 * - Starts a minimal static server, serving:
 *   - /tests/* from ./tests/*
 *   - /KaTeX/* from ./KaTeX/*
 * - Loads cases from KaTeX/test/screenshotter/ss_data.js if possible,
 *   otherwise tries parsing KaTeX/test/screenshotter/ss_data.yaml with `yaml` module.
 *   If neither works, supports --tex for ad-hoc validation.
 * - Drives headless Chrome via Puppeteer to open tests/screenshotter/test.html,
 *   waits for window.__ready, screenshots #math (clip), and compares with baseline.
 * - Baseline path: KaTeX/test/screenshotter/images/{CaseKey}-chrome.png
 * - Outputs on failure:
 *   - tests/screenshotter/new/{CaseKey}-chrome.png
 *   - tests/screenshotter/diff/{CaseKey}-chrome-diff.png (if pixelmatch+pngjs available)
 */

const fs = require("fs");
const fsp = fs.promises;
const path = require("path");
const http = require("http");
const url = require("url");
const cp = require("child_process");

// ---------- CLI options ----------
const argv = require("node:process").argv.slice(2);
const opts = {
  include: null,             // comma-separated substrings to include
  exclude: null,             // comma-separated substrings to exclude
  attempts: 1,               // retry attempts per case
  waitMs: 0,                 // extra wait before screenshot (ms)
  headless: "new",           // puppeteer headless mode
  port: 0,                   // 0 = random
  timeoutMs: 15000,          // per-case timeout for __ready
  case: null,                // run a single named case
  tex: null,                 // fallback: run a single tex (when YAML not available)
  build: "auto",             // auto|always|never (auto by default)
};
for (let i = 0; i < argv.length; i++) {
  const a = argv[i];
  if (a === "--include" && argv[i+1]) { opts.include = argv[++i]; }
  else if (a === "--exclude" && argv[i+1]) { opts.exclude = argv[++i]; }
  else if (a === "--attempts" && argv[i+1]) { opts.attempts = parseInt(argv[++i], 10) || opts.attempts; }
  else if (a === "--wait" && argv[i+1]) { opts.waitMs = parseFloat(argv[++i]) * 1000; }
  else if (a === "--port" && argv[i+1]) { opts.port = parseInt(argv[++i], 10) || 0; }
  else if (a === "--timeout" && argv[i+1]) { opts.timeoutMs = parseInt(argv[++i], 10) || opts.timeoutMs; }
  else if (a === "--case" && argv[i+1]) { opts.case = argv[++i]; }
  else if (a === "--tex" && argv[i+1]) { opts.tex = argv[++i]; }
  else if (a === "--build" && argv[i+1]) { opts.build = argv[++i]; }
}

// ---------- Paths ----------
const ROOT = process.cwd();
const TESTS_DIR = path.join(ROOT, "tests", "screenshotter");
const PAGE_PATH = "/tests/screenshotter/test.html";
const BASELINE_DIR = path.join(ROOT, "KaTeX", "test", "screenshotter", "images");
const NEW_DIR = path.join(TESTS_DIR, "new");
const DIFF_DIR = path.join(TESTS_DIR, "diff");
const SPECIAL_PATHS = new Map([
  ["/website/static/img/khan-academy.png", path.join(ROOT, "KaTeX", "website", "static", "img", "khan-academy.png")]
]);

// Ensure output dirs
fs.mkdirSync(NEW_DIR, { recursive: true });
fs.mkdirSync(DIFF_DIR, { recursive: true });

// ---------- Build helpers (one-command build WASM + run) ----------
function runCmd(cmd, args, opts = {}) {
  const res = cp.spawnSync(cmd, args, { stdio: "inherit", cwd: ROOT, ...opts });
  if (res.error) throw res.error;
  if (res.status !== 0) throw new Error(`${cmd} ${args.join(" ")} failed with code ${res.status}`);
  return res;
}

function fileExists(p) {
  try { fs.accessSync(p); return true; } catch (_) { return false; }
}

function ensureWasmBuiltSync(force = false) {
  const outDir = path.join("tests", "screenshotter", "pkg");
  console.log(`[build] Building WASM artifacts into ${outDir} ...`);
  // Try wasm-pack first
  try {
    runCmd("wasm-pack", ["--version"], { stdio: "ignore" });
    // Use the proper flag to pass features through wasm-pack
    runCmd("wasm-pack", ["build", "--target", "web", "--no-opt", "--dev"]);
    console.log("[build] wasm-pack build done");
    // Copy the ./pkg dir to current dir
    fs.cpSync(path.join(process.cwd(),"pkg"), path.join(__dirname,"pkg"), { recursive: true, force: true });
    return;
  } catch (e) {
    console.error(`[build] wasm-pack unavailable or failed: ${e && e.message ? e.message : e}`);
    throw e;
  }
}

// ---------- Static server ----------
function contentTypeFor(p) {
  const ext = path.extname(p).toLowerCase();
  switch (ext) {
    case ".html": return "text/html; charset=utf-8";
    case ".js": return "text/javascript; charset=utf-8";
    case ".mjs": return "text/javascript; charset=utf-8";
    case ".css": return "text/css; charset=utf-8";
    case ".png": return "image/png";
    case ".jpg":
    case ".jpeg": return "image/jpeg";
    case ".svg": return "image/svg+xml";
    case ".woff": return "font/woff";
    case ".woff2": return "font/woff2";
    case ".ttf": return "font/ttf";
    case ".wasm": return "application/wasm";
    case ".json": return "application/json; charset=utf-8";
    default: return "application/octet-stream";
  }
}

function startStaticServer(port = 0) {
  const mounts = [
    { mount: "/tests/", dir: path.join(ROOT, "tests") },
    { mount: "/KaTeX/", dir: path.join(ROOT, "KaTeX") },
    { mount: "/", dir: ROOT },
  ].sort((a, b) => b.mount.length - a.mount.length);

  const server = http.createServer(async (req, res) => {
    try {
      const parsed = url.parse(req.url);
      const reqPath = decodeURIComponent(parsed.pathname || "/");
      const specialPath = SPECIAL_PATHS.get(reqPath);
      if (specialPath) {
        const data = await fsp.readFile(specialPath);
        res.statusCode = 200;
        res.setHeader("Content-Type", contentTypeFor(specialPath));
        res.end(data);
        return;
      }
      // Resolve against the best matching mount
      let localPath = null;
      for (const m of mounts) {
        if (reqPath.startsWith(m.mount)) {
          const rel = reqPath.slice(m.mount.length);
          const safeRel = path.normalize(rel).replace(/^(\.\.[/\\])+/, "");
          localPath = path.join(m.dir, safeRel);
          break;
        }
      }
      if (!localPath) {
        res.statusCode = 404;
        res.end("Not found");
        return;
      }
      const stat = await fsp.stat(localPath).catch(() => null);
      if (!stat) {
        res.statusCode = 404;
        res.end("Not found");
        return;
      }
      let filePath = localPath;
      if (stat.isDirectory()) {
        filePath = path.join(localPath, "index.html");
      }
      const data = await fsp.readFile(filePath);
      res.statusCode = 200;
      res.setHeader("Content-Type", contentTypeFor(filePath));
      res.setHeader("Cache-Control", "no-store, no-cache, must-revalidate, proxy-revalidate");
      res.end(data);
    } catch (e) {
      res.statusCode = 500;
      res.end(String(e && e.stack || e));
    }
  });

  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, "127.0.0.1", () => {
      const addr = server.address();
      resolve({ server, port: addr.port });
    });
  });
}

// ---------- Case loading ----------
function buildQueryFromYamlItem(key, val) {
  // Build URLSearchParams equivalent to KaTeX screenshotter
  const params = new url.URLSearchParams();
  params.set("tex", typeof val === "string" ? val : (val.tex || ""));
  const v = typeof val === "object" && val ? val : {};
  if (v.display) params.set("display", "1");
  if (v.noThrow) params.set("noThrow", "1");
  if (v.errorColor) params.set("errorColor", v.errorColor);
  if (v.styles) params.set("styles", v.styles);
  if (v.pre) params.set("pre", v.pre);
  if (v.post) params.set("post", v.post);
  if (v.color) params.set("color", v.color);
  if (typeof v.leqno === "boolean") params.set("leqno", v.leqno ? "1" : "0");
  if (typeof v.fleqn === "boolean") params.set("fleqn", v.fleqn ? "1" : "0");
  if (typeof v.colorIsTextColor === "boolean") params.set("colorIsTextColor", v.colorIsTextColor ? "1" : "0");
  if (typeof v.globalGroup === "boolean") params.set("globalGroup", v.globalGroup ? "1" : "0");
  if (v.minRuleThickness != null) params.set("minRuleThickness", String(v.minRuleThickness));
  if (v.maxExpand != null) params.set("maxExpand", String(v.maxExpand));
  if (v.sizeMultiplier != null) params.set("sizeMultiplier", String(v.sizeMultiplier));
  if (v.strict != null) params.set("strict", String(v.strict));
  if (v.trust != null) params.set("trust", String(v.trust));
  if (v.output) params.set("output", String(v.output));

  if (v.macros && typeof v.macros === "object") {
    for (const [name, expansion] of Object.entries(v.macros)) {
      // KaTeX style: repeat ?macro=\name=expansion
      params.append("macro", String(name) + "=" + String(expansion));
    }
  }

  return { key, query: params.toString() };
}

function parseQueryToPayload(qs) {
  const params = new url.URLSearchParams(qs || "");
  const payload = {};
  payload.tex = params.get("tex") || "";

  // display/displayMode
  if (params.get("displayMode") != null) {
    payload.displayMode = (params.get("displayMode") === "1" || params.get("displayMode") === "true");
  } else if (params.get("display") != null) {
    payload.display = (params.get("display") === "1" || params.get("display") === "true");
  }

  // throwOnError / noThrow
  if (params.get("throwOnError") != null) {
    payload.throwOnError = !(params.get("throwOnError") === "0" || params.get("throwOnError") === "false");
  } else if (params.get("noThrow") != null) {
    payload.noThrow = (params.get("noThrow") === "1" || params.get("noThrow") === "true");
  }

  // scalars
  if (params.get("errorColor")) payload.errorColor = params.get("errorColor");
  if (params.get("styles")) payload.styles = params.get("styles");
  if (params.get("pre")) payload.pre = params.get("pre");
  if (params.get("post")) payload.post = params.get("post");
  if (params.get("color")) payload.color = params.get("color");

  // booleans
  ["leqno","fleqn","colorIsTextColor","globalGroup"].forEach(k => {
    const v = params.get(k);
    if (v != null) payload[k] = (v === "1" || v === "true");
  });

  // numbers
  ["minRuleThickness","maxExpand","sizeMultiplier"].forEach(k => {
    const v = params.get(k);
    if (v != null) payload[k] = Number(v);
  });

  // enums/strings
  const strict = params.get("strict");
  if (strict != null) {
    payload.strict = (strict === "true" || strict === "false") ? (strict === "true") : strict;
  }
  const trust = params.get("trust");
  if (trust != null) {
    if (trust === "0" || trust === "false") {
      payload.trust = false;
    } else if (trust === "1" || trust === "true") {
      payload.trust = true;
    } else {
      payload.trust = trust;
    }
  }
  const output = params.get("output");
  if (output) payload.output = output;

  // macros: either repeated ?macro=\cmd=expansion or keys starting with backslash
  const macros = {};
  for (const [k, v] of params.entries()) {
    if (k === "macro") {
      try {
        const s = decodeURIComponent(v);
        const m = s.match(/^\\([A-Za-z@]+)\s*=(.*)$/);
        if (m) macros["\\" + m[1]] = m[2];
      } catch (_) {}
    } else if (k && k[0] === "\\") {
      macros[k] = v;
    }
  }
  if (Object.keys(macros).length) payload.macros = macros;

  return payload;
}

async function loadCases() {
  // --case quick path
  if (opts.case && opts.tex) {
    return [{ key: opts.case, query: new url.URLSearchParams({ tex: opts.tex }).toString() }];
  }

  // Prefer YAML first
  const yamlPath = path.join(ROOT, "KaTeX", "test", "screenshotter", "ss_data.yaml");
  if (fs.existsSync(yamlPath)) {
    try {
      // eslint-disable-next-line import/no-extraneous-dependencies, global-require
      const YAML = require("yaml");
      const text = await fsp.readFile(yamlPath, "utf8");
      const obj = YAML.parse(text) || {}; // Map: key -> string | object
      const cases = [];
      for (const [key, val] of Object.entries(obj)) {
        cases.push(buildQueryFromYamlItem(key, val));
      }
      return cases;
    } catch (e) {
      console.warn("[warn] Failed to parse YAML; will attempt ss_data.js. Reason:", e.message || e);
    }
  }

  // Fallback: try KaTeX/test/screenshotter/ss_data.js (depends on js-yaml)
  const ssDataJs = path.join(ROOT, "KaTeX", "test", "screenshotter", "ss_data.js");
  if (fs.existsSync(ssDataJs)) {
    try {
      // eslint-disable-next-line import/no-dynamic-require, global-require
      const data = require(ssDataJs);
      if (Array.isArray(data)) {
        const cases = data.map((it, idx) => {
          const key = it.key || it.name || ("Case" + idx);
          const query = it.query || "";
          return { key, query };
        });
        return cases;
      } else if (data && typeof data === "object") {
        const cases = [];
        for (const [key, itm] of Object.entries(data)) {
          if (itm && typeof itm === "object" && typeof itm.query === "string") {
            cases.push({ key, query: itm.query });
          }
        }
        if (cases.length) return cases;
      }
    } catch (e) {
      console.warn("[warn] Failed to require ss_data.js:", e.message || e);
    }
  }

  // Last resort: --tex must be provided
  if (opts.tex) {
    const name = opts.case || "AdHoc";
    return [{ key: name, query: new url.URLSearchParams({ tex: opts.tex }).toString() }];
  }

  throw new Error("No test cases available. YAML parse failed and ss_data.js unavailable; or use --tex \"a+b\".");
}

function filterCases(cases) {
  let list = cases;
  if (opts.case) {
    list = list.filter(c => c.key === opts.case);
  }
  if (opts.include) {
    const inc = opts.include.split(",").map(s => s.trim()).filter(Boolean);
    list = list.filter(c => inc.some(p => c.key.includes(p)));
  }
  if (opts.exclude) {
    const exc = opts.exclude.split(",").map(s => s.trim()).filter(Boolean);
    list = list.filter(c => !exc.some(p => c.key.includes(p)));
  }
  return list;
}

// ---------- Puppeteer helpers ----------
async function openBrowser() {
  const puppeteer = require("puppeteer");
  const browser = await puppeteer.launch({
    headless: opts.headless,
    args: ["--no-sandbox", "--disable-setuid-sandbox"],
    defaultViewport: { width: 1024, height: 768, deviceScaleFactor: 1 },
  });
  return browser;
}

async function waitReady(page, timeoutMs) {
  await page.waitForFunction("window.__ready === true", { timeout: timeoutMs });
  if (opts.waitMs > 0) {
    await new Promise(r => setTimeout(r, opts.waitMs));
  }
}

async function clipOfMath(page) {
  // Prefer clipping the actual KaTeX root to better match baseline trimming
  let el = await page.$("#math .katex");
  if (!el) {
    el = await page.$("#math");
  }
  if (!el) return null;
  const box = await el.boundingBox();
  if (!box) return null;
  // Round to integer pixels to align with baseline -trim behavior
  return {
    x: Math.round(box.x),
    y: Math.round(box.y),
    width: Math.max(1, Math.round(box.width)),
    height: Math.max(1, Math.round(box.height)),
  };
}

function ensureDir(p) {
  fs.mkdirSync(path.dirname(p), { recursive: true });
}

async function compareOrDiff(actualPath, expectedPath, diffPath) {
  // Strict byte equality first
  try {
    const [a, e] = await Promise.all([
      fsp.readFile(actualPath),
      fsp.readFile(expectedPath),
    ]);
    if (a.equals(e)) return { equal: true, diffWritten: false };
  } catch (err) {
    // expected may not exist; treat as not equal
  }

  // Try pixelwise diff if available
  try {
    const { PNG } = require("pngjs");
    let pixelmatch = require("pixelmatch");
if (pixelmatch && typeof pixelmatch !== "function" && pixelmatch.default && typeof pixelmatch.default === "function") {
  pixelmatch = pixelmatch.default;
}
    const a = PNG.sync.read(await fsp.readFile(actualPath));
    const e = PNG.sync.read(await fsp.readFile(expectedPath));
    if (a.width !== e.width || a.height !== e.height) {
      // different size, write a size-mismatch note via diff (optional)
      console.warn(`[diff] size mismatch: actual ${a.width}x${a.height} vs expected ${e.width}x${e.height}`);
      const w = Math.max(a.width, e.width);
      const h = Math.max(a.height, e.height);
      const diff = new PNG({ width: w, height: h });
      PNG.sync.write(diff); // empty diff as placeholder
      ensureDir(diffPath);
      await fsp.writeFile(diffPath, PNG.sync.write(diff));
      return { equal: false, diffWritten: true };
    }
    const diff = new PNG({ width: a.width, height: a.height });
    const mismatched = pixelmatch(a.data, e.data, diff.data, a.width, a.height, { threshold: 0.025 });
    console.warn(`[diff] size=${a.width}x${a.height}, baseline=${e.width}x${e.height}, mismatched=${mismatched}/${a.width * a.height}`);
    ensureDir(diffPath);
    await fsp.writeFile(diffPath, PNG.sync.write(diff));
    return { equal: mismatched <= 40, diffWritten: true, diffPixels: mismatched }; // allow tiny diffs
  } catch (e) {
    // pixelmatch not available or PNG decode failed; log and skip diff
    console.warn(`[diff] pixel compare unavailable or failed: ${e && e.message ? e.message : e}`);
    return { equal: false, diffWritten: false };
  }
}

// ---------- Main ----------
(async function main() {
  // Auto-build WASM (once) unless disabled
  try {
    if (!("build" in opts) || opts.build !== "never") {
      const force = opts.build === "always";
      ensureWasmBuiltSync(force);
    }
  } catch (e) {
    console.error(`[build] Failed to build latest wasm: ${e && e.message ? e.message : e}`);
    throw e;
  }

  const { server, port } = await startStaticServer(opts.port);
  const serverUrl = `http://127.0.0.1:${port}`;
  const browser = await openBrowser();
  const page = await browser.newPage();
  // Forward browser console to Node for in-page diagnostics
  page.on("console", (msg) => {
    try {
      console.log("[page]", msg.text());
    } catch (_) {
      // ignore logging errors
    }
  });
  // Log failed network requests to identify 404s (e.g., WASM, fonts, images)
  page.on("requestfailed", (req) => {
    try {
      const f = req.failure && req.failure();
      console.warn(`[req-failed] ${req.url()}${f && f.errorText ? " :: " + f.errorText : ""}`);
    } catch (_) {}
  });
  page.on("response", async (res) => {
    try {
      const status = res.status();
      if (status >= 400) {
        console.warn(`[resp ${status}] ${res.url()}`);
      }
    } catch (_) {}
  });

  let cases = await loadCases();
  cases = filterCases(cases);

  console.log(`[info] Loaded ${cases.length} cases. Server: ${serverUrl}`);
  // Load the page once; subsequent cases use window.runCase(payload) without reloads
  const initialUrl = `${serverUrl}${PAGE_PATH}`;
  await page.goto(initialUrl, { waitUntil: "networkidle0" });
  await page.waitForFunction('typeof window.runCase === "function"');

  const failures = [];
  for (const c of cases) {
    let ok = false;
    let lastErr = null;
    let caseStatus = null;
    for (let attempt = 1; attempt <= opts.attempts && !ok; attempt++) {
      try {
        // Render this case on the existing page to avoid reload overhead
        const payload = parseQueryToPayload(c.query || "");
        const runResult = await page.evaluate((p) => window.runCase(p), payload);
        if (runResult && runResult.state === "error") {
          const msg = runResult.message || "Render error";
          lastErr = new Error(msg);
          if (runResult.stack) {
            lastErr.stack = runResult.stack;
          }
          caseStatus = "error";
          console.warn(`[error] ${c.key} (attempt ${attempt}/${opts.attempts}) :: ${msg}`);
          if (runResult.stack) {
            console.warn(runResult.stack);
          }
          break;
        }

        await waitReady(page, opts.timeoutMs);

        // Debug diagnostics: DPR and DOM snapshot info
        const dpr = await page.evaluate(() => window.devicePixelRatio || 1);
        console.log(`[debug] dpr=${dpr}`);
        try {
          const hasKatex = await page.$eval("#math .katex", () => true).catch(() => false);
          const htmlLen = await page.$eval("#math", n => n.innerHTML.length).catch(() => -1);
          console.log(`[debug] dom: has(.katex)=${hasKatex} innerHTML.len=${htmlLen}`);
        } catch (_) {}

        const outFile = path.join(NEW_DIR, `${c.key}-chrome.png`);
        ensureDir(outFile);
        // Match KaTeX baseline: capture fixed viewport (1024x768), no element clip
        await page.screenshot({ path: outFile, fullPage: false });

        const expected = path.join(BASELINE_DIR, `${c.key}-chrome.png`);
        const diffFile = path.join(DIFF_DIR, `${c.key}-chrome-diff.png`);
        const { equal, diffPixels } = await compareOrDiff(outFile, expected, diffFile);
        if (equal) {
          ok = true;
          caseStatus = "pass";
          await page.evaluate(() => window.updateCompareStatus("pass", null));
        } else {
          caseStatus = "mismatch";
          await page.evaluate((state, message) => window.updateCompareStatus(state, message), "mismatch", `Differs from baseline (diff pixels: ${diffPixels})`);
          console.warn(`[fail] ${c.key} (attempt ${attempt}/${opts.attempts})`);
        }
      } catch (e) {
        lastErr = e;
        caseStatus = "error";
        console.warn(`[warn] ${c.key} attempt failed: ${e && e.message ? e.message : e}`);
        // brief backoff
        await new Promise(r => setTimeout(r, 200));
      }
    }
    if (!ok) {
      const failure = {
        key: c.key,
        status: caseStatus && caseStatus !== "pass" ? caseStatus : (lastErr ? "error" : "mismatch"),
      };
      if (lastErr) {
        failure.error = lastErr.stack || lastErr.message || String(lastErr);
      } else if (failure.status === "mismatch") {
        failure.error = "Screenshot differs from baseline";
      }
      failures.push(failure);
    } else {
      console.log(`[pass] ${c.key}`);
    }
  }

  await browser.close();
  server.close();

  if (failures.length) {
    console.error(`[result] FAIL ${failures.length}/${cases.length}. New images at: ${NEW_DIR}. Diffs (if any) at: ${DIFF_DIR}`);
    for (const f of failures) {
      const suffix = f.status ? ` [${f.status}]` : "";
      if (f.error) console.error(`  ${f.key}${suffix}: ${f.error}`);
      else console.error(`  ${f.key}${suffix}`);
    }
    process.exitCode = 1;
    return;
  }
  console.log(`[result] OK ${cases.length}/${cases.length}`);
})();
