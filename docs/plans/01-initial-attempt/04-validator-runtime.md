# Validator Runtime Design

## Overview

This document specifies the runtime validation engine that applies compiled IR schemas to YAML/JSON documents. The validator performs recursive descent validation through a canonical `Value` model, threading instance and schema paths while accumulating errors in a validation context.

**Key characteristics:**
- **Recursive descent**: Traverses both schema and value trees depth-first
- **Path-threaded**: Maintains JSON Pointer paths for both instance and schema locations
- **Error-accumulating**: Collects all validation errors, not just the first
- **Strict/Open modes**: Runtime policy controls strictness behavior
- **Default-aware**: Applies default values according to schema directives

## Canonical Value Model

The validator operates on a single canonical representation regardless of source format (YAML or JSON):

```rust
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(IndexMap<String, Value>),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Number {
    Integer(i64),
    Float(f64),
}
```

**Design rationale:**
- `IndexMap<String, Value>` preserves order and provides O(1) lookups
- `Number` enum distinguishes between integer and float representations, which matters for:
  - YAML parsing (1 parses as int, 1.0 as float)
  - Type coercion policies
  - Integer-only constraints (e.g., port numbers)
- `serde` support enables round-tripping YAML/JSON documents

## Validation Context and Signature

```rust
#[derive(Debug, Default)]
pub struct ValidationContext {
    instance_path: JsonPointer,
    schema_path: JsonPointer,
    errors: Vec<ValidationError>,
    strict_mode: bool,
    open_mode: bool,
    depth: u32,
}

#[derive(Debug)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub instance_path: JsonPointer,
    pub schema_path: JsonPointer,
}

pub fn validate(
    value: &Value,
    schema: &Schema,
    ctx: &mut ValidationContext,
) -> Result<(), ValidationReport> {
    // Validation algorithm implementation
}
```

**Context threading:**
- `instance_path`: Tracks location in the YAML/JSON document (JSON Pointer)
- `schema_path`: Tracks location in the schema IR (JSON Pointer)
- `errors`: Accumulates validation errors as they're discovered
- `strict_mode`/`open_mode`: Control runtime behavior (see below)
- `depth`: Prevents stack overflow from recursive schemas

## Validation Algorithms by IR Variant

### Any Schema

**Always valid**. Accepts any `Value` without constraint.

```rust
fn validate_any(_value: &Value, ctx: &mut ValidationContext) {
    // No validation; always valid
}
```

### Scalar Schema

**Type check + constraints**. Validates that the value matches the expected scalar type and applies any constraints.

```rust
fn validate_scalar(
    value: &Value,
    scalar: &ScalarSchema,
    ctx: &mut ValidationContext,
) {
    match (&scalar.kind, value) {
        (ScalarKind::Boolean, Value::Bool(_)) => {}
        (ScalarKind::String, Value::String(s)) => {
            if let Some(pattern) = &scalar.pattern {
                if !regex::Regex::new(pattern).unwrap().is_match(s) {
                    ctx.errors.push(ValidationError {
                        code: ErrorCode::PatternMismatch,
                        instance_path: ctx.instance_path.clone(),
                        schema_path: ctx.schema_path.clone(),
                        message: format!("String does not match pattern: {}", pattern),
                        hint: Some(format!("Expected pattern: {}", pattern)),
                    });
                }
            }
        }
        (ScalarKind::Int, Value::Number(Number::Integer(_))) => {}
        (ScalarKind::Float, Value::Number(_)) => {}
        (expected, actual) => {
            ctx.errors.push(ValidationError {
                code: ErrorCode::TypeMismatch,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!("Expected type {:?}, found {:?}", expected, actual),
                hint: Some(format!("Value must be a {:?}", expected)),
            });
        }
    }

    // Apply range constraints for numbers
    if let (Some(min), Value::Number(Number::Integer(n))) = (scalar.min, value) {
        if *n < min {
            ctx.errors.push(ValidationError {
                code: ErrorCode::NumberOutOfRange,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!("Number {} is below minimum {}", n, min),
                hint: Some(format!("Value must be >= {}", min)),
            });
        }
    }
}
```

