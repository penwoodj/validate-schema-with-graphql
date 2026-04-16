# Testing Strategy

## Overview

This document defines the comprehensive testing strategy for `graphql-ish-schema-validator`, covering unit tests, integration tests, property-based tests, fuzz tests, snapshot tests, coverage targets, and performance benchmarks.

The goal is to achieve high confidence in correctness, crash resistance, and performance regression detection through a layered testing pyramid.

## Testing Pyramid

```
         /\
        /  \
       / Fuzz \          3-5% of effort
      /________\         Crash resistance
     /          \
    / Property  \      10-15% of effort
   /____________\     Invariant checking
  /              \
 /   Integration  \    25-30% of effort
/________________\   End-to-end workflows
/                  \
/       Unit Tests   \   50-60% of effort
/____________________\  Per-module correctness
```

### Test Type Summary

| Test Type | Count | Effort | Focus | Tooling |
|-----------|-------|---------|--------|---------|
| Unit tests | Many per crate | High | Local correctness | `#[test]` |
| Integration tests | 10-20 major flows | Medium | End-to-end | `cargo test` + fixtures |
| Property tests | 5-10 key invariants | Medium | Randomized invariants | `proptest` |
| Fuzz tests | 3-5 entry points | Medium | Crash resistance | `cargo-fuzz` |
| Snapshot tests | All error outputs | Low | Regression | `insta` |
| Coverage | Single run | Low | Code coverage | `cargo-llvm-cov` |
| Benchmarks | 5-10 metrics | Low | Performance | `criterion` |

## Unit Test Coverage

Unit tests are defined per module using `#[cfg(test)]`:

### IR Construction and Serialization

**File**: `crates/ir/src/schema.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_schema_serializes_to_json() {
        let schema = Schema::Scalar(ScalarSchema {
            kind: ScalarKind::String,
            pattern: Some(Regex::new("^[a-z]+$").unwrap()),
        });
        let json = serde_json::to_value(&schema).unwrap();
        // Verify roundtrip
    }

    #[test]
    fn enum_schema_preserves_values() {
        // Test enum values order and serialization
    }

    #[test]
    fn object_schema_tracks_required_optional() {
        // Verify required vs optional property handling
    }
}
```

**Coverage targets**:
- All `Schema` variants serialize/deserialize correctly
- Optional fields (pattern, discriminator, etc.) handle `None` correctly
- Complex nested structures (arrays of objects, maps) roundtrip

### JsonPointer Operations

**File**: `crates/ir/src/pointer.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_escapes_tilde_properly() {
        let mut ptr = JsonPointer::root();
        ptr.push_segment("~");
        assert_eq!(ptr.to_string(), "/~0");
    }

    #[test]
    fn pointer_escapes_slash_properly() {
        let mut ptr = JsonPointer::root();
        ptr.push_segment("a/b");
        assert_eq!(ptr.to_string(), "/a~1b");
    }

    #[test]
    fn pointer_roundtrip_stable() {
        let segments = vec!["foo", "bar~", "/baz"];
        let mut ptr = JsonPointer::root();
        for s in segments {
            ptr.push_segment(s);
        }
        let rendered = ptr.to_string();
        let parsed = JsonPointer::parse(&rendered).unwrap();
        assert_eq!(ptr.segments(), parsed.segments());
    }

    #[test]
    fn pointer_equality_consistent() {
        let ptr1 = JsonPointer::parse("/foo/bar").unwrap();
        let ptr2 = JsonPointer::parse("/foo/bar").unwrap();
        let ptr3 = JsonPointer::parse("/foo/baz").unwrap();
        assert_eq!(ptr1, ptr2);
        assert_ne!(ptr1, ptr3);
    }
}
```

**Coverage targets**:
- All JSON Pointer escaping rules (RFC 6901)
- Root pointer rendering (empty string vs "/" based on spec)
- Equality comparisons consider segment order
- Invalid pointer parsing returns errors

### SDL Parsing

