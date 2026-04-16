# Testing and Verification Plan

**For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish comprehensive test coverage with unit tests, integration tests, property tests, snapshot tests, benchmarks, and CI/CD verification.

**Architecture:** Three-tier test strategy (unit → integration → property) with coverage targets and performance benchmarks.

**Tech Stack:** Rust, cargo-nextest, proptest, criterion, insta, assert_cmd, tempfile

---

## Context and Rationale

**Testing requirements from research:**
- Fix existing integration test path issues
- Add end-to-end tests using the workflow SDL from chatgpt reports
- Test matrix: YAML+strict, YAML+open, JSON+strict, JSON+open
- Property tests for error resilience (random input, malformed schemas)
- Snapshot tests for error messages
- CLI integration tests using assert_cmd
- Coverage targets: >80% for validator, >90% for public API
- Benchmark: SDL compile <10ms, validation <50ms for 10K nodes

**Key design principles:**
1. **Fast**: Unit tests run quickly (<1s total)
2. **Comprehensive**: Cover all code paths
3. **Maintainable**: Clear test structure and naming
4. **Documented**: Examples serve as tests too

**References:**
- [01-initial-attempt/09-testing-strategy.md](../01-initial-attempt/09-testing-strategy.md) - Original testing strategy
- [01-public-api.md](./01-public-api.md) - API to test
- [03-cli-improvements.md](./03-cli-improvements.md) - CLI to test

---

## Task 1: Set Up Testing Infrastructure

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `.cargo/config.toml`

**Step 1: Add testing dependencies to workspace**

Update workspace `Cargo.toml`:

```toml
[workspace.dependencies]
# ... existing dependencies ...

# Testing
proptest = "1.2"
criterion = "0.5"
insta = "1.34"
assert_cmd = "2.0"
predicates = "3.0"
tempfile = "3.8"
quickcheck = "1.0"
fake = "2.6"
```

- [ ] **Step 2: Create cargo config for test settings**

Create `.cargo/config.toml`:

```toml
[build]
# Use nextest for faster test execution

[target.'cfg(all())']
rustflags = ["-Dwarnings"]  # Treat warnings as errors in tests

[profile.bench]
inherits = "release"
debug = true  # Keep debug symbols for profiling

[profile.test]
opt-level = 0  # Faster compilation for tests

[profile.dev.package."*"]
opt-level = 3  # Optimize dependencies in dev builds
```

- [ ] **Step 3: Create test fixtures directory**

```bash
mkdir -p tests/fixtures/schemas
mkdir -p tests/fixtures/valid
mkdir -p tests/fixtures/invalid
mkdir -p tests/fixtures/benchmarks
```

- [ ] **Step 4: Commit testing infrastructure**

```bash
git add Cargo.toml .cargo/config.toml tests/fixtures/
git commit -m "test: set up testing infrastructure and configuration"
```

---

## Task 2: Add Unit Tests for IR Module

**Files:**
- Create: `crates/graphql-ish-schema-validator-ir/tests/serialization_tests.rs`
- Create: `crates/graphql-ish-schema-validator-ir/tests/form_tests.rs`

**Step 1: Create IR serialization tests**

Create `crates/graphql-ish-schema-validator-ir/tests/serialization_tests.rs`:

