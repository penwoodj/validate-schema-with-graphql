# Project Structure

## Overview

This document defines the workspace structure for `graphql-ish-schema-validator`, a Rust-first schema system that compiles GraphQL SDL to a JTD-like intermediate representation (IR) and validates YAML/JSON documents against compiled schemas.

The workspace follows a multi-crate architecture where each crate has a single responsibility:

- **`ir`** - Schema IR types, JsonPointer, SchemaBundle
- **`sdl-parser`** - apollo-parser integration, AST, semantic validation
- **`compiler`** - AST → IR lowering
- **`validator`** - IR-based YAML/JSON validation
- **`registry`** - schema registries, caching
- **`diagnostics`** - error types, miette integration, formatting
- **`jtd-export`** - IR → JTD JSON export
- **`cli`** - binary, clap, main entry point

All crates share common dependencies and conventions defined in the workspace root.

## Workspace Layout

```
graphql-ish-schema-validator/
├── Cargo.toml                    # Workspace root with member definitions
├── crates/
│   ├── ir/                       # Schema IR types, JsonPointer, SchemaBundle
│   │   └── Cargo.toml
│   ├── sdl-parser/               # apollo-parser integration, AST, semantic validation
│   │   └── Cargo.toml
│   ├── compiler/                 # AST → IR lowering
│   │   └── Cargo.toml
│   ├── validator/                # IR-based YAML/JSON validation
│   │   └── Cargo.toml
│   ├── registry/                 # schema registries, caching
│   │   └── Cargo.toml
│   ├── diagnostics/              # error types, miette integration, formatting
│   │   └── Cargo.toml
│   ├── jtd-export/               # IR → JTD JSON export
│   │   └── Cargo.toml
│   └── cli/                     # binary, clap, main entry point
│       └── Cargo.toml
├── tests/                       # integration tests, fixtures
│   ├── fixtures/
│   │   ├── schemas/              # test SDL schemas
│   │   │   ├── minimal.graphql
│   │   │   ├── complex.graphql
│   │   │   └── workflow.graphql
│   │   └── documents/           # test YAML/JSON documents
│   │       ├── valid/
│   │       └── invalid/
│   └── integration/
│       ├── end_to_end.rs
│       └── regression.rs
├── fuzz/                        # cargo-fuzz targets
│   ├── Cargo.toml
│   └── fuzz_targets/
│       ├── sdl_parser.rs
│       ├── yaml_parser.rs
│       └── validator.rs
├── benches/                     # criterion benchmarks
│   ├── Cargo.toml
│   ├── compilation.rs
│   └── validation.rs
├── docs/
│   ├── plans/
│   └── research/
└── .github/workflows/            # CI
    ├── ci.yml
    ├── coverage.yml
    └── release.yml
```

## Workspace Cargo.toml

```toml
[workspace]
members = [
    "crates/ir",
    "crates/sdl-parser",
    "crates/compiler",
    "crates/validator",
    "crates/registry",
    "crates/diagnostics",
    "crates/jtd-export",
    "crates/cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
edition = "2021"
rust-version = "1.75"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/graphql-ish-schema-validator"

[workspace.dependencies]
# Core dependencies
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
miette = { version = "7.0", features = ["fancy"] }

# SDL parsing
apollo-parser = "0.7"

# YAML parsing
serde-saphyr = "0.0.21"

# CLI
clap = { version = "4.4", features = ["derive"] }

# Utilities
indexmap = "2.2"
regex = "1.10"
reqwest = { version = "0.11", features = ["json"] }
lru = "0.12"

# Dev dependencies
proptest = "1.4"
insta = { version = "1.36", features = ["json"] }
criterion = "0.5"
assert_cmd = "2.0"
tempfile = "3.10"
mockito = "1.2"

[workspace.lints.clippy]
deny = "all"
warn-pedantic = "warning"
allow = {
    # Safe patterns we explicitly allow
    "redundant-closure-for-method-calls",
    "module-name-repetitions",
    "must-use-candidate",
}

[workspace.metadata.cargo-llvm-cov]
lcov = true
```

## Key Dependencies and Justification

### Core Dependencies