**File**: `crates/sdl-parser/src/lib.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_scalar_definition_parses() {
        let sdl = r#"
            scalar SemVer @pattern(regex: "^[0-9]+\.[0-9]+\.[0-9]+$")
        "#;
        let result = parse_sdl(sdl);
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.scalars.len(), 1);
        assert_eq!(ast.scalars[0].name, "SemVer");
    }

    #[test]
    fn unknown_directive_produces_error() {
        let sdl = "scalar Foo @unknownDirective";
        let result = parse_sdl(sdl);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("unknown directive"));
    }

    #[test]
    fn enum_definition_preserves_values() {
        let sdl = "enum Status { PENDING RUNNING SUCCEEDED FAILED }";
        let result = parse_sdl(sdl);
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.enums[0].values, vec![
            "PENDING", "RUNNING", "SUCCEEDED", "FAILED"
        ]);
    }
}
```

**Coverage targets**:
- Valid SDL for each construct (scalar, enum, input, union)
- Invalid SDL produces descriptive errors
- Directive parsing (@closed, @open, @pattern, @default, @oneOf, @discriminator)
- Duplicate type names detected
- Missing type references detected

### Lowering Rules

**File**: `crates/compiler/src/lib.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_with_pattern_lowered_correctly() {
        let sdl_scalar = SdlScalar {
            name: "Email".to_string(),
            directives: vec![SdlDirective::Pattern(Regex::new(r"^[^@]+@[^@]+$").unwrap())],
        };
        let ir_schema = lower_scalar(&sdl_scalar);
        assert!(matches!(ir_schema, Schema::Scalar(_)));
        if let Schema::Scalar(scalar) = ir_schema {
            assert!(scalar.pattern.is_some());
        }
    }

    #[test]
    fn input_object_with_required_optional_fields() {
        let sdl_input = SdlInputObject {
            name: "Config".to_string(),
            fields: vec![
                SdlField { name: "host".to_string(), r#type: "String".to_string(), required: true, default: None },
                SdlField { name: "port".to_string(), r#type: "Int".to_string(), required: false, default: None },
            ],
        };
        let ir_schema = lower_input_object(&sdl_input);
        // Verify required and optional property separation
    }

    #[test]
    fn union_with_discriminator_lowered_correctly() {
        let sdl_union = SdlUnion {
            name: "Step".to_string(),
            members: vec!["AgentStep".to_string(), "ToolStep".to_string()],
            directives: vec![SdlDirective::Discriminator("type".to_string())],
        };
        let ir_schema = lower_union(&sdl_union);
        assert!(matches!(ir_schema, Schema::DiscriminatedUnion(_)));
    }
}
```

**Coverage targets**:
- Each SDL construct maps to expected IR variant
- Directive semantics translate correctly
- Required vs optional field detection follows GraphQL rules
- Union with discriminator vs union with @oneOf produce different IR

### Validator Per-Variant Tests

**File**: `crates/validator/src/lib.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_string_accepts_string() {
        let value = Value::String("hello".to_string());
        let schema = Schema::Scalar(ScalarSchema { kind: ScalarKind::String, pattern: None });
        let result = validate(&value, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn scalar_string_rejects_number() {
        let value = Value::Number(42.into());
        let schema = Schema::Scalar(ScalarSchema { kind: ScalarKind::String, pattern: None });
        let result = validate(&value, &schema);
        assert!(result.is_err());
    }

    #[test]
    fn enum_accepts_known_value() {
        let value = Value::String("PENDING".to_string());
        let schema = Schema::Enum(EnumSchema { values: vec!["PENDING".into(), "RUNNING".into()] });
        let result = validate(&value, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn object_rejects_missing_required_field() {
        let value = json!({"host": "localhost"}); // Missing required "port"
        let schema = Schema::Object(ObjectSchema {
            required: vec!["host".into(), "port".into()],
            optional: vec![],
            additional_policy: AdditionalPolicy::Reject,
        });
        let result = validate(&value, &schema);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].message.contains("missing required property"));
    }

    #[test]
    fn array_validates_each_element() {
        let value = json!([1, 2, "three"]);
        let elements_schema = Schema::Scalar(ScalarSchema { kind: ScalarKind::Int, pattern: None });
        let schema = Schema::Array(ArraySchema { elements: Box::new(elements_schema) });
        let result = validate(&value, &schema);
        assert!(result.is_err());
        // Should error on third element being string
    }
}
```