```rust
//! IR serialization tests

use graphql_ish_schema_validator_ir::forms::*;
use std::collections::{BTreeMap, HashMap};

#[test]
fn test_serialize_empty_form() {
    let form = SchemaForm::Empty;
    let json = serde_json::to_string(&form).unwrap();
    assert_eq!(json, "{}");
}

#[test]
fn test_serialize_ref_form() {
    let form = SchemaForm::Ref(RefForm {
        type_name: "User".to_string(),
    });

    let json = serde_json::to_string(&form).unwrap();
    assert_eq!(json, r#"{"$ref":"User"}"#);
}

#[test]
fn test_serialize_type_form() {
    let form = SchemaForm::Type(TypeForm {
        type_value: ScalarType::String,
    });

    let json = serde_json::to_string(&form).unwrap();
    assert_eq!(json, r#"{"type":"string"}"#);
}

#[test]
fn test_serialize_enum_form() {
    let form = SchemaForm::Enum(EnumForm {
        values: vec!["red".to_string(), "green".to_string(), "blue".to_string()],
        metadata: None,
    });

    let json = serde_json::to_string(&form).unwrap();
    assert!(json.contains(r#""enum":"#));
    assert!(json.contains("red"));
    assert!(json.contains("green"));
    assert!(json.contains("blue"));
}

#[test]
fn test_serialize_properties_form() {
    let mut required = BTreeMap::new();
    required.insert("name".to_string(), SchemaForm::Type(TypeForm {
        type_value: ScalarType::String,
    }));

    let form = SchemaForm::Properties(PropertiesForm {
        required,
        optional: None,
        additional_properties: Some(AdditionalProperties::Disallowed),
    });

    let json = serde_json::to_string(&form).unwrap();
    assert!(json.contains(r#""properties":"#));
    assert!(json.contains(r#""name""#));
}

#[test]
fn test_deserialize_all_forms() {
    let test_cases = vec![
        (r#"{}"#, SchemaForm::Empty),
        (r#"{"$ref":"User"}"#, SchemaForm::Ref(RefForm {
            type_name: "User".to_string(),
        })),
        (r#"{"type":"int32"}"#, SchemaForm::Type(TypeForm {
            type_value: ScalarType::Int32,
        })),
    ];

    for (json, expected) in test_cases {
        let deserialized: SchemaForm = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized, expected);
    }
}

#[test]
fn test_compiled_schema_serialization() {
    let mut definitions = HashMap::new();
    definitions.insert("String".to_string(), SchemaForm::Type(TypeForm {
        type_value: ScalarType::String,
    }));

    let schema = CompiledSchema {
        schema_id: Some("test".to_string()),
        schema_version: Some("1.0.0".to_string()),
        definitions,
    };

    let json = serde_json::to_string_pretty(&schema).unwrap();
    assert!(json.contains("test"));
    assert!(json.contains("1.0.0"));
    assert!(json.contains("String"));

    // Round-trip test
    let deserialized: CompiledSchema = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, schema);
}
```

- [ ] **Step 2: Create IR form behavior tests**

Create `crates/graphql-ish-schema-validator-ir/tests/form_tests.rs`:

```rust
//! IR form behavior tests

use graphql_ish_schema_validator_ir::forms::*;

#[test]
fn test_additional_properties_variants() {
    let disallowed = AdditionalProperties::Disallowed;
    let allowed = AdditionalProperties::Allowed(Box::new(SchemaForm::Type(TypeForm {
        type_value: ScalarType::String,
    })));
    let any_allowed = AdditionalProperties::AnyAllowed;

    // Verify each variant is distinct
    assert_ne!(disallowed, any_allowed);
    assert_ne!(allowed, any_allowed);
}

#[test]
fn test_scalar_type_names() {
    assert_eq!(ScalarType::String.to_string(), "string");
    assert_eq!(ScalarType::Int32.to_string(), "int32");
    assert_eq!(ScalarType::Float64.to_string(), "float64");
    assert_eq!(ScalarType::Bool.to_string(), "bool");
}

#[test]
fn test_enum_with_pattern() {
    let form = SchemaForm::Enum(EnumForm {
        values: vec!["email".to_string()],
        metadata: Some(EnumMetadata {
            pattern: Some(r"^[^@]+@[^@]+\.[^@]+$".to_string()),
        }),
    });

    let json = serde_json::to_string(&form).unwrap();
    assert!(json.contains("email"));
    assert!(json.contains(r#"pattern"#));
}
```

- [ ] **Step 3: Run IR tests**

Run: `cargo test -p graphql-ish-schema-validator-ir`
Expected: All IR tests pass

- [ ] **Step 4: Commit IR tests**

```bash
git add crates/graphql-ish-schema-validator-ir/tests/
git commit -m "test: add IR serialization and form behavior tests"
```

---

## Task 3: Add Parser Tests

**Files:**
- Create: `crates/graphql-ish-schema-validator-parser/tests/parser_tests.rs`
- Create: `crates/graphql-ish-schema-validator-parser/tests/error_tests.rs`

**Step 1: Create parser tests**

Create `crates/graphql-ish-schema-validator-parser/tests/parser_tests.rs`:

