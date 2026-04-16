# Error Reporting Design

## Overview

This document specifies the error reporting system for the GraphQL-ish schema validator. The design provides machine-readable JSON output for tooling integration and human-readable colored output for developers, with support for rich diagnostics via `miette` integration.

**Key characteristics:**
- **Dual format**: Machine-readable JSON and human-readable text
- **Rich context**: Includes paths, error codes, hints, and value snippets
- **Stable pointers**: JSON Pointer (RFC 6901) for consistent path representation
- **Tooling-friendly**: Integrates with `miette` for rich terminal output
- **CI-ready**: GitHub Actions format for automated workflows

## Error Data Model

### ValidationError Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Where in the YAML/JSON document the error occurred
    pub instance_path: JsonPointer,

    /// Where in the schema the error originated
    pub schema_path: JsonPointer,

    /// Machine-readable error code
    pub code: ErrorCode,

    /// Human-readable error message
    pub message: String,

    /// Optional remediation suggestion
    pub hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// All validation errors
    pub errors: Vec<ValidationError>,

    /// Root instance path (empty for top-level validation)
    pub instance_path: JsonPointer,

    /// Root schema path (empty for root schema)
    pub schema_path: JsonPointer,

    /// Whether validation succeeded
    pub is_valid: bool,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}
```

### ErrorCode Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Type mismatch between expected and actual value
    TypeMismatch,

    /// Required property missing from object
    RequiredPropertyMissing,

    /// Additional property rejected (strict mode)
    AdditionalPropertyRejected,

    /// Enum value not in allowed set
    EnumValueInvalid,

    /// String pattern mismatch
    PatternMismatch,

    /// OneOf: no variants matched
    OneOfNoMatch,

    /// OneOf: multiple variants matched (ambiguous)
    OneOfAmbiguous,

    /// Discriminated union: discriminator field missing
    DiscriminatorMissing,

    /// Discriminated union: discriminator value invalid
    DiscriminatorInvalid,

    /// Schema reference not found
    RefUnresolved,

    /// Recursive type detected
    RecursiveType,

    /// Number outside allowed range
    NumberOutOfRange,

    /// Array element validation failed
    ArrayItemInvalid,

    /// Map value validation failed
    MapValueInvalid,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap();
        write!(f, "{}", s.trim_matches('"'))
    }
}
```

**Error code semantics:**
- `TypeMismatch`: Expected `String`, found `Number`
- `RequiredPropertyMissing`: Object missing `name` field
- `AdditionalPropertyRejected`: Unknown key `extra_field` in strict mode
- `EnumValueInvalid`: Value `"unknown"` not in enum `[a, b, c]`
- `PatternMismatch`: String `"abc"` doesn't match `^\d+$`
- `OneOfNoMatch`: Value doesn't match any variant
- `OneOfAmbiguous`: Value matches 2+ variants
- `DiscriminatorMissing`: Object missing `type` field
- `DiscriminatorInvalid`: Discriminator value `"x"` not in mapping
- `RefUnresolved`: Schema reference `User` not found in bundle
- `RecursiveType`: Schema references itself without intermediate types
- `NumberOutOfRange`: Value `-1` below minimum `0`
- `ArrayItemInvalid`: Element at index `3` failed validation
- `MapValueInvalid`: Map value for key `"foo"` failed validation

## JSON Pointer Implementation

### JsonPointer Type

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JsonPointer {
    segments: Vec<String>,
}

impl JsonPointer {
    /// Create a new empty pointer
    pub fn new() -> Self {
        Self { segments: Vec::new() }
    }

    /// Create a pointer from a string (RFC 6901 format)
    pub fn from_string(s: &str) -> Result<Self, ParseError> {
        if s == "" {
            return Ok(Self::new());
        }

        if !s.starts_with('/') {
            return Err(ParseError::MissingLeadingSlash);
        }

        let segments: Vec<String> = s[1..]
            .split('/')
            .map(Self::decode_segment)
            .collect::<Result<_, _>>()?;

        Ok(Self { segments })
    }