**Coverage targets**:
- All IR variants validate correctly
- Type mismatches produce clear errors
- Required fields detection works
- Optional fields are validated when present
- Additional keys rejected based on policy

### Strict vs Open Mode

**File**: `crates/validator/src/mode.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_mode_rejects_unknown_keys() {
        let value = json!({"known": "value", "unknown": "extra"});
        let schema = Schema::Object(ObjectSchema {
            required: vec!["known".into()],
            optional: vec![],
            additional_policy: AdditionalPolicy::Reject,
        });
        let result = validate_with_mode(&value, &schema, ValidationMode::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn open_mode_accepts_unknown_keys() {
        let value = json!({"known": "value", "unknown": "extra"});
        let schema = Schema::Object(ObjectSchema {
            required: vec!["known".into()],
            optional: vec![],
            additional_policy: AdditionalPolicy::AllowAny,
        });
        let result = validate_with_mode(&value, &schema, ValidationMode::Open);
        assert!(result.is_ok());
    }

    #[test]
    fn closed_directive_overrides_open_mode() {
        // Even in open mode, @closed enforces no unknown keys
        let value = json!({"known": "value", "unknown": "extra"});
        let schema = Schema::Object(ObjectSchema {
            required: vec!["known".into()],
            optional: vec![],
            additional_policy: AdditionalPolicy::Reject, // @closed
        });
        let result = validate_with_mode(&value, &schema, ValidationMode::Open);
        assert!(result.is_err());
    }
}
```

**Coverage targets**:
- Strict mode rejects unknown keys
- Open mode accepts unknown keys
- @closed directive forces rejection regardless of mode
- @mapRest validates unknown keys against schema

### Registry Resolution

**File**: `crates/registry/src/local.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn local_registry_reads_from_disk() {
        let temp = TempDir::new().unwrap();
        let schema_dir = temp.path().join("schemas").join("workflow").join("1.0.0");
        fs::create_dir_all(&schema_dir).unwrap();
        fs::write(schema_dir.join("schema.graphql"), "scalar Foo").unwrap();

        let registry = LocalRegistry::new(temp.path().to_path_buf());
        let result = registry.get("workflow", "1.0.0");
        assert!(result.is_ok());
    }

    #[test]
    fn local_registry_returns_not_found_for_missing() {
        let temp = TempDir::new().unwrap();
        let registry = LocalRegistry::new(temp.path().to_path_buf());
        let result = registry.get("missing", "1.0.0");
        assert!(matches!(result, Err(RegistryError::NotFound(_))));
    }
}
```

**Coverage targets**:
- Local filesystem resolution works
- HTTP registry fetching with timeout
- Composite registry tries sources in order
- Disk cache prevents duplicate fetches

## Integration Test Fixtures

### Fixture Categories

#### Minimal Valid Schemas

**File**: `tests/fixtures/schemas/minimal.graphql`

```graphql
"""
Minimal schema covering all IR variants.
"""
scalar String
scalar Int
scalar Float
scalar Boolean

enum Status {
  PENDING
  RUNNING
  SUCCEEDED
  FAILED
}

input Config @closed {
  host: String!
  port: Int
  timeout: Float
  enabled: Boolean
}

input Items {
  elements: [String!]!
}

input Mapping {
  values: String!
}
```

#### Complex Schema

**File**: `tests/fixtures/schemas/workflow.graphql`

The full workflow schema from the ChatGPT report, modeling:
- Nested objects
- Arrays with element schemas
- Unions (@oneOf)
- Enums
- Custom scalars with @pattern
- @closed and @mapRest directives

See `/home/jon/code/graphql-ish-schema-validator/docs/research/chatgpt/second-deep-research-report-chatgpt.md` lines 112-322 for the complete SDL.

#### Edge Cases

**File**: `tests/fixtures/schemas/edge-cases.graphql`