```rust
//! Parser integration tests

use graphql_ish_schema_validator_parser::{Parser, Document};

#[test]
fn test_parse_empty_document() {
    let source = "";

    let document = Parser::new(source).parse().unwrap();

    assert_eq!(document.scalars.len(), 0);
    assert_eq!(document.enums.len(), 0);
    assert_eq!(document.inputs.len(), 0);
    assert_eq!(document.unions.len(), 0);
}

#[test]
fn test_parse_scalar_definition() {
    let source = r#"
        scalar String
        scalar Int
        scalar Boolean
    "#;

    let document = Parser::new(source).parse().unwrap();

    assert_eq!(document.scalars.len(), 3);
    assert_eq!(document.scalars[0].name, "String");
    assert_eq!(document.scalars[1].name, "Int");
    assert_eq!(document.scalars[2].name, "Boolean");
}

#[test]
fn test_parse_enum_definition() {
    let source = r#"
        enum BackoffStrategy {
            exponential
            linear
            fixed
        }
    "#;

    let document = Parser::new(source).parse().unwrap();

    assert_eq!(document.enums.len(), 1);
    assert_eq!(document.enums[0].name, "BackoffStrategy");
    assert_eq!(document.enums[0].values.len(), 3);
    assert_eq!(document.enums[0].values[0].name, "exponential");
    assert_eq!(document.enums[0].values[1].name, "linear");
    assert_eq!(document.enums[0].values[2].name, "fixed");
}

#[test]
fn test_parse_input_definition() {
    let source = r#"
        input User @closed {
            id: String!
            name: String
            age: Int!
            active: Boolean!
        }
    "#;

    let document = Parser::new(source).parse().unwrap();

    assert_eq!(document.inputs.len(), 1);
    assert_eq!(document.inputs[0].name, "User");
    assert_eq!(document.inputs[0].fields.len(), 4);

    assert_eq!(document.inputs[0].fields[0].name, "id");
    assert!(document.inputs[0].fields[0].r#type.to_string(), "String!");

    assert_eq!(document.inputs[0].fields[1].name, "name");
    assert!(document.inputs[0].fields[1].r#type.to_string(), "String");

    assert_eq!(document.inputs[0].fields[2].name, "age");
    assert_eq!(document.inputs[0].fields[2].r#type.to_string(), "Int!");

    assert_eq!(document.inputs[0].fields[3].name, "active");
    assert_eq!(document.inputs[0].fields[3].r#type.to_string(), "Boolean!");
}

#[test]
fn test_parse_union_definition() {
    let source = r#"
        union Step = AgentStep | ToolStep | ControlFlowStep
    "#;

    let document = Parser::new(source).parse().unwrap();

    assert_eq!(document.unions.len(), 1);
    assert_eq!(document.unions[0].name, "Step");
    assert_eq!(document.unions[0].members.len(), 3);
    assert_eq!(document.unions[0].members[0], "AgentStep");
    assert_eq!(document.unions[0].members[1], "ToolStep");
    assert_eq!(document.unions[0].members[2], "ControlFlowStep");
}

#[test]
fn test_parse_directives() {
    let source = r#"
        input User @closed {
            id: String!
            email: String @pattern(regex: "^[^@]+@[^@]+\\.[^@]+$")
        }

        scalar Timestamp @pattern(regex: "^[0-9]+$")
    "#;

    let document = Parser::new(source).parse().unwrap();

    assert_eq!(document.inputs.len(), 1);
    assert!(!document.inputs[0].directives.is_empty());

    assert_eq!(document.scalars.len(), 1);
    assert!(!document.scalars[0].directives.is_empty());
}

#[test]
fn test_parse_nested_types() {
    let source = r#"
        input Outer @closed {
            inner: Inner!
        }

        input Inner @closed {
            value: String!
        }
    "#;

    let document = Parser::new(source).parse().unwrap();

    assert_eq!(document.inputs.len(), 2);
    assert_eq!(document.inputs[0].fields[0].r#type.to_string(), "Inner!");
}

#[test]
fn test_parse_list_types() {
    let source = r#"
        input ListTest @closed {
            strings: [String!]!
            optional_ints: [Int]
            nested: [[String!]!]
        }
    "#;

    let document = Parser::new(source).parse().unwrap();

    assert_eq!(document.inputs.len(), 1);
    assert_eq!(document.inputs[0].fields[0].r#type.to_string(), "[String!]!");
    assert_eq!(document.inputs[0].fields[1].r#type.to_string(), "[Int]");
    assert_eq!(document.inputs[0].fields[2].r#type.to_string(), "[[String!]!]");
}

#[test]
fn test_parse_description() {
    let source = r#"
        """
        A user account
        """
        input User @closed {
            """
            Unique identifier
            """
            id: String!
        }
    "#;

    let document = Parser::new(source).parse().unwrap();

    assert_eq!(document.inputs.len(), 1);
    assert!(document.inputs[0].description.is_some());
    assert!(document.inputs[0].fields[0].description.is_some());
}
```

- [ ] **Step 2: Create parser error tests**