**Edge cases:**
- In strict mode: `"123"` is not coerced to `123`
- In open mode: limited coercion may apply (e.g., YAML `1` parsed as int is accepted for Float scalar)
- Pattern matching applies only to string-like scalars

### Enum Schema

**String enum validation**. Value must be a string and match one of the allowed enum values.

```rust
fn validate_enum(
    value: &Value,
    enum_schema: &EnumSchema,
    ctx: &mut ValidationContext,
) {
    match value {
        Value::String(s) if enum_schema.values.contains(s) => {}
        Value::String(s) => {
            ctx.errors.push(ValidationError {
                code: ErrorCode::EnumValueInvalid,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!(
                    "Invalid enum value '{}'. Expected one of: {}",
                    s,
                    enum_schema.values.join(", ")
                ),
                hint: Some(format!("Use one of: {}", enum_schema.values.join(", "))),
            });
        }
        other => {
            ctx.errors.push(ValidationError {
                code: ErrorCode::TypeMismatch,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!("Expected string enum, found {:?}", other),
                hint: Some("Enum values must be strings".into()),
            });
        }
    }
}
```

### Array Schema

**Array element validation**. Must be an array; each element validated against the `elements` schema.

```rust
fn validate_array(
    value: &Value,
    array_schema: &ArraySchema,
    ctx: &mut ValidationContext,
) {
    match value {
        Value::Array(items) => {
            for (idx, item) in items.iter().enumerate() {
                let prev_instance_path = ctx.instance_path.clone();
                let prev_schema_path = ctx.schema_path.clone();

                ctx.instance_path.push(idx.to_string());
                ctx.schema_path.push("elements");

                validate(item, &array_schema.elements, ctx);

                ctx.instance_path = prev_instance_path;
                ctx.schema_path = prev_schema_path;
            }
        }
        other => {
            ctx.errors.push(ValidationError {
                code: ErrorCode::TypeMismatch,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!("Expected array, found {:?}", other),
                hint: Some("Value must be an array".into()),
            });
        }
    }
}
```

### Object Schema

**Record validation**. Must be a map; validates required and optional properties; handles additional keys per policy.

```rust
fn validate_object(
    value: &Value,
    object_schema: &ObjectSchema,
    ctx: &mut ValidationContext,
) {
    match value {
        Value::Object(map) => {
            // Check required properties
            for (name, property) in &object_schema.required {
                if !map.contains_key(name) {
                    let mut error_path = ctx.schema_path.clone();
                    error_path.push("required");
                    error_path.push(name);

                    ctx.errors.push(ValidationError {
                        code: ErrorCode::RequiredPropertyMissing,
                        instance_path: ctx.instance_path.clone(),
                        schema_path: error_path,
                        message: format!("Required property '{}' is missing", name),
                        hint: Some(format!("Add property: {}", name)),
                    });
                }
            }

            // Validate present properties
            for (key, val) in map {
                let mut instance_path = ctx.instance_path.clone();
                instance_path.push(key.clone());

                let mut schema_path = ctx.schema_path.clone();

                // Check if key is a defined property
                if let Some(property) = object_schema.required.get(key)
                    .or_else(|| object_schema.optional.get(key))
                {
                    schema_path.push(if object_schema.required.contains_key(key) {
                        "required"
                    } else {
                        "optional"
                    });
                    schema_path.push(key);

                    validate(val, &property.schema, ctx);
                    continue;
                }

                // Handle additional keys
                match &object_schema.additional {
                    AdditionalPolicy::Reject => {
                        if ctx.strict_mode || object_schema.closed {
                            ctx.errors.push(ValidationError {
                                code: ErrorCode::AdditionalPropertyRejected,
                                instance_path: instance_path.clone(),
                                schema_path: schema_path.clone(),
                                message: format!("Additional property '{}' is not allowed", key),
                                hint: Some("Remove this property or add to schema".into()),
                            });
                        }
                    }
                    AdditionalPolicy::AllowAny => {
                        // Ignore in open mode
                        if !ctx.open_mode && object_schema.closed {
                            ctx.errors.push(ValidationError {
                                code: ErrorCode::AdditionalPropertyRejected,
                                instance_path: instance_path,
                                schema_path,
                                message: format!("Additional property '{}' is not allowed", key),
                                hint: Some("Remove this property".into()),
                            });
                        }
                    }
                    AdditionalPolicy::AllowSchema(schema) => {
                        schema_path.push("additional_schema");
                        validate(val, schema, ctx);
                    }
                }
            }
        }
        other => {
            ctx.errors.push(ValidationError {
                code: ErrorCode::TypeMismatch,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!("Expected object, found {:?}", other),
                hint: Some("Value must be an object/map".into()),
            });
        }
    }
}
```