```graphql
"""
Edge cases for validation.
"""
# Deeply nested objects (depth 10+)
input DeepLevel1 { level2: DeepLevel2 }
input DeepLevel2 { level3: DeepLevel3 }
# ... (continue to DeepLevel10)

# Large arrays (1000+ elements)
input LargeArray {
  items: [String!]!
}

# Recursive references (should fail semantic validation)
input Recursive {
  recursive: Recursive
}

# Ambiguous oneOf (two variants both match)
input VariantA { field1: String }
input VariantB { field1: String }

union Ambiguous @oneOf = VariantA | VariantB
```

#### Invalid Documents for Snapshot Tests

**File**: `tests/fixtures/documents/invalid/type-mismatch.json`

```json
{
  "host": 123,
  "port": "should-be-number"
}
```

Expected output snapshot includes:
- instancePath: `/host`
- schemaPath: `/properties/host`
- message: "Expected string, got number"

**File**: `tests/fixtures/documents/invalid/missing-required.yaml`

```yaml
# Missing required field "host"
port: 8080
```

## Property-Based Testing Strategy

### Arbitrary Value Generator

**File**: `fuzz/fuzz_targets/validator.rs` (for proptest integration)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn validator_terminates_for_all_values(value in any_value()) {
        let schema = Schema::Scalar(ScalarSchema {
            kind: ScalarKind::Any,
            pattern: None,
        });
        let _ = validate(&value, &schema);
        // No panic = pass
    }
}
```

**Arbitrary Value Strategy**:

```rust
use proptest::strategy::Strategy;

fn any_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|n| Value::Number(n.into())),
        ".*".prop_map(Value::String),
        prop::collection::vec(any_value(), 0..10).prop_map(Value::Array),
        prop::collection::hash_map(".*", any_value(), 0..10)
            .prop_map(|m| Value::Object(m.into_iter().collect())),
    ]
}
```

### Arbitrary Schema Generator

```rust
fn any_schema(max_depth: usize) -> impl Strategy<Value = Schema> {
    let leaf = prop_oneof![
        Just(Schema::Any),
        Just(Schema::Scalar(ScalarSchema {
            kind: ScalarKind::String,
            pattern: None,
        })),
    ];

    if max_depth == 0 {
        leaf
    } else {
        prop_oneof![
            leaf,
            prop::collection::vec(any_schema(max_depth - 1), 0..3)
                .prop_map(|elements| Schema::Array(ArraySchema {
                    elements: Box::new(elements.into_iter().next().unwrap()),
                })),
        ]
    }
}
```

### Property Tests

#### Property: Validator Terminates

```rust
#[test]
fn validate_never_panics_for_any_value_schema_pair() {
    proptest!(|(value in any_value(), schema in any_schema(5))| {
        let _ = validate(&value, &schema);
        // Pass if no panic
    });
}
```

#### Property: Strict is Subset of Open

```rust
#[test]
fn strict_validate_is_subset_of_open_validate() {
    proptest!(|(value in any_value(), schema in any_schema(5))| {
        let strict_result = validate_with_mode(&value, &schema, ValidationMode::Strict);
        let open_result = validate_with_mode(&value, &schema, ValidationMode::Open);

        // If strict passes, open must pass
        if strict_result.is_ok() {
            assert!(open_result.is_ok(), "Strict passed but open failed");
        }
    });
}
```

#### Property: JsonPointer Roundtrip

```rust
#[test]
fn json_pointer_roundtrip_stable() {
    proptest!(|(segments in prop::collection::vec(".*", 0..10))| {
        let mut ptr = JsonPointer::root();
        for s in &segments {
            ptr.push_segment(s);
        }
        let rendered = ptr.to_string();
        let parsed = JsonPointer::parse(&rendered).unwrap();
        assert_eq!(ptr, parsed);
    });
}
```

#### Property: IR Serialization Roundtrip

```rust
#[test]
fn schema_serialization_roundtrip_stable() {
    proptest!(|(schema in any_schema(5))| {
        let json = serde_json::to_value(&schema).unwrap();
        let deserialized: Schema = serde_json::from_value(json).unwrap();
        assert_eq!(schema, deserialized);
    });
}
```

## Fuzzing Strategy

### Fuzz Targets

#### Fuzz Target 1: SDL Parser

**File**: `fuzz/fuzz_targets/sdl_parser.rs`

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(sdl) = std::str::from_utf8(data) {
        let _ = parse_sdl(sdl); // Should never panic
    }
});
```