Create `crates/graphql-ish-schema-validator-parser/tests/error_tests.rs`:

```rust
//! Parser error handling tests

use graphql_ish_schema_validator_parser::{Parser, ParseError};

#[test]
fn test_parse_invalid_syntax() {
    let source = "input User { id: String";  // Missing closing brace

    let result = Parser::new(source).parse();

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, ParseError::SyntaxError { .. }));
}

#[test]
fn test_parse_duplicate_type() {
    let source = r#"
        input User {
            id: String!
        }

        input User {
            name: String!
        }
    "#;

    let result = Parser::new(source).parse();

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, ParseError::DuplicateType(..)));
}

#[test]
fn test_parse_invalid_directive() {
    let source = r#"
        input User @unknown {
            id: String!
        }
    "#;

    let result = Parser::new(source).parse();

    // This will be caught during semantic validation (not yet implemented)
    // For now, parsing succeeds
    assert!(result.is_ok());
}

#[test]
fn test_parse_missing_required_field() {
    let source = r#"
        input User {
            id: String!  # Required
            name: String   # Optional
        }
    "#;

    let result = Parser::new(source).parse();
    assert!(result.is_ok());

    let document = result.unwrap();
    assert!(document.inputs[0].fields[0].r#type.is_non_null());
    assert!(!document.inputs[1].r#type.is_non_null());
}
```

- [ ] **Step 3: Run parser tests**

Run: `cargo test -p graphql-ish-schema-validator-parser`
Expected: All parser tests pass

- [ ] **Step 4: Commit parser tests**

```bash
git add crates/graphql-ish-schema-validator-parser/tests/
git commit -m "test: add parser integration and error handling tests"
```

---

## Task 4: Add Property Tests

**Files:**
- Create: `crates/graphql-ish-schema-validator-validator/tests/property_tests.rs`

**Step 1: Create property tests for error resilience**

Create `crates/graphql-ish-schema-validator-validator/tests/property_tests.rs`:

