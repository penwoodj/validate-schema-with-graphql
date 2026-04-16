# IR Design

## Overview

The IR (Intermediate Representation) is the compilation target for SDL schemas. It is modeled on JTD's eight mutually-exclusive schema forms, extended with pragmatic features needed for YAML/JSON validation (`OneOf`, `Map` of-rest support).

The IR is designed to:
- Map cleanly to a Rust `enum`
- Be serializable (for caching and optional JTD export)
- Support recursive type definitions via named references
- Carry metadata for error reporting (schema paths, descriptions)

## Core Schema Enum

```rust
use indexmap::IndexMap;

/// JTD-like schema IR.
/// Mirrors RFC 8927's schema forms with pragmatic extensions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Schema {
    /// Accepts any value.
    Any,

    /// Scalar type with optional constraints.
    Scalar(ScalarKind),

    /// Enum with string values.
    Enum {
        values: Vec<String>,
    },

    /// Array with element schema.
    Array {
        elements: Box<Schema>,
    },

    /// Object with required/optional properties and additional key policy.
    Object {
        required: IndexMap<String, Box<Schema>>,
        optional: IndexMap<String, Box<Schema>>,
        additional: AdditionalPolicy,
    },

    /// Map where all values match a single schema (JTD "values" form).
    Map {
        values: Box<Schema>,
    },

    /// Discriminated union (tagged union).
    DiscriminatedUnion {
        discriminator: String,
        mapping: IndexMap<String, Box<Schema>>,
    },

    /// Shape-based union: exactly one variant must match.
    OneOf {
        variants: Vec<OneOfVariant>,
    },

    /// Named schema reference (for recursion).
    Ref {
        name: String,
    },
}

/// Policy for additional/unknown keys in objects.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AdditionalPolicy {
    /// Reject unknown keys.
    Reject,

    /// Allow any unknown key (any value type).
    AllowAny,

    /// Allow unknown keys, but values must match schema.
    /// This is the KEY extension beyond JTD for @mapRest.
    AllowSchema(Box<Schema>),
}

/// Variant in a OneOf union.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OneOfVariant {
    /// Human-readable label for diagnostics.
    pub label: String,

    /// Schema to validate against.
    pub schema: Box<Schema>,
}
```

## Scalar Kinds and Constraints

```rust
/// Built-in and custom scalar types with optional constraints.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScalarKind {
    /// Built-in string scalar.
    String {
        pattern: Option<String>,
    },

    /// Built-in boolean scalar.
    Boolean,

    /// Built-in integer scalar.
    Int {
        min: Option<i64>,
        max: Option<i64>,
    },

    /// Built-in float scalar.
    Float {
        min: Option<f64>,
        max: Option<f64>,
    },

    /// Built-in timestamp scalar (ISO 8601 string).
    Timestamp,

    /// Custom scalar with name and constraints.
    Custom {
        name: String,
        constraints: ScalarConstraints,
    },
}

/// Constraints applicable to string-like scalars.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ScalarConstraints {
    /// Regular expression pattern.
    pub pattern: Option<String>,

    /// Minimum length.
    pub min_length: Option<usize>,

    /// Maximum length.
    pub max_length: Option<usize>,
}
```

### Scalar Validation Rules

- **String**: Must be JSON string. Apply `pattern` via regex (if set).
- **Boolean**: Must be JSON boolean (`true`/`false`).
- **Int**: Must be JSON number with zero fractional part. Apply `min`/`max` bounds.
- **Float**: Must be JSON number. Apply `min`/`max` bounds.
- **Timestamp**: Must be JSON string matching ISO 8601 format.
- **Custom**: Delegates to named scalar definition in the schema bundle; inherits constraints.

## Schema Bundle (Type Registry)

```rust
/// Bundle of named schemas (type registry).
/// Enables recursion and cross-references.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchemaBundle {
    /// Map from type name to schema definition.
    pub schemas: IndexMap<String, Schema>,

    /// Root schema name (entry point).
    pub root_name: Option<String>,
}

impl SchemaBundle {
    /// Resolve a named schema reference.
    pub fn resolve(&self, name: &str) -> Option<&Schema> {
        self.schemas.get(name)
    }

    /// Detect recursive cycles (using DFS).
    pub fn detect_cycles(&self) -> Vec<Diagnostic>;
}
```

### Recursion Detection