**Goal**: Crash resistance on arbitrary SDL input.

#### Fuzz Target 2: YAML Parser

**File**: `fuzz/fuzz_targets/yaml_parser.rs`

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(yaml) = std::str::from_utf8(data) {
        let _ = parse_yaml_to_value(yaml); // Should never panic
    }
});
```

**Goal**: Crash resistance on arbitrary YAML input.

#### Fuzz Target 3: Full Compile + Validate

**File**: `fuzz/fuzz_targets/validator.rs`

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        // Try to split into SDL + document (naive split by "---")
        let parts: Vec<&str> = input.splitn(2, "---").collect();
        if parts.len() == 2 {
            let sdl = parts[0];
            let document = parts[1];

            if let Ok(schema) = compile_sdl_to_ir(sdl) {
                if let Ok(value) = parse_yaml_to_value(document) {
                    let _ = validate(&value, &schema); // Should never panic
                }
            }
        }
    }
});
```

**Goal**: Crash resistance on arbitrary (SDL, YAML) pairs.

### Fuzz Corpus Management

```bash
# Build fuzz targets
cargo fuzz build

# Run with initial corpus
cargo fuzz run sdl_parser fuzz/sdl_parser/corpus

# Minimize corpus
cargo fuzz tmin sdl_parser -- -runs=10000

# Add known interesting cases
cp interesting_case.graphql fuzz/sdl_parser/corpus/
```

## Snapshot Testing

### Error Message Snapshots

**File**: `tests/integration/snapshot_tests.rs`

```rust
#[test]
fn snapshot_type_mismatch_error() {
    let schema = compile_sdl("input Config { host: String! }").unwrap();
    let value = json!({"host": 123});
    let result = validate(&value, &schema);
    assert!(result.is_err());

    let errors = result.unwrap_err();
    insta::assert_snapshot!(format_errors(&errors));
}
```

**Snapshot file**: `tests/integration/snapshots/snapshot_tests__snapshot_type_mismatch_error.snap`

```insta
---
source: integration/snapshot_tests.rs
expression: format_errors(&errors)
---
Error: type mismatch

  × host: Expected string, got number

   ╭────[/root/input/Config/properties/host]
   │
 1 │ host: String!
   ·     ──┬──
   ·       ╰── type: String
   ╰────

  instancePath: /host
  schemaPath: /properties/host

  hint: Provide a string value, e.g., "localhost"
```

### Snapshot Review Workflow

```bash
# Run tests with review mode
insta review

# CI rejects unreviewed snapshots
cargo insta test --review --unreferenced=delete
```

## Coverage Targets

### Tooling

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Run coverage
cargo llvm-cov --html

# Generate report
cargo llvm-cov --lcov --output-path lcov.info
```

### Coverage Configuration

**File**: `.cargo/config.toml`

```toml
[env]
CARGO_INCREMENTAL = "0"
RUSTFLAGS = "-Cinstrument-coverage"
```

### Coverage Targets

| Metric | Target | Tool |
|--------|--------|------|
| Line coverage | ≥ 80% | cargo-llvm-cov |
| Function coverage | ≥ 90% | cargo-llvm-cov |
| Branch coverage | ≥ 75% | cargo-llvm-cov |
| Uncovered lines | Documented | Manual review |

### CI Gate

**File**: `.github/workflows/coverage.yml`

```yaml
name: Coverage

on: [push, pull_request]

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@cargo-llvm-cov
      - run: cargo llvm-cov --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v3
        with:
          files: lcov.info
          fail_ci_if_error: true
          minimum_coverage: 80
