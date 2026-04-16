# Rust Project Tooling & Best Practices (2026)

> Research by OpenCode librarian, 2026-04-15

## Error Handling

| Crate | Version | Use Case |
|---|---|---|
| `thiserror` | 2.x | Libraries (typed errors for callers) |
| `anyhow` | 1.x | Applications (context-rich errors) |
| `miette` | 7.6+ | Diagnostics (rich error display + thiserror integration) |

## Testing Tools

| Tool | Version | Stable? | Use Case |
|---|---|---|---|
| **cargo-nextest** | v0.9.132 | ✅ | Fast test runner (3x faster), MSRV 1.91 |
| **proptest** | v1.11.0 | ✅ | Property-based testing, MSRV 1.85 |
| **insta** | v1.47.2 | ✅ | Snapshot testing + `cargo insta review` |
| **cargo-fuzz** | v0.13.1 | Nightly only | Fuzzing with libFuzzer, Unix-only |
| **cargo-llvm-cov** | v0.6+ | ✅ | LLVM coverage reports |

## Clippy Configuration

```toml
[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "deny"
unused_crate_dependencies = "warn"
unused_results = "deny"
missing_debug_implementations = "deny"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "warn"
todo = "warn"
```

## CI (GitHub Actions)

```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    components: rustfmt, clippy
- uses: Swatinem/rust-cache@v2
  with:
    prefix-key: "v0-rust"
    cache-on-failure: true
```

## Profiling

- **criterion** v0.5+ — Statistically meaningful microbenchmarks
- **cargo-flamegraph** v0.6.11 — Flamegraph profiling

## Release

- **cargo-release** — Version bumping, tagging, CHANGELOG updates
- `shared-version = true` in workspace
- Semver: patch (fix), minor (feature), major (breaking)

## Workspace Pattern

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.dependencies]
thiserror = "2"
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
indexmap = "2"
```

## docs.rs Configuration

```toml
[package.metadata.docs.rs]
default-target = "x86_64-unknown-linux-gnu"
```

> Note: docs.rs now only builds default target unless `targets` specified.

## References

- cargo-nextest: https://nexte.st/
- proptest: https://crates.io/crates/proptest
- insta: https://insta.rs/
- cargo-fuzz: https://rust-fuzz.github.io/book/cargo-fuzz.html
- cargo-llvm-cov: https://github.com/taiki-e/cargo-llvm-cov
- dtolnay/rust-toolchain: https://github.com/dtolnay/rust-toolchain
- Swatinem/rust-cache: https://github.com/Swatinem/rust-cache
