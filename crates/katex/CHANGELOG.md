# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/katex-rs/katex-rs/compare/katex-rs-v0.1.1...katex-rs-v0.2.0) - 2025-09-30

### Other

- Refactor data extraction scripts to Rust
- Update GitHub CI workflow for the latest features and configure release-plz
- Split wasm binding to a seperated crate
- Better token processing
- Use Arc in function and env map
- Reduce clone in build_html
- Simplify CssStyle for most cases
- Avoid vec flatten with push_combine_chars and optimize the parser by removing unnecessary clones
- Use ClassList to avoid clones in class list manipulations
- Make various optimizations
- Add flamegraph xtask tool
- Fix svg_geometry
- Refactor the project to use cargo  workspace
