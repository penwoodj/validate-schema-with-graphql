# JTD Export

## Overview

This document defines the JTD (JSON Type Definition) export capability for `graphql-ish-schema-validator`, which compiles the internal IR to JTD-compatible JSON.

The export is intentionally lossy for schema features that JTD cannot represent. The exporter reports which features were dropped or approximated, allowing users to understand the fidelity of the exported JTD schema.

**Key principle**: Export only the JTD-representable subset. Do not attempt to encode non-JTD features in JTD's limited form system.

## IR to JTD Form Mapping

### Mapping Table

| IR Variant | JTD Form | Notes |
|------------|-----------|-------|
| `Any` | `{}` (Empty schema) | Accepts any value |
| `Scalar(String)` | `{"type": "string"}` | Built-in string type |
| `Scalar(Boolean)` | `{"type": "boolean"}` | Built-in boolean type |
| `Scalar(Int)` | `{"type": "int32"}` or `{"type": "float64"}` | JTD numeric types |
| `Scalar(Float)` | `{"type": "float64"}` | JTD float64 type |
| `Scalar(Timestamp)` | `{"type": "timestamp"}` | JTD built-in timestamp |
| `Scalar(Custom)` | `{"type": "string"}` | Lose pattern constraint |
| `Enum` | `{"enum": ["a", "b", "c"]}` | JTD enum is list of strings |
| `Array` | `{"elements": <elements_schema>}` | JTD elements form |
| `Object` | `{"properties": {...}, "optionalProperties": {...}, "additionalProperties": true/false}` | JTD properties form |
| `Map` | `{"values": <values_schema>}` | JTD values form |
| `DiscriminatedUnion` | `{"discriminator": "field", "mapping": {...}}` | JTD discriminator form |
| `Ref` | `{"ref": "TypeName"}` | JTD ref form |
| `OneOf` | **Cannot export** | JTD has no oneOf (only discriminator) |

### Detailed Mappings

#### Any

**IR**:
```rust
Schema::Any
```

**JTD**:
```json
{}
```

#### Scalar Types

**IR**:
```rust
Schema::Scalar(ScalarSchema {
    kind: ScalarKind::String,
    pattern: None,
})
```

**JTD**:
```json
{
  "type": "string"
}
```

**IR**:
```rust
Schema::Scalar(ScalarSchema {
    kind: ScalarKind::Boolean,
    pattern: None,
})
```

**JTD**:
```json
{
  "type": "boolean"
}
```

**IR**:
```rust
Schema::Scalar(ScalarSchema {
    kind: ScalarKind::Int,
    pattern: None,
})
```

**JTD** (choice: int32 or float64):
```json
{
  "type": "int32"
}
```

**IR**:
```rust
Schema::Scalar(ScalarSchema {
    kind: ScalarKind::Float,
    pattern: None,
})
```

**JTD**:
```json
{
  "type": "float64"
}
```

**IR**:
```rust
Schema::Scalar(ScalarSchema {
    kind: ScalarKind::Timestamp,
    pattern: None,
})
```

**JTD**:
```json
{
  "type": "timestamp"
}
```

#### Custom Scalar with Pattern

**IR**:
```rust
Schema::Scalar(ScalarSchema {
    kind: ScalarKind::Custom("Email".to_string()),
    pattern: Some(Regex::new(r"^[^@]+@[^@]+$").unwrap()),
})
```

**JTD** (lossy - pattern dropped):
```json
{
  "type": "string"
}
```

**Warning**: Pattern constraint is lost. Exporter should emit a warning:
```
Warning: Scalar 'Email' has @pattern constraint that cannot be represented in JTD.
Exported as {"type": "string"} without pattern validation.
```

#### Enum

**IR**:
```rust
Schema::Enum(EnumSchema {
    values: vec!["PENDING".into(), "RUNNING".into(), "SUCCEEDED".into()],
})
```

**JTD**:
```json
{
  "enum": ["PENDING", "RUNNING", "SUCCEEDED"]
}
```

#### Array

**IR**:
```rust
Schema::Array(ArraySchema {
    elements: Box::new(Schema::Scalar(ScalarSchema {
        kind: ScalarKind::String,
        pattern: None,
    })),
})
```

**JTD**:
```json
{
  "elements": {
    "type": "string"
  }
}
```

#### Object

**IR**:
```rust
Schema::Object(ObjectSchema {
    required: vec![
        ("host".to_string(), SchemaRef("String".to_string())),
        ("port".to_string(), SchemaRef("Int".to_string())),
    ],
    optional: vec![
        ("timeout".to_string(), SchemaRef("Float".to_string())),
    ],
    additional_policy: AdditionalPolicy::Reject, // @closed
})
```