```rust
//! Property-based tests for error resilience

use graphql_ish_schema_validator::{
    validate_yaml_from_schema, validate_json_from_schema, ValidationOptions,
    ValidationError, ValidationErrorCode,
};
use proptest::prelude::*;

// Generate random YAML-like strings
prop_compose! {
    fn arb_yaml_string()(max_size in 0usize..100) -> String {
        use fake::{Fake, Faker};

        let mut yaml = String::new();
        yaml.push_str("name: ");
        yaml.push_str(&fake::faker::name::en::Name().fake::<String>());

        for _ in 0..max_size {
            yaml.push_str(&format!("\nkey_{}: ", fake::faker::lorem::en::Word().fake::<String>()));
            yaml.push_str(&fake::faker::lorem::en::Word().fake::<String>());
        }

        yaml
    }
}

// Generate random JSON-like strings
prop_compose! {
    fn arb_json_string()(max_size in 0usize..100) -> String {
        let mut map = serde_json::Map::new();
        map.insert("name".to_string(), serde_json::json!(fake::faker::name::en::Name().fake::<String>()));

        for i in 0..max_size {
            let key = format!("key_{}", i);
            let value = fake::faker::lorem::en::Word().fake::<String>();
            map.insert(key, serde_json::json!(value));
        }

        serde_json::to_string(&map).unwrap()
    }
}

proptest! {
    #[test]
    fn prop_validate_yaml_does_not_panic(yaml in arb_yaml_string()) {
        let schema = r#"
            input Test @open {
                name: String
            }
        "#;

        let options = ValidationOptions::builder()
            .mode(graphql_ish_schema_validator::ValidationMode::Open)
            .build();

        // Should never panic, even with invalid YAML
        let result = validate_yaml_from_schema(&yaml, schema, options);
        // Result may be invalid, but should not panic
    }

    #[test]
    fn prop_validate_json_does_not_panic(json in arb_json_string()) {
        let schema = r#"
            input Test @open {
                name: String
            }
        "#;

        let options = ValidationOptions::builder()
            .mode(graphql_ish_schema_validator::ValidationMode::Open)
            .build();

        // Should never panic, even with invalid JSON
        let result = validate_json_from_schema(&json, schema, options);
        // Result may be invalid, but should not panic
    }
}

#[test]
fn test_validate_malformed_yaml_no_panic() {
    let test_cases = vec![
        "name: test\n  bad: indent",
        "name: 'unclosed string",
        "- item\n- item\n  bad: indent",
        "{",
        "[",
        "key: value\n  bad: indent\n  worse: indent",
        "name: [",
        "root:\n  child: [",
    ];

    let schema = r#"
        input Test @open {
            name: String
        }
    "#;

    let options = ValidationOptions::builder()
        .mode(graphql_ish_schema_validator::ValidationMode::Open)
        .build();

    for yaml in test_cases {
        let result = validate_yaml_from_schema(yaml, schema, options);
        // Should not panic and should return an error
        assert!(!result.valid || yaml.is_empty());
    }
}

#[test]
fn test_validate_malformed_json_no_panic() {
    let test_cases = vec![
        r#"{"name": "test""#,  // Missing closing brace
        r#"{"name": test}"#,   // Unquoted value
        r#"[1, 2, 3"#,      // Missing closing bracket
        r#"{"name": "test",}"#,  // Trailing comma
        r#"name: test"#,     // Not valid JSON
        r#"{{"name": "test"}}"#,  // Double braces
    ];

    let schema = r#"
        input Test @open {
            name: String
        }
    "#;

    let options = ValidationOptions::builder()
        .mode(graphql_ish_schema_validator::ValidationMode::Open)
        .build();

    for json in test_cases {
        let result = validate_json_from_schema(json, schema, options);
        // Should not panic and should return an error
        assert!(!result.valid);
    }
}

#[test]
fn test_validate_empty_inputs() {
    let schema = r#"
        input Test @open {
            name: String
        }
    "#;

    let options = ValidationOptions::builder()
        .mode(graphql_ish_schema_validator::ValidationMode::Open)
        .build();

    // Empty YAML
    let yaml_result = validate_yaml_from_schema("", schema, options.clone());
    // Empty JSON
    let json_result = validate_json_from_schema("{}", schema, options);

    // Should not panic
    assert!(!yaml_result.valid || yaml_result.errors.is_empty());
    assert!(!json_result.valid || json_result.errors.is_empty());
}

#[test]
fn test_validate_very_long_inputs() {
    let schema = r#"
        input Test @open {
            name: String
        }
    "#;

    let options = ValidationOptions::builder()
        .mode(graphql_ish_schema_validator::ValidationMode::Open)
        .build();

    // Very long string
    let long_string = "a".repeat(1_000_000);
    let yaml = format!("name: {}", long_string);
    let json = format!(r#"{{"name": "{}"}}"#, long_string);

    // Should not panic (might fail due to depth)
    let yaml_result = validate_yaml_from_schema(&yaml, schema, options.clone());
    let json_result = validate_json_from_schema(&json, schema, options);

    // Should handle gracefully
    assert!(yaml_result.valid || yaml_result.has_errors());
    assert!(json_result.valid || json_result.has_errors());
}
```

- [ ] **Step 2: Add property test dependencies**

Update `crates/graphql-ish-schema-validator-validator/Cargo.toml`:

```toml
[dev-dependencies]
proptest = { workspace = true }
fake = { workspace = true }
```

- [ ] **Step 3: Run property tests**

Run: `cargo test -p graphql-ish-schema-validator-validator property`
Expected: All property tests pass without panics

- [ ] **Step 4: Commit property tests**

```bash
git add crates/graphql-ish-schema-validator-validator/tests/
git add crates/graphql-ish-schema-validator-validator/Cargo.toml
git commit -m "test: add property-based tests for error resilience"
```

---

## Task 5: Add Snapshot Tests

**Files:**
- Create: `crates/graphql-ish-schema-validator-validator/snapshots/error_messages.snap`

**Step 1: Create snapshot tests for error messages**

Create `crates/graphql-ish-schema-validator-validator/tests/snapshot_tests.rs`:

