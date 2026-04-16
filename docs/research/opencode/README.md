# OpenCode Research Index

> Research conducted 2026-04-15 for graphql-ish-schema-validator initial implementation planning.

## Research Files

| File | Topic | Key Finding |
|---|---|---|
| [apollo-parser-analysis.md](./apollo-parser-analysis.md) | SDL parser choice | **apollo-parser v0.8.5** — actively maintained, error-resilient CST, custom directives preserved |
| [jtd-rfc8927-analysis.md](./jtd-rfc8927-analysis.md) | IR inspiration + JTD ecosystem | Build **custom IR** (JTD-inspired with extensions); `jtd` crate optional for export validation |
| [yaml-parser-analysis.md](./yaml-parser-analysis.md) | YAML parser for strict mode | **serde-saphyr** — configurable duplicate key rejection, fastest, no unsafe |
| [kubeconform-miette-competitors.md](./kubeconform-miette-competitors.md) | Behavioral model + diagnostics + competition | kubeconform strict/cache patterns; **miette v7.6** for diagnostics; no competitor does GraphQL SDL → validate YAML/JSON |
| [rust-tooling-2026.md](./rust-tooling-2026.md) | Project infrastructure | cargo-nextest, proptest, insta, thiserror/anyhow/miette, criterion |

## Critical Corrections to ChatGPT Reports

### 1. YAML Parser: yaml-rust2 → serde-saphyr
**ChatGPT report says**: "yaml-rust2 explicitly states it now errors on duplicate keys"
**Reality**: yaml-rust2 uses `HashMap` which silently overwrites. No configurable duplicate key rejection.
**Correct choice**: **serde-saphyr** — configurable `DuplicateKeyPolicy::Error` (default), fastest (294ms), no unsafe.

### 2. JTD Rust Crate: Not suitable as primary validator
**ChatGPT report implies**: `jtd` crate could be the validation backend
**Reality**: `jtd` crate is stable but inactive (last release Jan 2021, ~604 downloads/month). Our IR has extensions (regex, oneOf, mapRest) that JTD cannot express.
**Correct approach**: Build custom validator; use `jtd` crate only for optional JTD export compliance checking.

### 3. apollo-compiler: Accepts custom directives
**ChatGPT report raises concern**: apollo-compiler "may reject custom directives"
**Reality**: apollo-compiler does NOT reject custom directives — validation checks spec rules, not directive whitelist.
**Decision still valid**: Use apollo-parser (lighter, raw CST, no extra validation layer).

### 4. serde_yaml: Confirmed deprecated
**ChatGPT report says**: "serde_yaml is explicitly deprecated/unmaintained"
**Confirmed**: Archived March 2024 by dtolnay. Ecosystem migrating.

## Dependency Version Summary

| Dependency | Version | Purpose |
|---|---|---|
| `apollo-parser` | 0.8.5 | GraphQL SDL parsing |
| `serde-saphyr` | 0.0.21+ | YAML parsing (strict dup keys) |
| `serde_json` | 1.x | JSON parsing |
| `thiserror` | 2.x | Library error types |
| `anyhow` | 1.x | Application error handling |
| `miette` | 7.6+ | Rich diagnostics |
| `clap` | 4.x | CLI argument parsing |
| `indexmap` | 2.x | Ordered maps (preserves field order) |
| `reqwest` | 0.12+ | HTTP registry fetching |
| `lru` | 0.12+ | LRU memory cache |
| `regex` | 1.x | Pattern validation |
| `serde` | 1.x | Serialization framework |

## Dev Dependencies

| Dependency | Version | Purpose |
|---|---|---|
| `proptest` | 1.11+ | Property-based testing |
| `insta` | 1.47+ | Snapshot testing |
| `criterion` | 0.5+ | Benchmarks |
| `cargo-nextest` | 0.9+ | Fast test runner |