**Property resolution order:**
1. Required properties (defined in `required` map)
2. Optional properties (defined in `optional` map)
3. Additional properties (handled by `additional_policy`)

**Additional policy semantics:**
- `Reject`: Reject unknown keys (strict baseline)
- `AllowAny`: Accept unknown keys, ignore them (open mode)
- `AllowSchema`: Validate unknown keys against a schema (used for `@mapRest`)

### Map Schema

**Map value validation**. Must be a map; all values validated against a single schema.

```rust
fn validate_map(
    value: &Value,
    map_schema: &MapSchema,
    ctx: &mut ValidationContext,
) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let mut instance_path = ctx.instance_path.clone();
                instance_path.push(key.clone());

                let mut schema_path = ctx.schema_path.clone();
                schema_path.push("values");

                validate(val, &map_schema.values, ctx);
            }
        }
        other => {
            ctx.errors.push(ValidationError {
                code: ErrorCode::TypeMismatch,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!("Expected map/object, found {:?}", other),
                hint: Some("Value must be an object with string keys".into()),
            });
        }
    }
}
```

**Use cases:**
- `@mapRest` directive: Validates unknown keys against a schema
- Generic map types: When all values must conform to a single type

### @mapRest Validation Order

Validating objects with `@mapRest` requires a specific three-phase order to ensure correct error reporting and collision handling.

**Phase 1**: Validate known properties (required + optional) against their schemas.
**Phase 2**: Collect remaining keys not matched by known properties.
**Phase 3**: Validate each remaining key's value against `@mapRest` schema.

### Validation Algorithm

```rust
fn validate_object_with_maprest(
    value: &Value,
    object_schema: &ObjectSchema,
    ctx: &mut ValidationContext,
) {
    match value {
        Value::Object(map) => {
            let known_keys: std::collections::HashSet<&String> = object_schema
                .required
                .keys()
                .chain(object_schema.optional.keys())
                .collect();

            // Phase 1: Validate known properties
            let mut validated_keys = std::collections::HashSet::new();

            for (key, val) in map {
                if known_keys.contains(key) {
                    validated_keys.insert(key.clone());

                    // Check for collision (should be impossible by construction)
                    assert!(!object_schema.required.contains_key(key)
                           || !object_schema.optional.contains_key(key),
                           "Bug: rest key matches known property");

                    // Validate against property schema
                    let property = object_schema
                        .required
                        .get(key)
                        .or_else(|| object_schema.optional.get(key))
                        .unwrap();

                    let mut instance_path = ctx.instance_path.clone();
                    instance_path.push(key.clone());

                    let mut schema_path = ctx.schema_path.clone();
                    schema_path.push(if object_schema.required.contains_key(key) {
                        "required"
                    } else {
                        "optional"
                    });
                    schema_path.push(key);

                    validate(val, &property.schema, ctx);

                    ctx.instance_path = instance_path;
                    ctx.schema_path = schema_path;
                }
            }

            // Phase 2: Collect rest keys
            let rest_keys: Vec<&String> = map
                .keys()
                .filter(|k| !validated_keys.contains(k))
                .collect();

            // Phase 3: Validate each rest key against mapRest schema
            if let AdditionalPolicy::AllowSchema(rest_schema) = &object_schema.additional {
                for key in rest_keys {
                    let val = map.get(key).unwrap();

                    let mut instance_path = ctx.instance_path.clone();
                    instance_path.push(key.clone());

                    let mut schema_path = ctx.schema_path.clone();
                    schema_path.push("additional_schema");
                    schema_path.push("~rest");
                    schema_path.push(key);

                    validate(val, rest_schema, ctx);

                    ctx.instance_path = instance_path;
                    ctx.schema_path = schema_path;
                }
            }
        }
        other => {
            ctx.errors.push(ValidationError {
                code: ErrorCode::TypeMismatch,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!("Expected object, found {:?}", other),
                hint: Some("Value must be a map/object".into()),
            });
        }
    }
}
```