Use depth-first search (DFS) to detect cycles in Ref chains:
1. Track visited schemas in the current path.
2. When encountering a `Ref`, check if already in path → cycle detected.
3. Report cycle with full path for debugging.

## JSON Pointer Type

JSON Pointers (RFC 6901) identify locations in documents and schemas.

```rust
/// JSON Pointer for stable path representation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct JsonPointer {
    segments: Vec<String>,
}

impl JsonPointer {
    /// Create empty pointer (root).
    pub fn root() -> Self;

    /// Push an object key segment.
    pub fn push_key(&mut self, key: &str);

    /// Push an array index segment.
    pub fn push_index(&mut self, index: usize);

    /// Pop the last segment.
    pub fn pop(&mut self) -> Option<String>;

    /// Clone and push (immutable operation).
    pub fn with_key(&self, key: &str) -> Self;

    /// Clone and push index.
    pub fn with_index(&self, index: usize) -> Self;

    /// Render as JSON Pointer string (RFC 6901).
    pub fn render(&self) -> String;

    /// Parse from JSON Pointer string.
    pub fn parse(s: &str) -> Result<Self, ParseError>;
}
```

### Pointer Encoding Rules

Follow RFC 6901:
- Prefix each segment with `/`
- Escape `~` as `~0`
- Escape `/` as `~1`
- Example: `/foo/bar[0]/baz` (for key "bar[0]")
- Example: `/foo~1bar` (for key "foo/bar")

## SchemaPath Generation Strategy

Every IR node gets a stable `SchemaPath` assigned during IR lowering. This enables error messages to point to the exact schema location.

### Path Assignment Rules

- **Root schema**: `/` (empty pointer)
- **Object properties**: `SchemaPath` + `/properties/{key}` or `/required/{key}`
- **Array elements**: `SchemaPath` + `/elements`
- **Map values**: `SchemaPath` + `/values`
- **OneOf variants**: `SchemaPath` + `/oneOf/{variant-index}`
- **DiscriminatedUnion mappings**: `SchemaPath` + `/mapping/{tag}`
- **Scalar constraints**: `SchemaPath` + `/constraints/{constraint-name}` (e.g., `/pattern`)

### Determinism Guarantee

The same SDL source must always produce identical `SchemaPath` values for each node. This ensures reproducible error reports.

### Example

Given SDL:

```graphql
input WorkflowDocument {
  name: String!
  steps: [Step!]!
}
```

IR `SchemaPath` values:
- Root: `/`
- `WorkflowDocument.name`: `/properties/name`
- `WorkflowDocument.steps`: `/properties/steps`
- Array elements: `/properties/steps/elements`

## @mapRest Extension

The `@mapRest` directive is the KEY extension beyond JTD. It models "fixed keys + rest" patterns common in YAML/JSON configurations.

### Mapping to IR

```graphql
input ModelsSection @mapRest(value: ModelDefinition) {
  global_config_path: String
  default_router: String
}
```

Lowers to:

```rust
Schema::Object {
    required: IndexMap::from([
        ("global_config_path".into(), Box::new(Schema::Scalar(ScalarKind::String { pattern: None })),
        ("default_router".into(), Box::new(Schema::Scalar(ScalarKind::String { pattern: None })),
    ]),
    optional: IndexMap::new(),
    additional: AdditionalPolicy::AllowSchema(Box::new(Schema::Ref {
        name: "ModelDefinition".into(),
    })),
}
```

### Semantics

- Known keys (required/optional) validate against their schemas.
- Unknown keys (not in required/optional) validate against the `AllowSchema` schema.
- This enables patterns like `models:` where fixed fields coexist with arbitrary model entries.

### Relation to JTD

JTD's `additionalProperties` is a boolean. This is a **schema-valued** extension. When exporting to JTD, schemas with `AllowSchema` must either:
- Fail export (recommended), or
- Use a workaround if the schema is `Any` (treat as `additionalProperties: true`).

## Serialization Format

The IR must be serializable for:
- Compiled schema caching (binary or JSON)
- Optional JTD export (JSON only)

### Binary Serialization (for caching)

