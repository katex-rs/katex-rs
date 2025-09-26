# KaTeX-rs

[![Crates.io](https://img.shields.io/crates/v/katex-rs.svg)](https://crates.io/crates/katex-rs)
[![Documentation](https://docs.rs/katex-rs/badge.svg)](https://docs.rs/katex-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![NPM version](https://img.shields.io/npm/v/katex-rs.svg)](https://www.npmjs.com/package/katex-rs)

**KaTeX-rs** is a Rust implementation of [KaTeX](https://github.com/KaTeX/KaTeX), providing fast mathematical typesetting capabilities, not limited to Javascript environments.

## Project Introduction

KaTeX-rs is a working in progress Rust port of KaTeX (a fast mathematical typesetting library). It converts LaTeX mathematical expressions into HTML and MathML formats, supporting server-side rendering, command-line tools, and WebAssembly environments.

This project is based on KaTeX's commit [9fb63136e680715ad83c119366f6f697105d2c55](https://github.com/KaTeX/KaTeX/commit/9fb63136e680715ad83c119366f6f697105d2c55).

- [x] Basic parsing and rendering
- [x] Unit and integration tests
- [x] Offline rendering tests
- [x] Compatible with `no-std` and `wasm` target
- [ ] Fully consistent with KaTeX result

## Workspace Layout

This repository is organised as a Cargo workspace. The core crate lives in [`crates/katex`](crates/katex), while supporting assets such as the screenshot tests remain at the repository root.

## How to Use

Add `katex-rs` to your `Cargo.toml`:

```toml
[dependencies]
katex-rs = "0.1"
```

Basic usage:

```rust
use katex::{KatexContext, Settings, render_to_string};

fn main() -> Result<(), katex::ParseError> {
    let ctx = KatexContext::default();
    let settings = Settings::default();

    let html = render_to_string(&ctx, r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}", &settings)?;
    println!("{}", html);
    Ok(())
}
```

For display mode (block math):

```rust
use katex::{KatexContext, Settings, render_to_string};

fn main() -> Result<(), katex::ParseError> {
    let ctx = KatexContext::default();
    let settings = Settings::builder()
        .display_mode(true)
        .build();

    let html = render_to_string(&ctx, r"\sum_{i=1}^{n} x_i", &settings)?;
    println!("{}", html);
    Ok(())
}
```

### Feature Flags

- `backtrace`: Enables backtrace support for better error diagnostics
- `wasm`: Enables WebAssembly support

## Prerequisites for development

For development, ensure you have fully checked out the repository with all submodules and Git LFS files:
```
git lfs install && git lfs pull
git submodule update --init --recursive
```

For testing, you will need to have `node` and `npm` installed and available in your `PATH`.
You also need nightly Rust toolchain for some linting and testing features.
`wasm-pack` would automatically install required toolchain for wasm target.

Install `wasm-pack` and `cargo-nextest` (for running tests) either via the script:
```bash
rustup default nightly
curl -LsSf https://get.nexte.st/latest/linux | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin
curl https://drager.github.io/wasm-pack/installer/init.sh -sSf | sh
```

(Or with cargo if you prefer but it's very slow)
```bash
cargo install --locked cargo-nextest
cargo install wasm-pack
```

### Const Data Extraction

The `crates/katex/data` directory contains the JSON files extracted with scripts from the original KaTeX repository. They are kept here to simplify crate compilation. You can regenerate them with the scripts in `./utils` if needed.

```bash
node utils/extract_font_metric.cjs
node utils/extract_sigmas_and_xis.cjs
node utils/extract_symbols.js
```

## Testing and Linting

For formatting, you can use:

```bash
cargo fmt --all
```

For linting, you will need nightly Rust toolchain. You can run the linter with:

```bash
cargo clippy --all-targets --all-features
```

### Unit tests
```bash
cargo nextest run --no-fail-fast
```

### Screenshot tests

The script will automatically build the wasm and run the screenshot tests. Currently recommend to install Google Chrome for best compatibility.

```bash
pushd ./tests/screenshotter && npm install && popd
node tests/screenshotter/run.js
```

## Compatibility

- **Rust**: 1.70+ (Testing and Linting needs nightly)
- **WebAssembly**: Supports all modern browsers
- **KaTeX**: Fully compatible with the original KaTeX JavaScript version

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.