### Collision Rule

If a remaining key name matches a known property name, this is a bug in the compiler (impossible by construction). Add assertion:

```rust
assert!(!object_schema.required.contains_key(rest_key)
       || !object_schema.optional.contains_key(rest_key),
       "Compiler bug: rest key collides with known property");
```

### Error Ordering

Known property errors are reported first, then rest-key errors. This ensures clear error messages.

### SchemaPath Assignment

Rest keys get `/properties/~rest/{key}` path for precise error reporting.

### DiscriminatedUnion Schema

**Tagged union validation**. Must be an object; discriminator field determines which schema to apply.

```rust
fn validate_discriminated_union(
    value: &Value,
    du_schema: &DiscriminatedUnionSchema,
    ctx: &mut ValidationContext,
) {
    match value {
        Value::Object(map) => {
            let discriminator_value = match map.get(&du_schema.discriminator_field) {
                Some(Value::String(tag)) => Some(tag),
                None => {
                    ctx.errors.push(ValidationError {
                        code: ErrorCode::DiscriminatorMissing,
                        instance_path: ctx.instance_path.clone(),
                        schema_path: ctx.schema_path.clone(),
                        message: format!(
                            "Discriminator field '{}' is missing",
                            du_schema.discriminator_field
                        ),
                        hint: Some(format!(
                            "Add field: '{}': <tag>",
                            du_schema.discriminator_field
                        )),
                    });
                    return;
                }
                _ => {
                    ctx.errors.push(ValidationError {
                        code: ErrorCode::DiscriminatorInvalid,
                        instance_path: ctx.instance_path.clone(),
                        schema_path: ctx.schema_path.clone(),
                        message: format!(
                            "Discriminator field '{}' must be a string",
                            du_schema.discriminator_field
                        ),
                        hint: Some("Discriminator value must be a string".into()),
                    });
                    return;
                }
            };

            if let Some(schema) = du_schema.mapping.get(discriminator_value) {
                let mut instance_path = ctx.instance_path.clone();
                instance_path.push(du_schema.discriminator_field.clone());

                validate(value, schema, ctx);
            } else {
                ctx.errors.push(ValidationError {
                    code: ErrorCode::DiscriminatorInvalid,
                    instance_path: ctx.instance_path.clone(),
                    schema_path: ctx.schema_path.clone(),
                    message: format!(
                        "Unknown discriminator value '{}'. Expected one of: {}",
                        discriminator_value,
                        du_schema.mapping.keys().collect::<Vec<_>>().join(", ")
                    ),
                    hint: Some(format!(
                        "Use one of: {}",
                        du_schema.mapping.keys().collect::<Vec<_>>().join(", ")
                    )),
                });
            }
        }
        other => {
            ctx.errors.push(ValidationError {
                code: ErrorCode::TypeMismatch,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!("Expected object, found {:?}", other),
                hint: Some("Discriminated unions require an object".into()),
            });
        }
    }
}
```

**Mapping resolution:**
- `@discriminator(field: "type")` directive specifies discriminator field name
- `@variant(tag: "agent")` on member types maps tags to schemas

### OneOf Schema

**Exclusive disjunction validation**. Exactly one variant must match with zero errors.

## Gap Fix: OneOf Matching Algorithm

OneOf matching requires a specific algorithm to detect ambiguity and provide clear error messages.

### Matching Algorithm

