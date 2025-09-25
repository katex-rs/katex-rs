# KaTeX Rendering Performance Benchmarks

This folder contains scripts that provide an apples-to-apples rendering
benchmark between the Rust and JavaScript implementations of KaTeX.
The workloads mirror the scenarios exercised by
[`KaTeX/test/perf-test.js`](../../KaTeX/test/perf-test.js).

## JavaScript (reference)

```bash
cd KaTeX
npm install
npm run test:perf
```

The upstream script uses [`benchmark.js`](https://benchmarkjs.com) and reports
operations per second for KaTeX’s JavaScript renderer.

## Rust (native)

```bash
cargo bench --bench perf
```

## Rust (WebAssembly)

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