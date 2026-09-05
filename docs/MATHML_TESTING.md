# MathML browser tests and first-case readiness

## Running and diagnosing

Build current sources and exercise the first screenshot case:

```sh
cargo xtask screenshotter --browser firefox --case Accents --build always --allow-js-fallback --html-on-failure
cargo xtask screenshotter --browser firefox --case Accents --mathml --allow-js-fallback --html-on-failure
```

To stress cold starts (after building once), repeat the command in new processes:

```sh
for i in $(seq 1 20); do
  cargo xtask screenshotter --browser firefox --case Accents --build never --allow-js-fallback --html-on-failure || break
done
```

Repeat with `--mathml` for native MathML. Each process starts a fresh browser
session. HTML and MathML run in independent CI matrix jobs, so one failure does
not prevent the other mode from running.

`--mathml` sets the payload once, before any rendering. Baseline loading, WASM
rendering, JS fallback and diagnostic DOM rendering all use the same mode.
MathML files use `Accents-mathml-firefox.png` rather than
`Accents-firefox.png`. No comparison tolerance was relaxed.

On a fallback mismatch, `artifacts/screenshots/new/` contains both the WASM
image and a `-js` image; `diff/` contains the JS-vs-WASM comparison, not a stale
baseline diff. With `--html-on-failure`, `html/` contains both implementations'
DOM for the failing case. Diagnostics explicitly re-render the case because
asynchronous image comparison may finish after another case is on screen.
CI uploads all three directories, with separate artifact names for each mode.

## Known upstream MathML warnings

`LowerAccent` and `StretchyAccent` contain line-segment accents whose MathML
code points are missing in the pinned JS reference, which emits literal
`undefined` where Rust emits a blank fallback.

With `--allow-js-fallback` (as used in CI), these two MathML pixel mismatches
are warnings rather than failures, but only when the successfully rendered
JS DOM still contains the known `undefined` operator. GitHub Actions receives
a `::warning` annotation and the summary counts warnings separately. A run
with only these warnings exits successfully. PNG/diff/HTML artifacts are
retained and uploaded normally; neither case is skipped and no baseline or
pixel tolerance is changed.

HTML-mode mismatches, other case names, rendering/timeouts/comparison errors,
and mismatches after the reference stops emitting `undefined` remain errors.
Baseline-only comparisons without JS fallback are not waived.

Regression tests: `cargo test -p xtask`.

## Readiness contract

Previously, the runner waited only for the existence of `window.runCase`.
The initial empty render and font preloading could still be running, and
`__ready` could be true before font preloading completed. This is unsafe for
the first case, especially on cold Firefox sessions.

The runner now waits for `__initialSetupDone`, and initialization and all
programmatic renders share a queue. A successful render waits for image load
and decode, requests layout, awaits `document.fonts.ready` for fonts selected
by that render (including native MathML), and samples stable geometry across
three animation frames. `__ready` is set only after that work completes, or
with an explicit error status. Image failures are errors, including cached
failures whose `error` event has already fired. Readiness waits are bounded by
`--timeout`; WebDriver's script timeout allows the page to report the error.
Initialization errors fail immediately rather than timing out as screenshots.

The dependency-free regression suite runs in CI:

```sh
node --test xtask/tests/screenshot-ready.test.mjs
```

It covers delayed initialization racing the first `Accents` render, queued
WASM/JS rendering, lazy fonts, changing layout, broken or stalled images,
decode completion, initialization errors and macro isolation. Rust tests
also ensure MathML baseline lookup cannot read an HTML baseline.