```rust
fn validate_oneof(
    value: &Value,
    oneof_schema: &OneOfSchema,
    ctx: &mut ValidationContext,
) {
    let mut matching_variants = Vec::new();
    let mut failed_variants = Vec::new();

    for (idx, variant) in oneof_schema.variants.iter().enumerate() {
        // Step 1: Check if all required fields in variant schema are present
        if !all_required_fields_present(value, &variant.schema) {
            // Step 2: Short-circuit - skip this variant
            failed_variants.push(idx);
            continue;
        }

        // Step 3: Run full validation, count errors
        let mut trial_ctx = ctx.clone();
        let mut schema_path = ctx.schema_path.clone();
        schema_path.push("variants");
        schema_path.push(idx.to_string());

        validate(value, &variant.schema, &mut trial_ctx);

        // Step 4: Score (0 errors = match)
        if trial_ctx.errors.is_empty() {
            matching_variants.push((idx, &variant.label));
        } else {
            failed_variants.push(idx);
        }
    }

    // Step 5-7: Determine outcome
    match matching_variants.len() {
        0 => {
            // Step 6: No variants match
            ctx.errors.push(ValidationError {
                code: ErrorCode::OneOfNoMatch,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!(
                    "No OneOf variant matched. Tried variants: {}",
                    oneof_schema.variants.iter().map(|v| &v.label).collect::<Vec<_>>().join(", ")
                ),
                hint: Some("Value must match exactly one variant".into()),
            });
        }
        1 => {
            // Step 5: Exactly one variant matches - success
        }
        n if n > 1 => {
            // Step 7: Ambiguous - 2+ variants match
            let matching_labels: Vec<&String> = matching_variants
                .iter()
                .map(|(_, label)| *label)
                .collect();

            ctx.errors.push(ValidationError {
                code: ErrorCode::OneOfAmbiguous,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!(
                    "OneOf is ambiguous: {} variants matched ({})",
                    n,
                    matching_labels.join(", ")
                ),
                hint: Some("Tighten constraints to eliminate ambiguity".into()),
            });
        }
        _ => unreachable!(),
    }
}

fn all_required_fields_present(value: &Value, schema: &Schema) -> bool {
    match schema {
        Schema::Object(object) => {
            match value {
                Value::Object(map) => {
                    object.required.keys().all(|key| map.contains_key(key))
                }
                _ => false,
            }
        }
        // For other schema types, always return true (no required fields)
        _ => true,
    }
}
```

### Performance Optimization

Early exit after finding 2 matches (can immediately report ambiguity):

```rust
// Inside the variant loop
if matching_variants.len() >= 2 {
    // Early exit - already ambiguous
    break;
}
```

### Error Messages

- **No match**: List all variant labels to help debugging.
- **Ambiguous**: List matching variant labels to identify conflict.
- **Hint**: "Tighten constraints to eliminate ambiguity" for ambiguous cases.

### Example: Ambiguous Case

```yaml
# Value
field1: "same value"

# Schema
input VariantA { field1: String }
input VariantB { field1: String }

union Example @oneOf = VariantA | VariantB

# Result: Ambiguous error
# Both VariantA and VariantB have required field "field1" present
```

**Exclusive disjunction validation**. Exactly one variant must match with zero errors.

```rust
fn validate_oneof(
    value: &Value,
    oneof_schema: &OneOfSchema,
    ctx: &mut ValidationContext,
) {
    let mut matching_variants = Vec::new();

    for (idx, variant) in oneof_schema.variants.iter().enumerate() {
        let mut trial_ctx = ctx.clone();

        ctx.schema_path.push("variants");
        ctx.schema_path.push(idx.to_string());

        validate(value, variant, &mut trial_ctx);

        if trial_ctx.errors.is_empty() {
            matching_variants.push((idx, variant));
        }

        ctx.schema_path.pop();
        ctx.schema_path.pop();
    }

    match matching_variants.len() {
        0 => {
            // No variant matched: report closest match as hint
            ctx.errors.push(ValidationError {
                code: ErrorCode::OneOfNoMatch,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: "No OneOf variant matched".into(),
                hint: Some("Value must match exactly one variant".into()),
            });
        }
        1 => {
            // Exactly one variant matched: success
        }
        n if n > 1 => {
            // Multiple variants matched: ambiguous
            ctx.errors.push(ValidationError {
                code: ErrorCode::OneOfAmbiguous,
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.clone(),
                message: format!("OneOf is ambiguous: {} variants matched", n),
                hint: Some("Tighten constraints to eliminate ambiguity".into()),
            });
        }
        _ => unreachable!(),
    }
}
```