```

## Performance Testing

### Benchmark Categories

## Gap Fix: Benchmark Targets

Specific performance targets for key operations to ensure acceptable performance.

### Target Metrics

| Operation | Target | Tool |
|------------|--------|------|
| SDL compile (100-type schema) | < 10ms | criterion |
| Validation (10K-node document) | < 50ms | criterion |
| Registry cache hit | < 1μs | criterion |
| Strict vs open overhead | < 5% | criterion |

### Benchmark 1: SDL Compilation

**File**: `benches/compilation.rs`

```rust
fn bench_compile_100_types(c: &mut Criterion) {
    // Generate 100-type schema
    let sdl = generate_large_schema(100);

    c.bench_function("compile_100_types", |b| {
        b.iter(|| compile_sdl(black_box(&sdl)))
    });
}
```

**Target**: < 10ms for 100-type schema.

### Benchmark 2: Validation Performance

**File**: `benches/validation.rs`

```rust
fn bench_validate_10k_nodes(c: &mut Criterion) {
    let schema = compile_sdl(include_str!("../fixtures/schemas/workflow.graphql")).unwrap();
    let document = generate_large_document(10_000); // 10K nodes

    c.bench_function("validate_10k_nodes", |b| {
        b.iter(|| validate(black_box(&document), black_box(&schema)))
    });
}
```

**Target**: < 50ms for 10K-node document.

### Benchmark 3: Registry Cache Hit

**File**: `benches/registry.rs`

```rust
fn bench_cache_hit(c: &mut Criterion) {
    let registry = CachedRegistry::new(/* cache size */);

    // Warm cache
    let _ = registry.get("workflow", "1.0.0");

    c.bench_function("cache_hit", |b| {
        b.iter(|| registry.get(black_box("workflow"), black_box("1.0.0")))
    });
}
```

**Target**: < 1μs for cache hit.

### Benchmark 4: Strict vs Open Overhead

**File**: `benches/mode.rs`

```rust
fn bench_mode_overhead(c: &mut Criterion) {
    let schema = compile_sdl("input Config { host: String! }").unwrap();
    let value = json!({"host": "localhost", "extra": "key"});

    let mut group = c.benchmark_group("mode_overhead");
    group.bench_function("strict", |b| {
        b.iter(|| validate_with_mode(black_box(&value), black_box(&schema), ValidationMode::Strict))
    });
    group.bench_function("open", |b| {
        b.iter(|| validate_with_mode(black_box(&value), black_box(&schema), ValidationMode::Open))
    });
    group.finish();
}
```

**Target**: Mode selection overhead < 5% (strict vs open time difference).

### CI Performance Gate

**File**: `.github/workflows/bench.yml`

```yaml
name: Benchmark

on: [push, pull_request]

jobs:
  bench:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo bench -- --output-format bencher | tee benchmark.txt
      - uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: benchmark.txt
          alert-threshold: '200%'
          fail-on-alert: true
```

### Benchmark Categories

#### Benchmark 1: SDL Compilation

**File**: `benches/compilation.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_compile_sdl(c: &mut Criterion) {
    let sdl_small = "scalar String";
    let sdl_medium = include_str!("../tests/fixtures/schemas/workflow.graphql");
    let sdl_large = // Large schema with 1000+ types

    let mut group = c.benchmark_group("compile_sdl");
    group.bench_function("small", |b| {
        b.iter(|| compile_sdl(black_box(sdl_small)))
    });
    group.bench_function("medium", |b| {
        b.iter(|| compile_sdl(black_box(sdl_medium)))
    });
    group.bench_function("large", |b| {
        b.iter(|| compile_sdl(black_box(sdl_large)))
    });
    group.finish();
}