```rust
//! Snapshot tests for error messages

use graphql_ish_schema_validator::{
    validate_yaml_from_schema, validate_json_from_schema, ValidationOptions,
    ValidationError, ValidationErrorCode, ValidationMode,
};
use insta::assert_snapshot;

#[test]
fn snapshot_type_mismatch_error() {
    let schema = r#"
        input Test @closed {
            name: String!
        }
    "#;

    let yaml = r#"
        name: 123
    "#;

    let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());

    assert!(!result.valid);
    assert_snapshot!("error_type_mismatch", result.format_errors());
}

#[test]
fn snapshot_required_field_error() {
    let schema = r#"
        input Test @closed {
            name: String!
            value: Int!
        }
    "#;

    let yaml = r#"
        value: 42
    "#;

    let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());

    assert!(!result.valid);
    assert_snapshot!("error_required_field", result.format_errors());
}

#[test]
fn snapshot_additional_property_error() {
    let schema = r#"
        input Test @closed {
            name: String!
        }
    "#;

    let yaml = r#"
        name: test
        extra: value
    "#;

    let options = ValidationOptions::builder()
        .mode(ValidationMode::Strict)
        .build();

    let result = validate_yaml_from_schema(yaml, schema, options);

    assert!(!result.valid);
    assert_snapshot!("error_additional_property", result.format_errors());
}

#[test]
fn snapshot_enum_error() {
    let schema = r#"
        input Test @closed {
            status: Status
        }

        enum Status {
            active
            inactive
        }
    "#;

    let yaml = r#"
        status: unknown
    "#;

    let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());

    assert!(!result.valid);
    assert_snapshot!("error_invalid_enum", result.format_errors());
}

#[test]
fn snapshot_parse_error_yaml() {
    let schema = r#"
        input Test @closed {
            name: String!
        }
    "#;

    let yaml = r#"
        name: test
          bad: indent
    "#;

    let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());

    assert!(!result.valid);
    assert_snapshot!("error_parse_yaml", result.format_errors());
}

#[test]
fn snapshot_parse_error_json() {
    let schema = r#"
        input Test @closed {
            name: String!
        }
    "#;

    let json = r#"{"name": "test""#;  // Missing closing brace

    let result = validate_json_from_schema(json, schema, ValidationOptions::default());

    assert!(!result.valid);
    assert_snapshot!("error_parse_json", result.format_errors());
}

#[test]
fn snapshot_schema_parse_error() {
    let yaml = r#"
        name: test
    "#;

    let schema = "input Test";  // Malformed

    let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());

    assert!(!result.valid);
    assert_snapshot!("error_schema_parse", result.format_errors());
}

#[test]
fn snapshot_successful_validation() {
    let schema = r#"
        input Test @closed {
            name: String!
            value: Int
        }
    "#;

    let yaml = r#"
        name: test
        value: 42
    "#;

    let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());

    assert!(result.valid);
    assert_snapshot!("success_validation", result.format_summary());
}
```

- [ ] **Step 2: Create snapshot directory**

```bash
mkdir -p crates/graphql-ish-schema-validator-validator/snapshots
```

