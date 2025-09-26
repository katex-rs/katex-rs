# Flamegraph tooling and baseline measurements

This document summarises the new profiling workflow introduced via
`cargo xtask flamegraph` and records the first set of measurements gathered while
validating the native Criterion benchmark harness.

## Prerequisites

Before collecting data, make sure the repository is fully hydrated:

- Fetch the KaTeX submodule so that the shared test fixtures are available:
  ```bash
  git submodule update --init --recursive
  ```
- Install the external tools used by the helpers:
  - Linux `perf`: 
    ```bash
    sudo apt install linux-tools-common linux-tools-generic linux-tools-$(uname -r)
    ```
  - `wasm-pack` (for WebAssembly profiling)
  - `npm` (for the Node.js scripts)

The native harness expects debugging information for accurate stack traces. The
workspace now exposes a `profiling` Cargo profile that inherits from `release`
while forcing `debug = true` and disabling LTO.

## `cargo xtask flamegraph`

The `xtask` binary wraps all three benchmarking environments and emits
`inferno`-compatible flamegraphs under `target/flamegraphs/`:

```bash
# Native Criterion benchmark (Rust)
cargo xtask flamegraph native

# WebAssembly renderer executed under Node.js
cargo xtask flamegraph wasm

# Upstream JavaScript renderer from the KaTeX checkout
cargo xtask flamegraph js
```

Each subcommand accepts a common set of options:

- `--frequency <HZ>` to tweak the sampling rate (`perf -F`, defaults to 999Hz)
- `--call-graph <dwarf|fp|lbr>` to configure call stack unwinding
- `--output <svg>` and `--perf-data <file>` to customise artefact locations
- `--title <text>` to override the SVG heading
- `--open` to launch the generated SVG when profiling finishes

Target-specific switches are also available:

- `native`
  - `--case <NAME>` limits Criterion to a single benchmark function
  - `--profile <NAME>` selects an alternate Cargo profile (default: `profiling`)
  - additional arguments after `--` are forwarded to Criterion verbatim
- `wasm`
  - `--case <NAME>` filters the wasm benchmark (repeatable)
  - `--node-arg` / `--script-arg` forward extra flags to Node.js / the script
  - `--skip-build`, `--skip-npm` skip the wasm build or dependency install
- `js`
  - `--node-arg` / `--script-arg` mirror the wasm options
  - `--skip-npm` avoids re-installing KaTeX’s dependencies

## Outstanding environment gaps

- WebAssembly profiling requires `wasm-pack`; the tool is not present in the
  current container, so the wasm flamegraph command will fail until it is
  installed (`cargo install wasm-pack` or the official installer script).
- Installing KaTeX’s JavaScript dependencies with `npm install` currently hits a
  peer-dependency conflict between `stylelint` and `stylelint-scss`. Running
  `npm install --legacy-peer-deps` (or updating the dependency constraints)
  resolves the issue locally before invoking `cargo xtask flamegraph js`.

Once these prerequisites are met, `cargo xtask flamegraph wasm --case ArrayMode`
(or similar) can be used to generate comparable flamegraphs for the wasm and
JavaScript harnesses.
