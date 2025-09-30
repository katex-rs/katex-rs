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

This repository is organised as a Cargo workspace. The core crate lives in [`crates/katex`](crates/katex), the WebAssembly bindings are packaged via [`crates/wasm-binding`](crates/wasm-binding), and supporting assets such as the screenshot tests remain at the repository root.

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

The `crates/katex/data` directory contains the JSON files extracted from the original KaTeX repository. They are kept here to simplify crate compilation. You can regenerate them using the Rust-based xtask workflow:

```bash
git submodule update --init --recursive
cargo +nightly xtask extract-data
```

The command requires the nightly toolchain (to compile the `xtask` crate) and the upstream KaTeX submodule checkout.

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

Use the `xtask` runner to build the WebAssembly package, host the static test
assets, and drive browsers via WebDriver. By default the harness targets Safari
(on macOS hosts), Firefox, and Google Chrome (chromedriver and friends are
launched automatically when available). Install chromedriver and geckodriver as
needed. Safari is supported on macOS hosts when `safaridriver` is available and
WebDriver has been enabled (`safaridriver --enable`). You can also pass
`--webdriver` to reuse an existing endpoint. Run:

```bash
cargo xtask screenshotter
```

See `cargo xtask screenshotter --help` for additional flags such as filtering
the dataset, skipping the Wasm rebuild, or changing browsers (for example,
`--browser firefox` or `--browser chrome,firefox,safari`). Use `--safaridriver`
to point to a custom binary path when needed. The diff tolerance can be tuned
with `--tolerance strict|normal|tolerant`, `strict` enforces pixel-perfect matches, and `tolerant` allows minor rendering drift during broader refactors while still flagging
significant mismatches and writing diff composites for the latter.

The harness consumes stylesheets and fonts from the upstream KaTeX submodule.
When the compiled assets are missing (or `--build always` is provided) the
runner executes `yarn install --frozen-lockfile` and `yarn build` inside the
submodule to produce `dist/katex.min.css` and its fonts, which are then served
through the same `/tests/screenshotter/katex.min.css` route used by the legacy
JavaScript harness. WebAssembly artifacts are loaded directly from
`crates/wasm-binding/pkg` rather than being copied into the test directory; they are
rebuilt automatically when `katex.js` is missing. Ensure Yarn is installed and
fetch the submodule via `git submodule update --init --recursive` before running
the screenshots. To speed up runs, the Rust harness only keeps artifacts for
cases with differences or tolerance notes; perfect matches clear any prior files
from `artifacts/screenshots/new` and `artifacts/screenshots/diff`.

On Linux hosts without browser tooling installed, install the browsers,
matching WebDrivers, and `wasm-pack` before running the harness:

```bash
# Google Chrome stable
wget https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
sudo apt-get install -y ./google-chrome-stable_current_amd64.deb

# ChromeDriver matching the installed build (replace CHROME_BUILD if needed)
CHROME_BUILD="$(google-chrome --version | awk '{print $3}' | cut -d. -f1-4)"
wget "https://storage.googleapis.com/chrome-for-testing-public/${CHROME_BUILD}/linux64/chromedriver-linux64.zip"
unzip chromedriver-linux64.zip
sudo install -m755 chromedriver-linux64/chromedriver /usr/local/bin/chromedriver

# Firefox and geckodriver (update versions as required)
wget -O firefox.tar.xz "https://download.mozilla.org/?product=firefox-latest&os=linux64&lang=en-US"
sudo tar -xf firefox.tar.xz -C /opt
sudo ln -sf /opt/firefox/firefox /usr/local/bin/firefox

wget https://github.com/mozilla/geckodriver/releases/download/v0.36.0/geckodriver-v0.36.0-linux64.tar.gz
tar -xf geckodriver-v0.36.0-linux64.tar.gz
sudo install -m755 geckodriver /usr/local/bin/geckodriver

# wasm-pack (installs to ~/.cargo/bin)
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

Adjust the download URLs when newer browser builds are released.

### Performance profiling

Use the `xtask` helper to capture flamegraphs for the native, wasm, and
JavaScript harnesses. Detailed instructions and baseline measurements live in
[`docs/FLAMEGRAPH.md`](docs/FLAMEGRAPH.md).

## Compatibility

- **Rust**: 1.70+ (Testing and Linting needs nightly)
- **WebAssembly**: Supports all modern browsers
- **KaTeX**: Fully compatible with the original KaTeX JavaScript version

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.