**JTD**:
```json
{
  "properties": {
    "host": {
      "type": "string"
    },
    "port": {
      "type": "int32"
    }
  },
  "optionalProperties": {
    "timeout": {
      "type": "float64"
    }
  },
  "additionalProperties": false
}
```

**IR** (with `@open` or default open mode):
```rust
Schema::Object(ObjectSchema {
    required: vec![],
    optional: vec![],
    additional_policy: AdditionalPolicy::AllowAny, // @open
})
```

**JTD**:
```json
{
  "additionalProperties": true
}
```

#### Map

**IR**:
```rust
Schema::Map(MapSchema {
    values: Box::new(Schema::Scalar(ScalarSchema {
        kind: ScalarKind::String,
        pattern: None,
    })),
})
```

**JTD**:
```json
{
  "values": {
    "type": "string"
  }
}
```

#### Discriminated Union

**IR**:
```rust
Schema::DiscriminatedUnion(DiscriminatedUnionSchema {
    discriminator: "type".to_string(),
    mapping: vec![
        ("agent".to_string(), SchemaRef("AgentStep".to_string())),
        ("tool".to_string(), SchemaRef("ToolStep".to_string())),
    ],
})
```

**JTD**:
```json
{
  "discriminator": "type",
  "mapping": {
    "agent": {
      "properties": { /* AgentStep fields */ },
      "required": ["prompt"]
    },
    "tool": {
      "properties": { /* ToolStep fields */ },
      "required": ["name"]
    }
  }
}
```

#### Ref

**IR**:
```rust
Schema::Ref(RefSchema {
    name: "WorkflowDocument".to_string(),
})
```

**JTD**:
```json
{
  "ref": "workflow/1.0.0/WorkflowDocument"
}
```

Note: Ref names are namespaced by schema_id and version in JTD exports.

## Features That Cannot Be Exported Losslessly

### @mapRest (AllowSchema)

**IR**:
```rust
Schema::Object(ObjectSchema {
    required: vec![
        ("global_config_path".to_string(), SchemaRef("String".to_string())),
    ],
    optional: vec![],
    additional_policy: AdditionalPolicy::AllowSchema(Box::new(Schema::Ref(RefSchema {
        name: "ModelDefinition".to_string(),
    }))),
})
```

**JTD approximation**:
```json
{
  "properties": {
    "global_config_path": {
      "type": "string"
    }
  },
  "additionalProperties": true
}
```

**Problem**: JTD's `additionalProperties` is boolean only, not a schema. The `@mapRest` feature (unknown keys validated as ModelDefinition) cannot be represented.

**Warning**:
```
Warning: Object 'ModelsSection' uses @mapRest(value: ModelDefinition) which cannot be
represented in JTD (JTD only supports boolean additionalProperties).
Exported as {"additionalProperties": true} without validation of rest keys.
```

### @pattern on Scalars

**IR**:
```rust
Schema::Scalar(ScalarSchema {
    kind: ScalarKind::Custom("SemVer".to_string()),
    pattern: Some(Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+$").unwrap()),
})
```

**JTD**:
```json
{
  "type": "string"
}
```

**Problem**: JTD has no regex constraint. Pattern validation is lost.

**Warning**:
```
Warning: Scalar 'SemVer' has @pattern(regex: "^[0-9]+\.[0-9]+\.[0-9]+$") which cannot be
represented in JTD (JTD has no regex support).
Exported as {"type": "string"} without pattern validation.
```

### @default

**IR**:
```rust
Schema::Object(ObjectSchema {
    required: vec![],
    optional: vec![
        ("port".to_string(), SchemaRef("Int".to_string())),
    ],
    defaults: vec![
        ("port".to_string(), Value::Number(8080.into())),
    ],
    additional_policy: AdditionalPolicy::Reject,
})
```

**JTD**:
```json
{
  "optionalProperties": {
    "port": {
      "type": "int32"
    }
  },
  "additionalProperties": false
}
```

**Problem**: JTD has no default values. Default application is lost.

**Warning**:
```
Warning: Field 'port' has default value 8080 which cannot be represented in JTD
(JTD has no default values).
Exported without default; consumers must handle missing fields manually.
```

### OneOf Without Discriminator

**IR**:
```rust
Schema::OneOf(OneOfSchema {
    variants: vec![
        SchemaRef("AgentStep".to_string()),
        SchemaRef("ToolStep".to_string()),
    ],
})
```

