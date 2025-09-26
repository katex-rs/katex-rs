# KaTeX Rendering Performance Benchmarks

This folder contains scripts that provide an apples-to-apples rendering
benchmark between the Rust and JavaScript implementations of KaTeX.
The workloads mirror the scenarios exercised by
[`KaTeX/test/perf-test.js`](../../KaTeX/test/perf-test.js).

## Running the benchmarks directly

### JavaScript (reference)

```bash
cd KaTeX
npm install
npm run test:perf
```

The upstream script uses [`benchmark.js`](https://benchmarkjs.com) and reports
operations per second for KaTeX’s JavaScript renderer.

### Rust (native)

```bash
cargo bench --bench perf
```

### Rust (WebAssembly)

1. Compile the WebAssembly bindings (requires `wasm-pack`):
   ```bash
   wasm-pack build --release --target nodejs --features wasm
   ```
2. Install the JavaScript dependencies for the helper script:
   ```bash
   cd tests/perf
   npm install
   ```
3. Run the benchmark:
   ```bash
   node wasm-perf.js
   ```

You can limit execution to specific cases with the `--case` flag, e.g.

```bash
node wasm-perf.js --case AccentsText --case Units
```

Use `node wasm-perf.js --list` to print the available case names.

## Flamegraph tooling

The repository provides an `xtask` helper that wraps the setup steps above and
records CPU flamegraphs via Linux `perf` and
[`inferno`](https://github.com/jonhoo/inferno). Examples:

```bash
# Profile the Criterion benchmark harness
cargo xtask flamegraph native

# Profile the wasm renderer under Node.js
cargo xtask flamegraph wasm

# Profile the upstream JavaScript renderer
cargo xtask flamegraph js
```

All flamegraph SVGs are written to `target/flamegraphs/`. Use `--open` to launch
the generated file, or `--output`/`--perf-data` to customise the output paths.