| Dependency | Purpose | Justification |
|------------|---------|---------------|
| `apollo-parser` | SDL parsing | Error-resilient CST + errors, parses per Oct 2021 spec |
| `serde` / `serde_json` | JSON value model, serialization | De facto standard for JSON in Rust ecosystem |
| `serde-saphyr` | YAML parsing with strict duplicate keys | Fastest YAML parser (294ms), configurable dup key rejection, no unsafe, panic-free, YAML 1.2 |
| `thiserror` | Error derive macros | Concise error definitions with Display/From implementations |
| `miette` | Rich diagnostic reporting | Fancy terminal output, structured Diagnostic support |
| `clap` | CLI argument parsing | Derive mode for clean API, subcommand support |
| `indexmap` | Ordered maps for deterministic output | Preserves insertion order for stable error messages |
| `reqwest` | HTTP client for remote registry | Full-featured async HTTP with timeout support |
| `lru` | LRU cache for compiled schemas | O(1) LRU cache, single-process suitable |
| `regex` | Pattern validation | Fast regex engine for @pattern directive |
| `once_cell` / `std::sync` | Lazy initialization | Safe lazy static and thread-safe initialization |

### Dev Dependencies

| Dependency | Purpose | Justification |
|------------|---------|---------------|
| `proptest` | Property-based testing | Generate random IR + values, invariant checking |
| `insta` | Snapshot testing | Regression testing for error output |
| `criterion` | Benchmarking | Statistical significance, regression detection |
| `assert_cmd` | CLI integration testing | Assert exit codes, stdout/stderr |
| `tempfile` | Temp directory creation | Isolated test environments |
| `mockito` or `wiremock` | HTTP mocking | Test remote registry without network |

## Clippy Configuration

Clippy is configured to catch common issues while allowing patterns that are safe for this codebase:

```rust
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
```

Specific allowances:
- `redundant-closure-for-method-calls` - Method call syntax may be clearer
- `module-name-repetitions` - Crate names are intentionally specific
- `must-use-candidate` - May not want must-use on all results

## rustfmt Configuration

```toml
# rustfmt.toml
edition = "2021"
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_field_init_shorthand = true
use_try_shorthand = true
```

## Feature Flags

Feature flags control optional functionality and reduce dependency footprint:

```toml
[workspace.features]
default = ["cli", "json", "yaml", "registry-http"]
cli = ["dep:clap", "dep:assert_cmd"]
json = ["validator?/json"]
yaml = ["validator?/yaml"]
registry-http = ["registry?/http"]
jtd-export = ["dep:jtd-export"]
diagnostics-pretty = ["diagnostics?/fancy"]
```

## Gap Fix: Feature Flags Table

Complete table of all feature flags, their crate locations, and default values.

| Feature | Description | Crate Location | Default |
|---------|-------------|-----------------|----------|
| `cli` | CLI binary with clap | `crates/cli/Cargo.toml` | Yes |
| `json` | JSON parsing support | `crates/validator/Cargo.toml` | Yes |
| `yaml` | YAML parsing support | `crates/validator/Cargo.toml` | Yes |
| `registry-http` | HTTP registry support | `crates/registry/Cargo.toml` | Yes |
| `jtd-export` | JTD JSON export | `crates/jtd-export/Cargo.toml` | No |
| `diagnostics-pretty` | miette fancy terminal support | `crates/diagnostics/Cargo.toml` | No |

### Feature Descriptions

Feature flags control optional functionality and reduce dependency footprint:

```toml
[workspace.features]
default = ["cli", "json", "yaml", "registry-http"]
cli = ["dep:clap", "dep:assert_cmd"]
json = ["validator?/json"]
yaml = ["validator?/yaml"]
registry-http = ["registry?/http"]
jtd-export = ["dep:jtd-export"]
diagnostics-pretty = ["diagnostics?/fancy"]
```

### Feature Descriptions

| Feature | Description | Default |
|---------|-------------|----------|
| `cli` | CLI binary with clap | Yes |
| `json` | JSON parsing support | Yes |
| `yaml` | YAML parsing support | Yes |
| `registry-http` | HTTP registry support | Yes |
| `jtd-export` | JTD JSON export | No |
| `diagnostics-pretty` | miette fancy terminal support | No |

### Crate-Specific Features

Individual crates can gate functionality:

```toml
# crates/validator/Cargo.toml
[features]
default = ["json", "yaml"]
json = ["serde_json"]
yaml = ["serde-saphyr"]
```

```toml
# crates/diagnostics/Cargo.toml
[features]
default = ["fancy"]
fancy = ["miette/fancy"]
```

```toml
# crates/registry/Cargo.toml
[features]
default = ["http"]
http = ["reqwest"]
```

## Workspace Integration

