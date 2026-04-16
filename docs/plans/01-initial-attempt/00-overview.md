# Project Overview

## Mission and Scope

`graphql-ish-schema-validator` is a Rust-first schema system for validating YAML/JSON documents. It provides a GraphQL-inspired SDL (Schema Definition Language) as the authoring layer, compiles it into a JTD-like intermediate representation (IR), and validates documents against that IR with strictness and excellent diagnostics.

The project separates two concerns:

1. **Schema validation**: Parsing and verifying the custom SDL is internally consistent (names resolve, directives are well-formed, unions are coherent, defaults are type-compatible).
2. **Instance validation**: Applying the compiled IR to YAML/JSON content and emitting actionable errors with stable machine-readable pointers (JTD-style `instancePath`/`schemaPath`).

This architecture provides a "GraphQL schema feel" for authors but uses a "JTD-like, explicitly-typed, compiler-friendly IR" for the validator and optional export into literal JTD JSON.

## Architecture

The system follows a multi-stage compilation and validation pipeline:

```mermaid
flowchart LR
    A[SDL source\n*.graphql-ish] --> B[Parse\napollo-parser CST + errors]
    B --> C[Build AST\n(custom, subset)]
    C --> D[Schema semantic validation\n(names, directives, unions)]
    D --> E[Lowering\nAST -> JTD-like IR]
    E --> F[Optional export\nIR -> JTD JSON]
    E --> G[Validator runtime\nIR + canonical value]
    H[Input YAML/JSON] --> I[Parse\nserde_json + YAML parser]
    I --> J[Canonicalize\nValue model + defaults]
    J --> G
    G --> K[ValidationReport\nerrors + pointers + hints]
```

### Key Components

1. **SDL Parser**: Uses `apollo-parser` to parse GraphQL-like schema files. Returns a CST (Concrete Syntax Tree) with syntax errors for editor-like UX.

2. **AST Builder**: Transforms the CST into a minimal AST representing only the supported SDL subset (scalars, enums, inputs, unions, directives).

3. **Semantic Validator**: Validates the AST for:
   - Unknown/duplicate type names
   - Unknown directives or directive misplacement
   - Type reference resolution
   - Union coherence (discriminator/@oneOf consistency)
   - Recursive type cycles

4. **IR Lowering**: Compiles the validated AST into a JTD-like IR. This is the "compiler target" - a compact, validation-friendly representation.

5. **Validator Runtime**: Consumes the IR and canonicalized values, performing recursive validation with path tracking.

6. **Registry System**: Supports local disk, HTTP(S), and composite schema registries with caching (kubeconform-inspired).

## Key Design Principles

### 1. Compile-Time Safety

Schemas are compiled to IR once and validated semantically. Runtime validation only checks document conformance, not schema validity.

### 2. Strict by Default

Unknown properties are rejected unless explicitly allowed via `@open` or `@mapRest`. Duplicate keys are rejected in strict mode. This follows kubeconform's strictness model.

### 3. Excellent Diagnostics

All errors include:
- `instancePath`: JSON Pointer to the location in the validated document
- `schemaPath`: JSON Pointer to the location in the schema
- Human-readable message with expected vs found
- Remediation hints where applicable

### 4. Portable Error Format

Error format mirrors JTD's standardized error indicators, enabling integration with tooling ecosystems.

### 5. Determinism

The same SDL source must always produce the same IR and the same validation results for a given document.

## What This Project Is NOT

This project intentionally avoids:

- **A GraphQL server/runtime**: No query execution, no resolver system, no subscription handling. SDL is only for schema definition.
- **Full GraphQL implementation**: Only supports a semantic subset of GraphQL SDL. Output types (objects, interfaces), query/mutation/subscription definitions, and extension types are not supported.
- **JSON Schema**: While inspired by JSON Schema's validation goals, the IR is JTD-like and simpler. JSON Schema export is optional and may be lossy.

## Relationship to Upstream Standards

### GraphQL Spec (October 2021)

SDL syntax follows GraphQL's type definition grammar. However, semantic interpretation differs:
- Input types are the primary object form (output types are not supported)
- Unions are repurposed for input validation unions
- Directives are custom validation annotations

### JTD RFC 8927

The IR is modeled on JTD's eight mutually-exclusive schema forms. This provides:
- Portable validation errors via JSON Pointers
- Code generation friendliness
- Explicit typing constrained to mainstream language type systems