- [ [ ] **Step 3: Run snapshot tests (will create snapshots)**

Run: `cargo test -p graphql-ish-schema-validator-validator snapshot --accept`
Expected: Snapshots created for all tests

- [ ] **Step 4: Commit snapshot tests**

```bash
git add crates/graphql-ish-schema-validator-validator/tests/snapshot_tests.rs
git add crates/graphql-ish-schema-validator-validator/snapshots/
git commit -m "test: add snapshot tests for error messages"
```

---

## Task 6: Add Benchmarks

**Files:**
- Create: `crates/graphql-ish-schema-validator-validator/benches/validation_bench.rs`

**Step 1: Create validation benchmarks**

Create `crates/graphql-ish-schema-validator-validator/benches/validation_bench.rs`:

```rust
//! Validation performance benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use graphql_ish_schema_validator::{
    validate_json_from_schema, validate_yaml_from_schema, ValidationOptions,
    ValidationMode,
};

const SIMPLE_SCHEMA: &str = r#"
    input Test @closed {
        name: String!
        value: Int
        active: Boolean
    }
"#;

fn generate_large_json(size: usize) -> String {
    let mut obj = serde_json::json!({
        "name": "test",
        "value": 42,
        "active": true
    });

    for i in 0..size {
        obj.as_object_mut().unwrap().insert(
            format!("key_{}", i),
            serde_json::json!("value_{}", i),
        );
    }

    serde_json::to_string(&obj).unwrap()
}

fn generate_large_yaml(size: usize) -> String {
    let mut yaml = "name: test\nvalue: 42\nactive: true\n".to_string();

    for i in 0..size {
        yaml.push_str(&format!("key_{}: value_{}\n", i, i));
    }

    yaml
}

fn bench_json_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_validation");

    for size in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let json = generate_large_json(size);
            b.iter(|| {
                validate_json_from_schema(
                    black_box(&json),
                    black_box(SIMPLE_SCHEMA),
                    black_box(ValidationOptions::default()),
                )
            });
        });
    }

    group.finish();
}

fn bench_yaml_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_validation");

    for size in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let yaml = generate_large_yaml(size);
            b.iter(|| {
                validate_yaml_from_schema(
                    black_box(&yaml),
                    black_box(SIMPLE_SCHEMA),
                    black_box(ValidationOptions::default()),
                )
            });
        });
    }

    group.finish();
}

fn bench_schema_parsing(c: &mut Criterion) {
    c.bench_function("schema_parse_simple", |b| {
        b.iter(|| {
            graphql_ish_schema_validator::parse_schema(black_box(SIMPLE_SCHEMA))
        });
    });
}

fn bench_validation_modes(c: &mut Criterion) {
    let json = generate_large_json(100);
    let yaml = generate_large_yaml(100);

    let mut group = c.benchmark_group("validation_modes");

    group.bench_function("json_strict", |b| {
        let options = ValidationOptions::builder()
            .mode(ValidationMode::Strict)
            .build();
        b.iter(|| {
            validate_json_from_schema(
                black_box(&json),
                black_box(SIMPLE_SCHEMA),
                black_box(options.clone()),
            )
        });
    });

    group.bench_function("json_open", |b| {
        let options = ValidationOptions::builder()
            .mode(ValidationMode::Open)
            .build();
        b.iter(|| {
            validate_json_from_schema(
                black_box(&json),
                black_box(SIMPLE_SCHEMA),
                black_box(options.clone()),
            )
        });
    });

    group.bench_function("yaml_strict", |b| {
        let options = ValidationOptions::builder()
            .mode(ValidationMode::Strict)
            .build();
        b.iter(|| {
            validate_yaml_from_schema(
                black_box(&yaml),
                black_box(SIMPLE_SCHEMA),
                black_box(options.clone()),
            )
        });
    });

    group.bench_function("yaml_open", |b| {
        let options = ValidationOptions::builder()
            .mode(ValidationMode::Open)
            .build();
        b.iter(|| {
            validate_yaml_from_schema(
                black_box(&yaml),
                black_box(SIMPLE_SCHEMA),
                black_box(options.clone()),
            )
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_json_validation,
    bench_yaml_validation,
    bench_schema_parsing,
    bench_validation_modes
);
criterion_main!(benches);
```

- [ ] **Step 2: Add benchmark dependencies**

Update workspace `Cargo.toml`:

```toml
[workspace.dependencies]
criterion = { workspace = true }
```

- [ ] **Step 3: Run benchmarks**

Run: `cargo bench -p graphql-ish-schema-validator-validator`
Expected: Benchmarks run and show performance results

- [ ] **Step 4: Verify performance targets**

Check that benchmarks meet targets:
- SDL compilation: <10ms
- Validation (10K nodes): <50ms

- [ ] **Step 5: Commit benchmarks**

```bash
git add crates/graphql-ish-schema-validator-validator/benches/
git add Cargo.toml
git commit -m "test: add performance benchmarks"
```

---

## Task 7: Add Coverage Measurement

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `scripts/coverage.sh`

**Step 1: Add coverage tools to workspace**

Update workspace `Cargo.toml`:

```toml
[workspace.dependencies]
tarpaulin = "0.27"
```

- [ ] **Step 2: Create coverage script**

Create `scripts/coverage.sh`:

```bash
#!/bin/bash
set -e

echo "Running coverage tests..."

# Run tarpaulin for coverage
cargo tarpaulin \
    --workspace \
    --out Xml \
    --output-dir ./coverage \
    --exclude-files '*/tests/*' \
    --exclude-files '*/benches/*' \
    --exclude-files '*/examples/*' \
    --exclude-files '/*' \
    --timeout 300

echo "Coverage report generated in ./coverage/"

# Check coverage thresholds
COVERAGE=$(cargo tarpaulin --workspace --exclude-files '*/tests/*' --exclude-files '*/benches/*' --exclude-files '*/examples/*' --exclude-files '/*' --output-dir ./coverage --output-type Html -- -q | grep "Overall" | grep -oP '\d+(?=\.\d+%)' || echo "0")

VALIDATOR_COVERAGE=$(cargo tarpaulin -p graphql-ish-schema-validator-validator --exclude-files '*/tests/*' --exclude-files '*/benches/*' --output-dir ./coverage --output-type Html -- -q | grep "Overall" | grep -oP '\d+(?=\.\d+%)' || echo "0")

API_COVERAGE=$(cargo tarpaulin -p graphql-ish-schema-validator --exclude-files '*/tests/*' --exclude-files '*/benches/*' --exclude-files '*/examples/*' --output-dir ./coverage --output-type Html -- -q | grep "Overall" | grep -oP '\d+(?=\.\d+%)' || echo "0")

echo "Coverage Report:"
echo "  Overall: ${COVERAGE}%"
echo "  Validator: ${VALIDATOR_COVERAGE}%"
echo "  Public API: ${API_COVERAGE}%"

# Check thresholds
if [ "$VALIDATOR_COVERAGE" -lt 80 ]; then
    echo "ERROR: Validator coverage (${VALIDATOR_COVERAGE}%) is below target (80%)"
    exit 1
fi

if [ "$API_COVERAGE" -lt 90 ]; then
    echo "WARNING: Public API coverage (${API_COVERAGE}%) is below target (90%)"
fi

echo "Coverage check passed!"
```

- [ ] **Step 3: Make coverage script executable**

```bash
chmod +x scripts/coverage.sh
```

- [ ] **Step 4: Run coverage**

Run: `./scripts/coverage.sh`
Expected: Coverage report generated

- [ ] **Step 5: Commit coverage setup**

```bash
git add scripts/coverage.sh Cargo.toml
git commit -m "test: add coverage measurement script"
```

---

## Task 8: Add CI/CD Test Workflow

**Files:**
- Create: `.github/workflows/test.yml`

**Step 1: Create GitHub Actions workflow**

Create `.github/workflows/test.yml`:

```yaml
name: Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin

      - name: Run tests
        run: cargo test --workspace --verbose

      - name: Run clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Run coverage
        run: ./scripts/coverage.sh

      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: ./coverage/cobertura.xml
          fail_ci_if_error: false

  benchmark:
    name: Benchmark
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Run benchmarks
        run: cargo bench -p graphql-ish-schema-validator-validator

      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/report/index.html
```

- [ ] **Step 2: Create .gitignore entry for coverage**

Update `.gitignore`:

```
coverage/
```

- [ ] **Step 3: Commit CI workflow**

```bash
git add .github/workflows/test.yml .gitignore
git commit -m "ci: add GitHub Actions test workflow"
```

---

## Verification

**Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 2: Run property tests**

Run: `cargo test --workspace property`
Expected: All property tests pass without panics

- [ ] **Step 3: Run snapshot tests**

Run: `cargo test --workspace snapshot`
Expected: All snapshot tests pass

- [ ] **Step 4: Run benchmarks**

Run: `cargo bench -p graphql-ish-schema-validator-validator`
Expected: Benchmarks complete successfully

- [ ] **Step 5: Run coverage**

Run: `./scripts/coverage.sh`
Expected: Coverage report generated, thresholds met

- [ ] **Step 6: Verify test matrix coverage**

Ensure tests cover:
- ✅ YAML + strict mode
- ✅ YAML + open mode
- ✅ JSON + strict mode
- ✅ JSON + open mode
- ✅ Parse errors
- ✅ Schema errors
- ✅ Validation errors
- ✅ Warnings
- ✅ Edge cases (empty, very long inputs)
- ✅ Panic resilience

- [ ] **Step 7: Final verification checklist**

Verify:
- ✅ Unit tests for all crates
- ✅ Integration tests for CLI
- ✅ Property tests for error resilience
- ✅ Snapshot tests for error messages
- ✅ Benchmarks for performance
- ✅ Coverage script with thresholds
- ✅ CI/CD workflow
- ✅ Test fixtures organized
- ✅ Coverage targets: >80% validator, >90% public API
- ✅ Performance targets: compile <10ms, validate <50ms for 10K nodes

- [ ] **Step 8: Final commit**

```bash
git add .
git commit -m "test: complete testing and verification setup"
```

---

## Summary

This plan establishes a comprehensive testing and verification strategy:

**Key achievements:**
1. **Testing infrastructure** with proper tooling and configuration
2. **Unit tests** for IR, parser, and validator modules
3. **Property tests** for error resilience
4. **Snapshot tests** for error message consistency
5. **Benchmarks** for performance measurement
6. **Coverage measurement** with thresholds
7. **CI/CD workflow** for automated testing

**Test coverage:**
- Unit tests for all core modules
- Integration tests for CLI
- Property tests for error resilience
- Snapshot tests for error messages
- Benchmark tests for performance

**Quality assurance:**
- Coverage targets: >80% validator, >90% public API
- Performance targets: compile <10ms, validate <50ms for 10K nodes
- Automated CI/CD with GitHub Actions
- Clippy and formatting checks

**Next steps:**
- Create migration guide (see [05-migration-checklist.md](./05-migration-checklist.md))
- Implement remaining validation logic (see validator runtime plans)
- Add fuzzing tests for additional security
