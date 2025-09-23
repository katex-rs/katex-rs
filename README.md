# KaTeX-rs

[![Crates.io](https://img.shields.io/crates/v/katex-rs.svg)](https://crates.io/crates/katex-rs)
[![Documentation](https://docs.rs/katex-rs/badge.svg)](https://docs.rs/katex-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**KaTeX-rs** is a Rust implementation of [KaTeX](https://github.com/KaTeX/KaTeX), providing fast mathematical typesetting capabilities, not limited to Javascript environments.

## Project Introduction

KaTeX-rs is a working in progress Rust port of KaTeX (a fast mathematical typesetting library). It converts LaTeX mathematical expressions into HTML and MathML formats, supporting server-side rendering, command-line tools, and WebAssembly environments.

This project is based on KaTeX's commit [9fb63136e680715ad83c119366f6f697105d2c55](https://github.com/KaTeX/KaTeX/commit/9fb63136e680715ad83c119366f6f697105d2c55).

- [x] Basic parsing and rendering
- [x] Unit and integration tests
- [x] Offline rendering tests
- [x] Compatible with `no-std` and `wasm` target
- [ ] Fully consistent with KaTeX result

### Feature Flags

- `cli`: Enables command-line interface
- `wasm`: Enables WebAssembly support

```toml
[dependencies]
katex-rs = { version = "0.1.0", features = ["cli", "wasm"] }
```

## Testing

### Unit tests
```bash
cargo nextest run --no-fail-fast
```


### Screenshot tests
```bash
node tests/screenshotter/run.js
```

## Compatibility

- **Rust**: 1.70+
- **WebAssembly**: Supports all modern browsers
- **KaTeX**: Fully compatible with the original KaTeX JavaScript version

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.