**Note**: JTD is an Experimental RFC published on the Independent Submission stream. It is not IETF consensus, but provides a well-defined validation model.

### JSON Pointer RFC 6901

All error paths use JSON Pointer strings (`/foo/bar[0]/baz`). This ensures interoperability with tooling that follows the spec.

## Success Criteria

The project succeeds when:

1. A complete SDL subset compiles to IR without errors for the full workflow YAML example.
2. All IR variants (Scalar, Enum, Array, Object, Map, OneOf, DiscriminatedUnion, Ref, Any) are testable in isolation and composition.
3. Validation errors are reproducible and include stable `instancePath`/`schemaPath` pointers.
4. The registry subsystem supports local, HTTP, and composite sources with caching.
5. IR can be exported to JTD JSON for the representable subset.
6. Code coverage meets thresholds (target: 80%+ for core validation paths).
7. Fuzzing reveals no crashes or panics in parser or validator.

## Cross-Reference Links

- **[01-ir-design.md](./01-ir-design.md)**: Complete JTD-like IR specification as Rust enums
- **[02-sdl-grammar.md](./02-sdl-grammar.md)**: SDL subset, directive definitions, error messages
- **[03-compiler-lowering.md](./03-compiler-lowering.md)**: Lowering rules from SDL AST to IR
- **[04-validator-runtime.md](./04-validator-runtime.md)**: Validator algorithms and error reporting
- **[05-error-reporting.md](./05-error-reporting.md)**: Diagnostic format and UX
- **[06-registry-system.md](./06-registry-system.md)**: Local/HTTP/composite registries and caching
- **[07-cli-design.md](./07-cli-design.md)**: Command-line interface specification
- **[08-code-generation.md](./08-code-generation.md)**: Rust struct generation, JTD export, JSON Schema export
- **[09-testing-strategy.md](./09-testing-strategy.md)**: Unit/integration/property tests, fuzzing, coverage
- **[10-implementation-milestones.md](./10-implementation-milestones.md)**: Roadmap and effort estimates
- **[11-appendix.md](./11-appendix.md)**: Reference URLs, tooling links, examples

## Open Questions and Decisions Needed

1. **Regex library choice**: Which crate for `@pattern` validation? (options: `regex`, `fancy-regex`, `regex-lite`)
2. **YAML parser choice**: `serde-saphyr` (strict duplicate key handling, fastest, no unsafe) vs `serde_yaml_ng` (convenience, YAML 1.1)
3. **Async vs sync registry**: Should HTTP registry use async (`reqwest` vs `ureq`)?
4. **OneOf ambiguity threshold**: How many candidates to include in error hints? (suggested: top 3)
5. **IR serialization format**: Binary (bincode, MessagePack) vs JSON for caching compiled IR?
6. **Scalar coercion policy**: Strict mode (no coercion) vs limited coercion in open mode?

## Research Links

### SDL and GraphQL
- GraphQL Spec (October 2021): https://spec.graphql.org/October2021/
- GraphQL Spec (Draft, includes OneOf input objects): https://spec.graphql.org/draft/
- Apollo Rust GraphQL tooling (apollo-rs): https://github.com/apollographql/apollo-rs
- Apollo Parser docs.rs: https://docs.rs/apollo-parser/
- Apollo Parser crates.io: https://crates.io/crates/apollo-parser

### JTD and Validation
- RFC 8927 (JTD): https://datatracker.ietf.org/doc/html/rfc8927
- JTD validation errors guide: https://jsontypedef.com/docs/validation-errors/
- RFC 6901 (JSON Pointer): https://datatracker.ietf.org/doc/html/rfc6901

### Reference Tools
- kubeconform repo: https://github.com/yannh/kubeconform
- JTD ecosystem tooling: `jtd-derive`, `jtd` crates (for Rust code generation patterns)

### Example Context
- See "Example-driven SDL using your workflow YAML" section in the second research report for the complete workflow schema example.

### OpenCode Research Corrections
- [YAML Parser Analysis](../research/opencode/yaml-parser-analysis.md) — **Correction**: serde-saphyr replaces yaml-rust2. yaml-rust2 does NOT error on duplicate keys (silently overwrites). serde-saphyr has configurable DuplicateKeyPolicy::Error.
- [OpenCode Research Index](../research/opencode/README.md) — Full index of dependency research with version numbers and corrections.