    /// Encode a segment according to RFC 6901 escaping rules
    fn encode_segment(segment: &str) -> String {
        segment.replace('~', "~0").replace('/', "~1")
    }

    /// Decode a segment according to RFC 6901 escaping rules
    fn decode_segment(segment: &str) -> Result<String, ParseError> {
        let decoded = segment
            .replace("~1", "/")
            .replace("~0", "~");

        // Check for invalid escape sequences
        if decoded.contains("~") {
            return Err(ParseError::InvalidEscapeSequence);
        }

        Ok(decoded)
    }

    /// Push a new segment onto the pointer
    pub fn push(&mut self, segment: &str) {
        self.segments.push(segment.to_string());
    }

    /// Pop the last segment from the pointer
    pub fn pop(&mut self) -> Option<String> {
        self.segments.pop()
    }

    /// Convert to RFC 6901 string representation
    pub fn to_string(&self) -> String {
        if self.segments.is_empty() {
            String::new()
        } else {
            format!(
                "/{}",
                self.segments
                    .iter()
                    .map(Self::encode_segment)
                    .collect::<Vec<_>>()
                    .join("/")
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    MissingLeadingSlash,
    InvalidEscapeSequence,
}
```

### RFC 6901 Escaping Rules

- `~0` represents `~` (tilde)
- `~1` represents `/` (slash)
- `~` followed by anything else is invalid
- Empty segment is valid (e.g., `/foo//bar`)

**Examples:**

| Pointer segments | Encoded string | Decoded string |
|----------------|----------------|----------------|
| `["users", "0"]` | `/users/0` | `/users/0` |
| `["data", "foo/bar"]` | `/data/foo~1bar` | `/data/foo/bar` |
| `["a~b"]` | `/a~0b` | `/a~b` |
| `["a", "", "b"]` | `/a//b` | `/a//b` |

**Segment encoding examples:**

| Original segment | Encoded segment |
|-----------------|-----------------|
| `"foo"` | `"foo"` |
| `"foo/bar"` | `"foo~1bar"` |
| `"a~b"` | `"a~0b"` |
| `"~1"` | `"~01"` |
| `"/"` | `"~1"` |

## Machine-Readable Output Format

### JSON Error Array

```json
[
  {
    "instancePath": "/agentic_workflow/steps/read_config/input",
    "schemaPath": "/definitions/ToolStep/properties/input",
    "code": "type_mismatch",
    "message": "Expected object, got string",
    "hint": "ToolStep.input must be a map with tool-specific arguments. Example: { path: \"./workspace/model-config.yml\" }"
  },
  {
    "instancePath": "/models/global_config_path",
    "schemaPath": "/definitions/ModelsSection/properties/global_config_path",
    "code": "pattern_mismatch",
    "message": "String does not match pattern: ^/.*\\.yml$",
    "hint": "Path must be a .yml file"
  }
]
```

**Format specification:**
- `instancePath`: JSON Pointer string (RFC 6901)
- `schemaPath`: JSON Pointer string (RFC 6901)
- `code`: snake_case error code (matches `ErrorCode` enum)
- `message`: Human-readable description
- `hint`: Optional remediation suggestion

**JTD compatibility:**
- Matches JTD's `instancePath` + `schemaPath` convention
- Error codes are custom but follow similar naming patterns
- Field names match JTD's portable validation error format

## Human-Readable Output Format

### Terminal Output

```
✗ unified-workflow-schema.yml: 3 validation errors

Error 1:
  Code: type_mismatch
  Instance path: /agentic_workflow/steps/read_config/input
  Schema path: /definitions/ToolStep/properties/input

    Expected: object
    Found: string

    ToolStep.input must be a map with tool-specific arguments.
    Example: { path: "./workspace/model-config.yml" }

    Offending value:
    "this should be a map, not a string"

Error 2:
  Code: required_property_missing
  Instance path: /models
  Schema path: /definitions/ModelsSection/properties/default_router

    Required property 'default_router' is missing

    Add property: default_router

Error 3:
  Code: enum_value_invalid
  Instance path: /providers/lmstudio/config/requests/retry/backoff
  Schema path: /definitions/BackoffStrategy

    Invalid enum value 'fixed_backoff'. Expected one of: exponential, linear, fixed

    Use one of: exponential, linear, fixed

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Summary:
  Documents: 1
  Errors: 3
  Strict mode: true
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**Formatting features:**
- Color-coded error codes (red for errors, yellow for warnings)
- Multi-line with indentation
- Expected vs found values clearly separated
- Hints in italics
- Offending value snippets truncated to 100 chars
- Summary statistics at bottom

### GitHub Actions Format

```yaml
::error file=unified-workflow-schema.yml,line=42,endLine=45,title=type_mismatch::Expected object, got string at /agentic_workflow/steps/read_config/input
::error file=unified-workflow-schema.yml,line=100,title=required_property_missing::Required property 'default_router' is missing at /models
::error file=unified-workflow-schema.yml,line=155,endLine=160,title=enum_value_invalid::Invalid enum value 'fixed_backoff' at /providers/lmstudio/config/requests/retry/backoff
```

**Format specification:**
- `::error file=<file>,line=<start>,endLine=<end>,title=<code>::<message>`
- Line numbers derived from YAML/JSON source positions
- Title includes error code for filtering
- File-relative paths for CI display

## Error Context Enhancement

### Value Snippet Truncation

```rust
fn truncate_value(value: &Value, max_len: usize) -> String {
    let json = serde_json::to_string(value).unwrap();
    if json.len() <= max_len {
        json
    } else {
        format!("{}...", &json[..max_len])
    }
}
```

**Truncation rules:**
- Default max length: 100 characters
- Truncate at safe boundaries (e.g., after comma)
- Append ellipsis `...` to indicate truncation

### Nearby Lines for Position Context

```rust
fn get_nearby_lines(
    source: &str,
    line: usize,
    context: usize,
) -> Vec<String> {
    let lines: Vec<&str> = source.lines().collect();
    let start = line.saturating_sub(context);
    let end = (line + context).min(lines.len());

    lines[start..end].to_vec()
}
```

**Context display:**
- Show 2 lines before and after error location
- Mark error line with `>` prefix
- Number lines for reference

**Example:**

```text
98:   models:
 99:     global_config_path: /path/to/config.yml
>100:     default_router: "invalid"
101:     cache_policy: default
```

## Integration with Error Handling Crates

### miette Integration

```rust
use miette::{Diagnostic, SourceSpan};

#[derive(Debug, Diagnostic)]
#[diagnostic(
    code(error_code),
    help(hint),
)]
pub struct MietteDiagnostic {
    #[source_code]
    source_code: String,