Recommended: `bincode` for compact, fast binary format.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedIR {
    pub bundle: SchemaBundle,
    pub version: String, // Schema version or hash
}
```

### JSON Serialization (for JTD export)

Export to JTD JSON schema. Not all IR features map losslessly:
- `OneOf`: Represent as JTD `oneOf` if available in target JTD version
- `AllowSchema`: **Cannot** represent in JTD; must drop or error

Example JTD export (representable subset):

```json
{
  "definitions": {
    "WorkflowDocument": {
      "properties": {
        "name": { "type": "string" },
        "steps": {
          "elements": {
            "ref": "Step"
          }
        }
      },
      "required": ["name", "steps"],
      "additionalProperties": false
    }
  }
}
```

## Validation Context

During instance validation, the validator maintains:

```rust
/// Validation context threaded through recursive descent.
pub struct ValidationContext<'a> {
    /// Current instance path (JSON Pointer).
    pub instance_path: JsonPointer,

    /// Current schema path (JSON Pointer).
    pub schema_path: JsonPointer,

    /// Error accumulator.
    pub errors: &'a mut Vec<ValidationError>,

    /// Validation options (strict/open mode, etc.).
    pub options: ValidationOptions,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidationOptions {
    /// Strict mode: reject unknown keys, reject duplicate keys.
    pub strict: bool,

    /// Allow type coercion in non-strict mode.
    pub allow_coercion: bool,
}
```

### Error Structure

```rust
/// Validation error with JTD-style pointers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationError {
    /// JSON Pointer to location in validated document.
    pub instance_path: String,

    /// JSON Pointer to location in schema.
    pub schema_path: String,

    /// Error code for programmatic handling.
    pub code: ErrorCode,

    /// Human-readable message.
    pub message: String,

    /// Optional hint or remediation.
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ErrorCode {
    TypeMismatch,
    RequiredMissing,
    AdditionalKeyRejected,
    EnumMismatch,
    PatternMismatch,
    MinLengthViolation,
    MaxLengthViolation,
    MinValueViolation,
    MaxValueViolation,
    ArrayExpected,
    ObjectExpected,
    DiscriminatorMissing,
    DiscriminatorUnknown,
    OneOfAmbiguous,
    OneOfNoMatch,
    RefCycleDetected,
}
```

## Cross-Reference Links

- **[02-sdl-grammar.md](./02-sdl-grammar.md)**: SDL constructs that lower to IR (directives, types)
- **[03-compiler-lowering.md](./03-compiler-lowering.md)**: Rules for converting SDL AST to IR
- **[04-validator-runtime.md](./04-validator-runtime.md)**: How validator consumes IR
- **[05-error-reporting.md](./05-error-reporting.md)**: Error formatting using `instancePath`/`schemaPath`
- **[08-code-generation.md](./08-code-generation.md)**: IR to Rust structs, IR to JTD export

## Open Questions and Decisions Needed

1. **OneOf variant ordering**: Should `OneOf::variants` preserve SDL declaration order? (recommended: yes, for deterministic errors)
2. **SchemaPath vs JsonPointer**: Should we use the same type for both, or separate? (recommended: same type, consistent naming)
3. **AdditionalPolicy::AllowSchema**: Should it allow nested `AllowSchema`? (e.g., map of maps) - complexity vs utility tradeoff.
4. **Ref resolution depth limit**: Prevent infinite loops in malformed schemas even if no cycles (suggested: 1024 depth limit).
5. **Scalar constraints extensibility**: Should `ScalarConstraints` be an enum for future extensibility, or struct with `Option` fields? (recommended: struct for simplicity).
6. **Binary cache versioning**: How to detect cache invalidation? Schema hash vs timestamp vs explicit version field.

## Research Links

### JTD and IR Design
- RFC 8927 (JTD): https://datatracker.ietf.org/doc/html/rfc8927
- JTD validation errors guide: https://jsontypedef.com/docs/validation-errors/
- JTD schema forms documentation: https://jsontypedef.com/docs/jtd-forms/

### SDL to IR Mapping
- See "Compiler architecture and JTD-like IR" section in second research report for full lowering rules.
- See "Lowering rules from SDL to IR" section for mapping from SDL directives to IR.

### @mapRest Extension
- See "Map types and @mapRest" section in second research report for semantics and use cases.
- See example `ModelsSection` in "Example-driven SDL" section for complete SDL modeling of fixed + rest keys.

### JSON Pointer
- RFC 6901 (JSON Pointer): https://datatracker.ietf.org/doc/html/rfc6901