**Error recovery:**
- If 0 match: Provide the variant with the fewest errors as a hint
- If 2+ match: Report all matching variant indices to help debugging

### Ref Schema

**Schema reference resolution**. Resolve reference in schema bundle; detect cycles; validate resolved schema.

```rust
fn validate_ref(
    value: &Value,
    ref_schema: &RefSchema,
    ctx: &mut ValidationContext,
) {
    // Check for recursion cycle
    if ctx.visited_schemas.contains(&ref_schema.name) {
        ctx.errors.push(ValidationError {
            code: ErrorCode::RecursiveType,
            instance_path: ctx.instance_path.clone(),
            schema_path: ctx.schema_path.clone(),
            message: format!("Recursive type detected: {}", ref_schema.name),
            hint: Some("Schemas must be acyclic".into()),
        });
        return;
    }

    ctx.visited_schemas.insert(ref_schema.name.clone());

    // Resolve reference
    if let Some(resolved) = ctx.schema_bundle.get(&ref_schema.name) {
        let mut schema_path = ctx.schema_path.clone();
        schema_path.push(ref_schema.name.clone());

        validate(value, resolved, ctx);
    } else {
        ctx.errors.push(ValidationError {
            code: ErrorCode::RefUnresolved,
            instance_path: ctx.instance_path.clone(),
            schema_path: ctx.schema_path.clone(),
            message: format!("Unresolved schema reference: {}", ref_schema.name),
            hint: Some("Referenced schema does not exist".into()),
        });
    }

    ctx.visited_schemas.remove(&ref_schema.name);
}
```

**Cycle detection:**
- Track visited schema names in `ValidationContext`
- Error on repeated reference to same schema without intermediate types

## Strict vs Open Runtime Modes

### Strict Mode

**Default behavior**. Enforces maximal strictness regardless of schema directives.

```rust
pub struct ValidationContext {
    strict_mode: bool,  // Default: true
    open_mode: bool,    // Default: false
}
```

**Strict mode behaviors:**
- Reject additional keys (unless `@mapRest` explicitly allows)
- Reject duplicate YAML keys at parse time
- No type coercion (`"123"` is not coerced to `123`)
- Enforce `@closed` directive on all object types
- Reject unknown directives in schema

**Implementation:**

```rust
fn validate_additional_property(
    key: &str,
    ctx: &mut ValidationContext,
) {
    if ctx.strict_mode {
        ctx.errors.push(ValidationError {
            code: ErrorCode::AdditionalPropertyRejected,
            instance_path: ctx.instance_path.clone(),
            schema_path: ctx.schema_path.clone(),
            message: format!("Additional property '{}' is not allowed", key),
            hint: Some("Remove this property".into()),
        });
    }
}
```

### Open Mode

**Permissive behavior**. Allows additional keys and limited type coercion.