    #[label("here")]
    span: SourceSpan,

    error_code: String,
    message: String,
    hint: Option<String>,
}

impl From<ValidationError> for MietteDiagnostic {
    fn from(error: ValidationError) -> Self {
        Self {
            source_code: error.source_code,
            span: error.span,
            error_code: error.code.to_string(),
            message: error.message,
            hint: error.hint,
        }
    }
}
```

**Features:**
- Source code highlighting
- Inline error labels
- Help text display
- Multi-error support
- Color output (via `miette`)

### thiserror Integration

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Type mismatch at {instance_path}: {message}")]
    TypeMismatch {
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        message: String,
    },

    #[error("Required property missing at {instance_path}: {message}")]
    RequiredPropertyMissing {
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        message: String,
    },

    // ... other error variants
}

impl From<ValidationError> for ErrorCode {
    fn from(error: ValidationError) -> Self {
        match error {
            ValidationError::TypeMismatch { .. } => ErrorCode::TypeMismatch,
            ValidationError::RequiredPropertyMissing { .. } => ErrorCode::RequiredPropertyMissing,
            // ... other variants
        }
    }
}
```

**Benefits:**
- Derive `std::error::Error` automatically
- Structured error messages
- Downcast support
- Source chain preservation

## Schema Path Stability

**Problem**: Schema compilation order may vary across builds, leading to unstable `schema_path` values for the same logical schema.

**Solution**: Use canonical schema path generation based on schema structure, not compilation order.

### Path Generation Algorithm

```rust
struct SchemaPathGenerator {
    schema_bundle: SchemaBundle,
    path_stack: Vec<String>,
}

impl SchemaPathGenerator {
    fn generate_path(&mut self, schema: &Schema) -> JsonPointer {
        match schema {
            Schema::Object(obj) => self.generate_object_path(obj),
            Schema::Array(arr) => self.generate_array_path(arr),
            Schema::Map(map) => self.generate_map_path(map),
            Schema::OneOf(oneof) => self.generate_oneof_path(oneof),
            Schema::DiscriminatedUnion(du) => self.generate_du_path(du),
            Schema::Ref(ref_schema) => self.generate_ref_path(ref_schema),
            _ => self.generate_leaf_path(schema),
        }
    }

    fn generate_object_path(&mut self, obj: &ObjectSchema) -> JsonPointer {
        self.path_stack.push("properties".into());

        // Sort required and optional properties alphabetically for stability
        let mut required: Vec<_> = obj.required.keys().collect();
        let mut optional: Vec<_> = obj.optional.keys().collect();
        required.sort();
        optional.sort();

        // Generate paths for each property
        for name in required.iter().chain(optional.iter()) {
            self.path_stack.push(name.clone());
            let prop = obj.required.get(name).or_else(|| obj.optional.get(name));
            if let Some(property) = prop {
                self.generate_path(&property.schema);
            }
            self.path_stack.pop();
        }

        self.path_stack.pop();
        JsonPointer::from_segments(self.path_stack.clone())
    }

    fn generate_ref_path(&mut self, ref_schema: &RefSchema) -> JsonPointer {
        // Look up the referenced schema
        if let Some(resolved) = self.schema_bundle.get(&ref_schema.name) {
            self.generate_path(resolved)
        } else {
            // Fallback to reference name
            self.path_stack.push(ref_schema.name.clone());
            let path = JsonPointer::from_segments(self.path_stack.clone());
            self.path_stack.pop();
            path
        }
    }
}
```

**Stability guarantees:**
- Property paths sorted alphabetically
- Array indices use sequential numbers
- OneOf variants ordered by definition
- Ref resolution follows schema bundle ordering

## Cross-References

- **[01-ir-design.md](./01-ir-design.md)**: `JsonPointer` type definition, schema structure
- **[04-validator-runtime.md](./04-validator-runtime.md)**: Error generation points during validation
- **[07-cli-design.md](./07-cli-design.md)**: Output format selection (`--format` flag), error display

## Open Questions and Decisions Needed

1. **Line number tracking**: Should we preserve line/column positions from YAML/JSON parsers? This requires custom parsing or post-processing.

2. **Error severity levels**: Should we distinguish between errors (must fix) and warnings (should fix)? JTD doesn't define severity levels.

3. **Localization**: Should error messages be internationalizable? How to handle non-English messages in machine-readable JSON output?

4. **Error count limit**: Should there be a maximum number of errors per document (e.g., 100) to prevent excessive output? What about max errors per path?

5. **Schema path canonicalization**: Should schema paths be fully expanded (include all intermediate nodes) or compressed (jump directly to leaf)?

## Research Links

This design draws from the following ChatGPT research sections:

- **"Error reporting format"** (second report, lines 540-557): Machine-readable JSON, human-readable multi-line, JTD-style pointers
- **"JTD's portable validation errors"** (second report, lines 605-610): `instancePath` + `schemaPath` convention, error object structure
- **"End-to-end validation examples"** (second report, lines 572-630): Example error output format, validation report structure
- **"miette diagnostics"** (second report, lines 556): Rich error library for Rust
- **"thiserror derive Error"** (second report, lines 557): Ergonomic error enum derivation