**JTD**: **Cannot export**

**Problem**: JTD has no `oneOf` form. Only discriminator-based unions are supported.

**Error**:
```
Error: Union 'Step' is a shape-based @oneOf union which cannot be represented in JTD
(JTD only supports discriminator-based unions).
Export failed for this schema.
```

**Workaround**: Redefine as discriminated union in SDL:

```graphql
union Step @discriminator(field: "type") = AgentStep | ToolStep

input AgentStep @variant(tag: "agent") { ... }
input ToolStep @variant(tag: "tool") { ... }
```

### Number Range Constraints

**IR** (hypothical future extension):
```rust
Schema::Scalar(ScalarSchema {
    kind: ScalarKind::Int,
    pattern: None,
    min: Some(0),
    max: Some(100),
})
```

**JTD**: **Cannot export**

**Problem**: JTD has no `min`/`max` constraints.

**Warning**:
```
Warning: Scalar 'Port' has range constraint [0, 100] which cannot be represented in JTD
(JTD has no min/max constraints).
Exported as {"type": "int32"} without range validation.
```

## Export Error Handling

### Error Types

**File**: `crates/jtd-export/src/error.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Cannot export OneOf without discriminator")]
    OneOfNotSupported { union_name: String },

    #[error("Pattern constraint lost for scalar '{name}'")]
    PatternConstraintLost { name: String },

    #[error("MapRest feature lost for object '{name}'")]
    MapRestLost { name: String },

    #[error("Default value lost for field '{field}' in type '{type_name}'")]
    DefaultLost { field: String, type_name: String },

    #[error("Range constraint lost for scalar '{name}'")]
    RangeConstraintLost { name: String },
}
```

### Warning Accumulator

**File**: `crates/jtd-export/src/lib.rs`

```rust
pub struct ExportReport {
    pub jtd_schema: serde_json::Value,
    pub warnings: Vec<ExportWarning>,
    pub errors: Vec<ExportError>,
}

pub enum ExportWarning {
    PatternConstraintLost { name: String },
    MapRestLost { name: String },
    DefaultLost { field: String, type_name: String },
    RangeConstraintLost { name: String },
}

pub fn export_to_jtd(ir_schema: &Schema) -> Result<ExportReport, ExportError> {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let jtd_schema = export_schema_recursive(ir_schema, &mut warnings, &mut errors)?;

    if !errors.is_empty() {
        return Err(ExportError::OneOfNotSupported { /* details */ });
    }

    Ok(ExportReport {
        jtd_schema,
        warnings,
        errors,
    })
}
```

### Example Export Output

**SDL Input**:
```graphql
scalar SemVer @pattern(regex: "^[0-9]+\\.[0-9]+\\.[0-9]+$")

input ModelsSection @closed @mapRest(value: ModelDefinition) {
  global_config_path: String
}

input ModelDefinition @closed {
  name: String!
  provider: String!
}

union Step @oneOf = AgentStep | ToolStep

input AgentStep @closed {
  prompt: String!
}

input ToolStep @closed {
  tool: String!
}
```

**Export Result**:
```json
{
  "warnings": [
    {
      "code": "pattern_constraint_lost",
      "message": "Scalar 'SemVer' has @pattern constraint that cannot be represented in JTD. Exported as {\"type\": \"string\"} without pattern validation."
    },
    {
      "code": "maprest_lost",
      "message": "Object 'ModelsSection' uses @mapRest which cannot be represented in JTD. Exported with additionalProperties: true without validation of rest keys."
    }
  ],
  "errors": [
    {
      "code": "oneof_not_supported",
      "message": "Union 'Step' is a shape-based @oneOf union which cannot be represented in JTD. Export failed for this schema."
    }
  ]
}
```

## JSON Schema Export (Stretch Goal)

### Complexity Note

JSON Schema is significantly more expressive than JTD:

- Multiple drafts (2020-12, 2019-09, etc.)
- Many validation keywords (`pattern`, `minLength`, `minimum`, etc.)
- Composition keywords (`anyOf`, `oneOf`, `allOf`)
- Conditional validation (`if`, `then`, `else`)

This complexity makes a complete IR → JSON Schema export non-trivial.

### Recommended Approach

**Defer to future implementation**. Focus on JTD export first, then evaluate JSON Schema export based on:

1. User demand for JSON Schema
2. Feasibility of representing all IR features
3. Tooling support (jsonschema crate, schemars, etc.)

### Partial JSON Schema (Alternative)