criterion_group!(benches, bench_compile_sdl);
criterion_main!(benches);
```

**Metrics**:
- Compile time per schema size (lines, types)
- Memory allocation profile (using criterion-dhat)

#### Benchmark 2: Validation

**File**: `benches/validation.rs`

```rust
fn bench_validate_document(c: &mut Criterion) {
    let schema = compile_sdl(include_str!("../tests/fixtures/schemas/workflow.graphql")).unwrap();
    let document_small = json!({"name": "test"});
    let document_large = include_str!("../tests/fixtures/documents/large-workflow.json");

    let mut group = c.benchmark_group("validate");
    group.bench_function("small_document", |b| {
        b.iter(|| validate(black_box(&document_small), black_box(&schema)))
    });
    group.bench_function("large_document", |b| {
        b.iter(|| validate(black_box(&document_large), black_box(&schema)))
    });
    group.finish();
}
```

**Metrics**:
- Validation time per document size (nodes, depth)
- Validation time per schema complexity

#### Benchmark 3: Strict vs Open Mode

```rust
fn bench_validation_mode(c: &mut Criterion) {
    let schema = compile_sdl("input Config { host: String! }").unwrap();
    let value = json!({"host": "localhost", "extra": "key"});

    let mut group = c.benchmark_group("validation_mode");
    group.bench_function("strict", |b| {
        b.iter(|| validate_with_mode(black_box(&value), black_box(&schema), ValidationMode::Strict))
    });
    group.bench_function("open", |b| {
        b.iter(|| validate_with_mode(black_box(&value), black_box(&schema), ValidationMode::Open))
    });
    group.finish();
}
```

**Metrics**: Mode selection overhead

#### Benchmark 4: Registry Cache Hit vs Miss

```rust
fn bench_registry_cache(c: &mut Criterion) {
    let registry = CachedRegistry::new(/* cache size */);
    let schema_id = "workflow";
    let version = "1.0.0";

    let mut group = c.benchmark_group("registry_cache");
    group.bench_function("cache_miss", |b| {
        b.iter(|| registry.get(black_box(schema_id), black_box(version)))
    });
    group.bench_function("cache_hit", |b| {
        // Warm cache
        let _ = registry.get(schema_id, version);
        b.iter(|| registry.get(black_box(schema_id), black_box(version)))
    });
    group.finish();
}
```

**Metrics**: Cache hit performance

### Regression Detection

**File**: `.github/workflows/bench.yml`

```yaml
name: Benchmark

on: [push, pull_request]

jobs:
  bench:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo bench -- --output-format bencher | tee benchmark.txt
      - uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: benchmark.txt
          alert-threshold: '200%'
          fail-on-alert: true
```

## Cross-References

- **`01-ir-design.md`** - IR types to test (Schema, JsonPointer, ScalarKind, etc.)
- **`02-sdl-parsing.md`** - SDL parsing tests (AST construction, directive parsing)
- **`03-compiler-lowering.md`** - Lowering tests (AST → IR mapping)
- **`04-validator-runtime.md`** - Validation tests (per-variant behavior, strict/open modes)
- **`05-registry-subsystem.md`** - Registry tests (resolution, caching)
- **`06-error-reporting.md`** - Error snapshot tests (formatting regression)
- **`08-project-structure.md`** - Workspace layout for test organization

## Open Questions / Decisions Needed

1. **Fuzz Target Granularity**: Should fuzz targets be separate (parser, validator) or combined (full pipeline)? Plan proposes 3 separate targets for focused crash detection.

2. **Snapshot Review CI**: Should snapshot updates require manual review in CI or allow auto-merge? Plan requires manual review via `insta review`.

3. **Coverage Threshold**: Is 80% line coverage achievable given complexity? Lower to 70% if needed, but target high initially.

4. **Benchmark Regression Threshold**: What percent change triggers alert? Plan proposes 200% threshold, but 10-20% may be more realistic.

5. **Property Test Complexity**: Should arbitrary schema generator support all IR variants or simplified subset? Plan uses simplified subset for performance.

## Research Links

### Testing Tools
- [proptest docs.rs](https://docs.rs/proptest/)
- [insta docs.rs](https://insta.rs/)
- [criterion docs.rs](https://bheisler.github.io/criterion.rs/book/)
- [cargo-fuzz book](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)

### Fuzzing Resources
- [cargo-fuzz docs](https://rust-fuzz.github.io/book/)
- [libFuzzer docs](https://llvm.org/docs/LibFuzzer.html)

### Coverage Tools
- [cargo-llvm-cov GitHub](https://github.com/taiki-e/cargo-llvm-cov)
- [Coverage guidelines](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html)

### Performance
- [Criterion user guide](https://bheisler.github.io/criterion.rs/book/user_guide/advanced_configuration.html)
- [criterion-dhat for heap profiling](https://github.com/AdamNiederl/criterion-dhat)

### Testing Best Practices
- [Rust testing patterns](https://matklad.github.io/2021/05/31/how-to-test-rust-code.html)
- [Property-based testing in Rust](https://blog.yoshuawuyts.com/property-based-testing-in-rust/)
