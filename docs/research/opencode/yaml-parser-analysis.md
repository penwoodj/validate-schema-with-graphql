# YAML Parser Analysis for Rust (2026)

> Research by OpenCode librarian, 2026-04-15

## Verdict: USE serde-saphyr (replaces ChatGPT report's yaml-rust2 recommendation)

> ⚠️ **IMPORTANT**: The ChatGPT research report recommended `yaml-rust2` for strict duplicate key detection. Our 2026 research reveals this was wrong. `yaml-rust2` does NOT have configurable duplicate key rejection (it silently overwrites). `serde-saphyr` is the correct choice.

## Comparison Table

| Crate | Status | YAML | Duplicate Keys | Serde | Speed* | Safety |
|---|---|---|---|---|---|---|
| **serde-saphyr** | ✅ Very Active | 1.2 | ✅ Configurable (reject default) | ✅ | **294ms** ⚡ | No unsafe, panic-free |
| serde_yaml_bw | ✅ Active | 1.2+1.1 | ✅ Configurable (reject default) | ✅ | 702ms | No unsafe |
| serde_yaml | ❌ Deprecated | 1.1 | Silent overwrite | ✅ | 477ms | unsafe-libyaml |
| serde_yaml_ng | ✅ Active | 1.1 only | Silent overwrite | ✅ | 470ms | Migrating to libyaml-safer |
| yaml_serde | ✅ Active (YAML org) | 1.1 | Silent overwrite | ✅ | ~477ms | libyaml-rs |
| yaml-rust2 | ✅ Basic maint. | 1.2 | Silent overwrite | ❌ | Unknown | Pure Rust |
| saphyr | ✅ Active | 1.2 | Anchors detected, maps silent | ⏳ Coming | Pure Rust |

\* 25MB YAML file benchmark

## Why serde-saphyr Wins

1. **Configurable duplicate key rejection by default** — Errors on duplicate keys, critical for kubeconform-like strict mode
2. **Fastest** — 294ms vs 470-770ms for alternatives
3. **No unsafe code** — Pure Rust via saphyr-parser
4. **Panic-free** — Safe for adversarial input
5. **Full serde integration** — Direct deserialization, canonical `Value` support
6. **YAML 1.2 compliant** — Modern spec version
7. **Active maintenance** — Frequent releases

## Duplicate Key Configuration

```rust
use serde_saphyr::{from_str_with_options, options};

let opts = serde_saphyr::options! {
    duplicate_keys: serde_saphyr::options::DuplicateKeyPolicy::Error,
};

let value: serde_saphyr::Value = from_str_with_options(yaml_str, opts)?;
```

Policies: `Error` (default), `FirstWins`, `LastWins`

## Usage Pattern for Validator

```toml
[dependencies]
serde = "1"
serde-saphyr = "0.0.21"  # Check latest version on crates.io
serde_json = "1"
```

```rust
// Parse YAML with strict duplicate key detection
fn parse_yaml_strict(input: &str) -> Result<Value, ParseError> {
    let opts = serde_saphyr::options! {
        duplicate_keys: DuplicateKeyPolicy::Error,
    };
    let value: serde_saphyr::Value = serde_saphyr::from_str_with_options(input, opts)?;
    Ok(convert_to_canonical_value(value))
}
```

## ChatGPT Report Correction Required

The second ChatGPT research report says:
> "yaml-rust2 project explicitly states it now errors on duplicate keys as part of spec compliance"

**This is incorrect.** yaml-rust2 uses `HashMap` which silently overwrites. The correct crate for strict duplicate key detection is **serde-saphyr**.

## References

- serde-saphyr on docs.rs: https://docs.rs/serde-saphyr/latest/serde_saphyr/
- serde-saphyr benchmarks (25MB file): documented in crate README
- yaml-rust2: https://github.com/Ethiraric/yaml-rust2