All crates are linked through the workspace `Cargo.toml`:

### Dependency Graph

```
cli (binary)
├── validator
├── registry
├── compiler
└── diagnostics

validator
├── ir
├── diagnostics
└── serde_json / serde-saphyr

compiler
├── ir
├── sdl-parser
└── diagnostics

sdl-parser
├── apollo-parser
└── diagnostics

registry
├── ir
└── diagnostics

jtd-export
├── ir
└── serde_json

diagnostics
├── thiserror
└── miette

ir (leaf crate)
├── serde
├── indexmap
└── regex
```

## Cross-References

- **`01-ir-design.md`** - Defines Schema IR types implemented in `ir` crate
- **`02-sdl-parsing.md`** - SDL parsing details in `sdl-parser` crate
- **`03-compiler-lowering.md`** - AST → IR lowering in `compiler` crate
- **`04-validator-runtime.md`** - Validation algorithms in `validator` crate
- **`05-registry-subsystem.md`** - Schema registries in `registry` crate
- **`06-error-reporting.md`** - Error types and formatting in `diagnostics` crate
- **`09-testing-strategy.md`** - Testing approach for all crates
- **`10-jtd-export.md`** - IR → JTD export in `jtd-export` crate
- **`11-milestones.md`** - Implementation order across crates

## Open Questions / Decisions Needed

1. **Async vs Sync**: Should registry fetching be async (reqwest) or sync? Current plan uses sync for simplicity, but async may be needed for concurrent registry access.

2. **YAML Parser**: `serde-saphyr` is the clear winner — configurable duplicate key rejection, fastest benchmark speed, no unsafe, panic-free, YAML 1.2. Decision: use serde-saphyr.

3. **Lru vs Moka**: Single-process LRU (`lru`) vs concurrent cache (`moka`) for async workflows? Plan uses `lru` as CLI is typically single-process.

4. **Mockito vs Wiremock**: HTTP mocking library for registry tests? Both are viable, decision can be deferred to implementation.

5. **Criterion Benchmark Output**: Should benchmark results be stored in git or .gitignore? Typically .gitignore to avoid large binary diffs.

## Gap Fix: MSRV Policy

Minimum Supported Rust Version (MSRV) is set to Rust 1.75.

### MSRV Configuration

```toml
[workspace.package]
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
edition = "2021"
rust-version = "1.75"
```

### MSRV Rationale

- **Stability**: Rust 1.75 (2023-12) is stable and widely available.
- **Features**: Supports all required features for this project.
- **CI Testing**: Tested in CI with dtolnay/rust-toolchain@1.75.

### CI MSRV Check

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.75
      - run: cargo test --all-features
```

### MSRV Upgrade Path

If features from newer Rust versions are needed:
1. Update `rust-version` in `workspace.package`
2. Update CI `toolchain` version
3. Test with `cargo +nightly build` before upgrade
4. Document MSRV bump in CHANGELOG

## Research Links

### SDL Parsing
- [Apollo Parser docs.rs](https://docs.rs/apollo-parser/)
- [Apollo Parser crates.io](https://crates.io/crates/apollo-parser)
- [GraphQL Spec October 2021](https://spec.graphql.org/October2021/)

### JTD IR Design
- [RFC 8927 - JSON Type Definition](https://datatracker.ietf.org/doc/html/rfc8927)
- [JTD validation errors guide](https://jsontypedef.com/docs/validation-errors/)

### Diagnostic Tools
- [miette docs.rs](https://docs.rs/miette/)
- [thiserror docs.rs](https://docs.rs/thiserror/)

### CLI Tools
- [clap docs.rs](https://docs.rs/clap/)
- [kubeconform registry pattern](https://github.com/yannh/kubeconform)

### Testing Tools
- [proptest docs.rs](https://docs.rs/proptest/)
- [insta docs.rs](https://insta.rs/)
- [criterion docs.rs](https://bheisler.github.io/criterion.rs/book/)
- [cargo-fuzz book](https://rust-fuzz.github.io/book/cargo-fuzz.html)

### OpenCode Research Corrections
- [YAML Parser Analysis](../research/opencode/yaml-parser-analysis.md) — **Critical correction**: serde-saphyr replaces yaml-rust2. See detailed comparison table.
- [OpenCode Research Index](../research/opencode/README.md) — Full dependency version summary.
- [Rust Tooling 2026](../research/opencode/rust-tooling-2026.md) — Updated tooling versions.

