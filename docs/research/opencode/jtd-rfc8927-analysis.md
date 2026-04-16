# JTD (RFC 8927) Analysis + Rust Crate Ecosystem

> Research by OpenCode librarian, 2026-04-15

## Verdict: Build custom IR; optionally use `jtd` crate for JTD export validation

## The 8 JTD Schema Forms

1. **Empty** (`{}`) — Accepts any value. May include `nullable` + `metadata`.
2. **Ref** (`{"ref": "name"}`) — References a definition in root `definitions`.
3. **Type** (`{"type": "string"}`) — Primitives: `boolean`, `float32`, `float64`, `int8/16/32`, `uint8/16/32`, `string`, `timestamp`.
4. **Enum** (`{"enum": ["foo", "bar"]}`) — Non-empty array of unique **strings only**.
5. **Elements** (`{"elements": {...}}`) — Homogeneous arrays. Each element matches schema.
6. **Properties** (`{"properties": {...}, "optionalProperties": {...}}`) — Struct/record. Keys cannot overlap between properties/optionalProperties. Optional `additionalProperties: boolean`.
7. **Values** (`{"values": {...}}`) — Dictionary/map from string keys to schema values.
8. **Discriminator** (`{"discriminator": "tag", "mapping": {...}}`) — Tagged union. Discriminator key selects schema from mapping.

## Error Format

```json
[{
  "instancePath": "/path/to/error",
  "schemaPath": "/properties/age/type"
}]
```

- `instancePath`: JSON Pointer (RFC 6901) to rejected value
- `schemaPath`: JSON Pointer to rejecting schema member
- Array of error objects (order unspecified)

## What JTD CANNOT Express (Our Extensions)

| Feature | JTD | Our IR |
|---|---|---|
| Regex on strings | ❌ | ✅ `@pattern(regex)` |
| Numeric min/max | ❌ | ✅ Scalar constraints |
| String min/max length | ❌ | ✅ Scalar constraints |
| additionalProperties with schema | ❌ (boolean only) | ✅ `@mapRest` → `AllowSchema` |
| oneOf (shape-based) | ❌ | ✅ `OneOf` variant |
| anyOf/allOf/not | ❌ | N/A (not needed) |
| 64-bit integers | ❌ | ✅ Custom ScalarKind |
| Default values | ❌ | ✅ `@default` |
| Recursive constraints | ❌ (only definitions) | ✅ Via Ref |

## Rust Crate Ecosystem

### `jtd` v0.3.1
- **Author**: Ulysse Carion (RFC 8927 author)
- **Purpose**: Schema parser + validator
- **API**: `jtd::validate(&schema, &instance, Default::default())`
- **Features**: RFC-compliant error indicators (`ValidationErrorIndicator`), `ValidateOptions` with `max_depth`
- **Status**: ~604 downloads/month, last release Jan 2021 (stable but not actively developed)
- **Use case**: Validate that our JTD export is RFC-compliant

### `jtd-derive` v0.1.4
- **Author**: uint (Tomasz Kurcz)
- **Purpose**: Derive macro for generating JTD schemas FROM Rust types
- **Status**: "API is unstable. Expect breaking changes."
- **Verdict**: NOT useful for our use case

### `jtd-codegen` v0.2.0-beta.1
- **Author**: Ulysse Carion
- **Purpose**: Generate code FROM JTD schemas (Rust, TypeScript, Go, etc.)
- **Use case**: Could be used for JTD→Rust struct codegen export

### `jtd-infer` v0.2.1
- **Purpose**: Infer JTD schemas from example data
- **Use case**: Not relevant

## Recommendation

**Build a custom IR** that is JTD-inspired but with our extensions. The `jtd` crate can optionally be used as a validation backend for the JTD export feature (to ensure exported JTD is RFC-compliant). Do NOT use `jtd` as the primary validator — our IR has extensions (regex, oneOf, mapRest) that JTD cannot express.

## References

- RFC 8927: https://datatracker.ietf.org/doc/html/rfc8927
- JTD validation errors guide: https://jsontypedef.com/docs/validation-errors/
- jtd crate: https://docs.rs/jtd/latest/jtd/
- jtd-codegen: https://crates.io/crates/jtd-codegen