If JSON Schema export is pursued, consider a **partial export** strategy:

- Export JTD-representable features as exact JSON Schema
- Use non-standard custom keywords for unsupported features
- Document limitations clearly

**Example**:
```json
{
  "type": "object",
  "properties": {
    "models": {
      "type": "object",
      "x-mapRest": { "$ref": "#/definitions/ModelDefinition" }
    }
  },
  "additionalProperties": false
}
```

The `x-mapRest` is a custom keyword (non-standard) that JSON Schema validators would ignore.

## Rust Struct Codegen (Stretch Goal)

### JTD Ecosystem Tools

JTD has an established codegen ecosystem:

- **`jtd-codegen`** - Generate TypeScript types from JTD
- **`jtd-to-go`** - Generate Go structs from JTD
- **`jtd-derive`** - Rust derive macro for JTD

### Recommended Approach

**Defer to future implementation**. Leverage existing JTD tooling rather than implementing Rust codegen directly:

1. Export IR → JTD JSON (this document's scope)
2. Use `jtd-codegen` or `jtd-derive` to generate Rust structs
3. Document the workflow for users

### Future Considerations

If Rust codegen is pursued in-house:

- Use `syn` crate to parse Rust structs
- Generate `#[derive(Serialize, Deserialize)]` structs
- Handle optional fields via `Option<T>`
- Represent enums as Rust enums
- Document limitations (pattern constraints, @mapRest)

## CLI Integration

### Export Subcommand

**File**: `crates/cli/src/main.rs`

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a document against a schema
    Validate {
        #[arg(short, long)]
        schema: PathBuf,

        #[arg(short, long)]
        document: PathBuf,
    },

    /// Export compiled schema to JTD JSON
    ExportJtd {
        #[arg(short, long)]
        schema: PathBuf,

        #[arg(short, long)]
        output: PathBuf,

        /// Continue export even if warnings occur
        #[arg(short, long)]
        allow_warnings: bool,
    },
}
```

### Example Usage

```bash
# Export schema to JTD
graphql-ish-schema-validator export-jtd \
  --schema schemas/workflow.graphql \
  --output schemas/workflow.jtd.json \
  --allow-warnings

# Output with warnings
Warning: Scalar 'SemVer' has @pattern constraint that cannot be represented in JTD.
✓ Exported to schemas/workflow.jtd.json (2 warnings, 0 errors)
```

## Cross-References

- **`01-ir-design.md`** - IR types (Schema, ScalarKind, EnumSchema, etc.) to map to JTD forms
- **`03-compiler-lowering.md`** - IR construction from SDL, which is then exported to JTD
- **`00-overview.md`** - Project scope (JTD export is a secondary feature)
- **`08-project-structure.md`** - JTD export crate (`jtd-export`) placement

## Open Questions / Decisions Needed

1. **Ref Namespace Strategy**: How to namespace refs in JTD exports? Options: `{schema_id}/{version}/{type_name}`, `{type_name}`, or customizable via CLI flag.

2. **Warning vs Error**: Should unsupported features cause errors (blocking export) or warnings (allow export with fidelity loss)? Plan proposes errors for OneOf, warnings for others.

3. **Int Scalar Type**: Should IR `Scalar(Int)` map to JTD `int32` or `float64`? Both are valid, but `int32` is more precise for integers.

4. **Custom Keywords for JSON Schema**: Should JSON Schema export use custom keywords (`x-*`) to preserve unsupported features? Non-standard but may be useful.

5. **Codegen Priority**: Is Rust codegen higher priority than JSON Schema export? Both are stretch goals, but codegen may have more demand.

## Research Links

### JTD Specification
- [RFC 8927 - JSON Type Definition](https://datatracker.ietf.org/doc/html/rfc8927)
- [JTD forms reference](https://jsontypedef.com/docs/jtd-in-5-minutes/)
- [JTD discriminator semantics](https://jsontypedef.com/docs/form-definitions/#discriminator)

### JTD Tools
- [jtd-codegen TypeScript](https://github.com/jsontypedef/json-typedef-codegen)
- [jtd-to-go](https://github.com/jsontypedef/jtd-to-go)
- [jtd-derive Rust](https://crates.io/crates/jtd-derive)

### JSON Schema (Stretch Goal)
- [JSON Schema Specification](https://json-schema.org/)
- [Draft 2020-12](https://json-schema.org/draft/2020-12/json-schema-core.html)
- [jsonschema Rust crate](https://crates.io/crates/jsonschema)
- [schemars Rust crate](https://crates.io/crates/schemars)