**Open mode behaviors:**
- Allow additional keys (ignore them, don't validate)
- Accept duplicate YAML keys (last wins)
- Allow limited type coercion (e.g., YAML `1` → Float scalar)
- Ignore `@closed` directive (treat all objects as open)

**Implementation:**

```rust
fn validate_additional_property(
    key: &str,
    ctx: &mut ValidationContext,
) {
    if ctx.open_mode {
        // Ignore additional keys in open mode
        return;
    }

    // Default strict behavior
    ctx.errors.push(ValidationError {
        code: ErrorCode::AdditionalPropertyRejected,
        instance_path: ctx.instance_path.clone(),
        schema_path: ctx.schema_path.clone(),
        message: format!("Additional property '{}' is not allowed", key),
        hint: Some("Remove this property".into()),
    });
}
```

**Type coercion rules (open mode only):**

| Source Type | Target Scalar | Coerced? | Rationale |
|-------------|---------------|-----------|-----------|
| `Integer` | `Float` | Yes | Lossless numeric conversion |
| `String("123")` | `Int` | No | Requires explicit parsing |
| `Bool(true)` | `String` | No | Type mismatch |
| `Null` | `String` | No | Null is distinct |

## Gap Fix: Number Type Coercion Rules

Number coercion rules vary by validation mode (strict vs open) and handle edge cases like float-with-zero-fraction, large numbers, and scientific notation.

### Coercion Function

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationMode {
    Strict,
    Open,
}

fn coerce_number(
    value: &Value,
    expected: &ScalarKind,
    mode: ValidationMode,
) -> Result<Number, ErrorCode> {
    let number = match value {
        Value::Number(n) => n.clone(),
        _ => return Err(ErrorCode::TypeMismatch),
    };

    match (expected, mode) {
        // Strict mode: Int accepts only Integer
        (ScalarKind::Int, ValidationMode::Strict) => match number {
            Number::Integer(_) => Ok(number),
            Number::Float(_) => Err(ErrorCode::NumberNotInteger),
        },

        // Open mode: Int accepts float-with-zero-fraction
        (ScalarKind::Int, ValidationMode::Open) => match number {
            Number::Integer(_) => Ok(number),
            Number::Float(f) if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 => {
                Ok(Number::Integer(f as i64))
            }
            Number::Float(_) => Err(ErrorCode::NumberNotInteger),
        },

        // Both modes: Float accepts Integer or Float
        (ScalarKind::Float, _) => Ok(match number {
            Number::Integer(i) => Number::Float(i as f64),
            Number::Float(f) => Number::Float(f),
        }),

        // Float with large integer portion accepted as Float
        (ScalarKind::Int, ValidationMode::Open) => match number {
            Number::Float(f) if f.abs() > i64::MAX as f64 => Ok(Number::Float(f)),
            _ => unreachable!(),
        },

        _ => Ok(number),
    }
}
```

### Coercion Rules

**Strict mode**:
- `Int` schema: Accepts only `Number::Integer`. `1.0` → error.
- `Float` schema: Accepts both `Integer` and `Float`. `1` → coerced to `1.0`.

**Open mode**:
- `Int` schema: Coerces float-with-zero-fraction (e.g., `1.0` → `1`). Rejects non-zero fractions.
- `Float` schema: Accepts both `Integer` and `Float`. Same as strict.
- Large numbers: If integer portion exceeds `i64::MAX`, accept as `Float`.

**Large numbers**:
- Strict mode: Reject if value > `i64::MAX`.
- Open mode: Accept as `Float` if integer portion too large.

**Scientific notation**:
- Parsed by serde into `Number` automatically.
- If integer portion fits `i64` → `Integer`, else `Float`.
- Example: `1e10` → `Number::Integer(10000000000)`.

### Edge Cases

| Input | Mode | Expected Type | Result |
|--------|-------|----------------|---------|
| `1` (Integer) | Strict | Int | Accept |
| `1.0` (Float) | Strict | Int | Error |
| `1.0` (Float) | Open | Int | Accept (coerce to 1) |
| `1.5` (Float) | Open | Int | Error |
| `1e10` (Integer) | Strict | Int | Accept |
| `9999999999999999999` (Float) | Open | Int | Accept (as Float) |
| `9223372036854775807` (Float) | Strict | Float | Accept |
| `9223372036854775808` (Float) | Strict | Int | Error |

## Default Value Application

**Application timing**. Defaults are applied during validation, not during parsing.

**Ordering rules:**
1. Parse YAML/JSON into canonical `Value`
2. Apply defaults for missing optional fields
3. Run validation

**Default value semantics:**

```rust
fn apply_defaults(
    value: &mut Value,
    object_schema: &ObjectSchema,
) {
    if let Value::Object(map) = value {
        for (name, property) in &object_schema.optional {
            if !map.contains_key(name) {
                if let Some(default) = &property.default {
                    map.insert(name.clone(), default.clone());
                }
            }
        }
    }
}
```

**Default merging:**
- Deep merge for nested objects
- Replace for scalars and arrays
- Default values are not validated (assumed to be valid)

**Directive handling:**
- `@default(value: "...")` directive on fields
- GraphQL default literal syntax (`name: String = "default"`)
- Conflict: GraphQL literal takes precedence over `@default`

## Cross-References

- **[01-ir-design.md](./01-ir-design.md)**: IR types being validated (`Schema` enum, `ScalarSchema`, `ObjectSchema`, etc.)
- **[05-error-reporting.md](./05-error-reporting.md)**: Error accumulation, `ValidationError` type, `ErrorCode` enum
- **[06-schema-registry.md](./06-schema-registry.md)**: Schema resolution for `Ref` validation, schema bundle loading

## Gap Fix: Value Conversion from Parsers

Parsers (YAML/JSON) produce different value types. Canonical `Value` requires conversion functions.

### serde_json::Value Conversion

```rust
impl From<serde_json::Value> for Value {
    fn from(json_value: serde_json::Value) -> Self {
        match json_value {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Number(Number::Integer(i))
                } else {
                    Value::Number(Number::Float(n.as_f64().unwrap()))
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(|v| v.into()).collect())
            }
            serde_json::Value::Object(obj) => {
                Value::Object(
                    obj.into_iter()
                        .map(|(k, v)| (k, v.into()))
                        .collect()
                )
            }
        }
    }
}
```

### serde_saphyr::Value Conversion

```rust
impl From<serde_saphyr::Value> for Value {
    fn from(yaml_value: serde_saphyr::Value) -> Self {
        match yaml_value {
            serde_saphyr::Value::Null => Value::Null,
            serde_saphyr::Value::Bool(b) => Value::Bool(b),
            serde_saphyr::Value::Number(n) => {
                // saphyr may represent integers differently
                match n.as_i64() {
                    Some(i) => Value::Number(Number::Integer(i)),
                    None => Value::Number(Number::Float(n.as_f64().unwrap_or(0.0))),
                }
            }
            serde_saphyr::Value::String(s) => Value::String(s),
            serde_saphyr::Value::Sequence(seq) => {
                Value::Array(seq.into_iter().map(|v| v.into()).collect())
            }
            serde_saphyr::Value::Mapping(map) => {
                Value::Object(
                    map.into_iter()
                        .map(|(k, v)| (k.to_string(), v.into()))
                        .collect()
                )
            }
        }
    }
}
```

### Conversion Characteristics

**Infallible conversions**: Both conversions are lossless and never fail.

**Number handling**:
- serde_saphyr may represent integers differently across YAML versions.
- Use `as_i64()` first, fall back to `as_f64()`.

**Map key handling**:
- serde_saphyr uses `Yaml` as keys → convert to `String` via `to_string()`.
- serde_json uses `String` keys directly.

**No data loss**: Canonical `Value` preserves all information from parser values.

## Open Questions and Decisions Needed

1. **Depth limit for recursion**: Should there be a configurable maximum depth (default 100) to prevent stack overflow from recursive schemas?

2. **Type coercion in open mode**: Should we support any string-to-number coercion (e.g., `"123"` → `123`) or only integer-to-float?

3. **Default value validation**: Should default values be validated against their field's schema, or assumed to be valid?

4. **Error truncation**: Should there be a limit on number of errors per document (e.g., 100) to prevent excessive output?

5. **Schema path stability**: How to ensure stable `schema_path` generation when compilation order may vary across builds?

## Research Links

This design draws from the following ChatGPT research sections:

- **"Runtime validator design and algorithms"** (second report, lines 454-539): Core validation algorithms by IR variant, recursive descent design, path threading
- **"Canonical value model"** (second report, lines 456-470): JSON/YAML parsing strategies, `serde-saphyr` for duplicate key detection
- **"Strict/open modes"** (second report, lines 471-486): kubeconform-inspired strictness behavior
- **"Core validation algorithms by IR variant"** (second report, lines 487-539): Detailed algorithms for Scalar, Enum, Array, Object, Map, DiscriminatedUnion, OneOf, Ref
- **"End-to-end validation examples"** (second report, lines 558-630): Validation output format, JTD-style error indicators

### OpenCode Research Corrections
- [YAML Parser Analysis](../research/opencode/yaml-parser-analysis.md) — serde-saphyr is the correct YAML parser choice, not yaml-rust2.
- [JTD RFC 8927 Analysis](../research/opencode/jtd-rfc8927-analysis.md) — Build custom validator, not jtd crate backend.
