# Implementation Milestones

## Overview

This document defines the implementation milestones for `graphql-ish-schema-validator`, organized by phase with deliverables, dependencies, and estimated effort for a solo developer.

The milestones follow a logical dependency order: foundations → parsing → compilation → validation → advanced features → hardening → exports.

Total estimated effort: **15-24 weeks** for a solo developer working full-time.

## Phase 1: Foundations (1-2 weeks)

### Goals
- Set up workspace structure and CI pipeline
- Define core types (IR, Value, JsonPointer)
- Establish testing infrastructure

### Deliverables
- [ ] Workspace compiles with all 8 crates
- [ ] CI pipeline passes (build, clippy, fmt, test)
- [ ] IR types defined (`Schema`, `ScalarKind`, `EnumSchema`, `ArraySchema`, `ObjectSchema`, `MapSchema`, `DiscriminatedUnionSchema`, `OneOfSchema`, `RefSchema`)
- [ ] Value type defined (`Value` enum: `Null`, `Bool`, `Number`, `String`, `Array`, `Object`)
- [ ] JsonPointer implemented with push_segment, render, parse methods

### Tasks
1. Create workspace `Cargo.toml` with all members
2. Create individual crate `Cargo.toml` files
3. Define `Schema` enum in `crates/ir/src/schema.rs`
4. Implement `JsonPointer` in `crates/ir/src/pointer.rs`
5. Implement `Value` enum in `crates/validator/src/value.rs`
6. Write unit tests for IR types
7. Write unit tests for JsonPointer (escaping, roundtrip, equality)
8. Set up GitHub Actions CI workflow

### Tests
- Unit tests for IR construction and serialization
- Unit tests for JsonPointer operations (escaping, parsing, equality)
- CI builds, tests, clippy, fmt on push

### Dependencies
- None (foundational phase)

### Estimated Effort
1-2 weeks

## Phase 2: SDL Parsing (2-3 weeks)

### Goals
- Integrate `apollo-parser` for SDL parsing
- Build minimal AST for GraphQL subset
- Implement semantic validation

### Deliverables
- [ ] Parse SDL string → CST + error list
- [ ] Build custom AST from CST (scalar, enum, input, union, directives)
- [ ] Semantic validation (unknown directives, duplicate types, missing refs)
- [ ] Parse all supported directives (@closed, @open, @pattern, @default, @oneOf, @discriminator, @mapRest, @variant)

### Tasks
1. Add `apollo-parser` dependency to `sdl-parser` crate
2. Implement `parse_sdl()` returning CST + errors
3. Define AST types (`SdlScalar`, `SdlEnum`, `SdlInputObject`, `SdlUnion`, `SdlDirective`)
4. Implement CST → AST conversion
5. Implement semantic validation pass:
   - Unknown directives error
   - Duplicate type names error
   - Referenced types must exist
   - Union members must exist
6. Write unit tests:
   - Valid SDL → AST
   - Invalid SDL → errors
   - Directive parsing
7. Add integration test parsing workflow schema

### Tests
- Valid SDL for each construct (scalar, enum, input, union)
- Invalid SDL produces descriptive errors
- Directive parsing and validation
- AST roundtrip with expected fields

### Dependencies
- Phase 1 (workspace, CI)

### Estimated Effort
2-3 weeks

## Phase 3: Compiler / Lowering (2-3 weeks)

### Goals
- Implement AST → IR lowering
- Perform semantic validation during lowering
- Generate stable schema paths for error reporting

### Deliverables
- [ ] Compile SDL → IR bundle
- [ ] Detect all schema errors (semantic issues)
- [ ] Generate stable `schemaPath` JSON Pointers for each schema node

### Tasks
1. Define `SchemaBundle` type (map of type names to `Schema`)
2. Implement lowering rules:
   - Scalar → IR (handle built-in vs custom scalars, @pattern)
   - Enum → IR (list of values)
   - Input object → IR (required/optional separation, @closed/@open)
   - Union → IR (discriminator vs @oneOf)
   - Directives → IR extensions
3. Implement name resolution (refs point to existing schemas)
4. Detect recursion cycles in refs
5. Generate stable schema paths (for error reporting)
6. Write unit tests:
   - Each SDL construct → expected IR
   - Error detection for invalid schemas
   - Schema path generation
7. Add integration test lowering workflow schema

### Tests
- Each SDL construct maps to expected IR variant
- Directives translate correctly
- Required vs optional field detection
- Union with discriminator vs @oneOf produces different IR
- Schema path generation is stable

### Dependencies
- Phase 2 (SDL parsing, AST)

### Estimated Effort
2-3 weeks

## Phase 4: Validator MVP (2-3 weeks)

### Goals
- Implement recursive descent validator for core IR variants
- Support strict and open modes
- Validate YAML/JSON values against compiled IR

### Deliverables
- [ ] Validate YAML/JSON values against compiled IR
- [ ] Support all IR variants except OneOf/DiscriminatedUnion
- [ ] Strict mode (reject unknown keys) vs open mode (accept unknown keys)
- [ ] JSON Pointer error paths (instancePath, schemaPath)

### Tasks
1. Implement validate function with mode parameter
2. Implement validation for:
   - Scalar (type check, @pattern for strings)
   - Enum (value in list)
   - Array (validate elements)
   - Object (required/optional/additional)
   - Map (validate values)
   - Ref (resolve, detect cycles)
3. Implement strict/open mode behavior:
   - Strict: unknown keys rejected
   - Open: unknown keys accepted
4. Track instancePath and schemaPath during validation
5. Collect validation errors with context
6. Write unit tests:
   - Per-variant validation
   - Strict vs open mode differences
   - Error paths (instancePath, schemaPath)
7. Add integration test validating simple YAML document

### Tests
- Scalar/Enum/Object/Array/Map/Ref validation
- Type mismatches produce errors
- Required fields detection
- Optional fields validated when present
- Additional keys rejected based on mode

### Dependencies
- Phase 3 (IR lowering, SchemaBundle)

### Estimated Effort
2-3 weeks

## Phase 5: Advanced Validation (1-2 weeks)

### Goals
- Implement OneOf validation with ambiguity detection
- Implement DiscriminatedUnion validation
- Implement default value application

### Deliverables
- [ ] OneOf validation (exactly one variant matches)
- [ ] DiscriminatedUnion validation (discriminator resolves to variant)
- [ ] Default values applied when fields missing
- [ ] Ambiguity detection for OneOf

### Tasks
1. Implement OneOf validation:
   - Try each variant schema
   - Count matching candidates
   - Error: no variants match
   - Error: multiple variants match (ambiguous)
   - Provide remediation hints for ambiguity
2. Implement DiscriminatedUnion validation:
   - Extract discriminator field
   - Map to variant schema
   - Validate using mapped schema
3. Implement default value application:
   - Apply defaults during validation
   - Merge defaults with instance values
4. Write unit tests:
   - OneOf edge cases (ambiguous, no match, exact match)
   - Discriminator resolution
   - Default application
5. Add integration test with workflow document

### Tests
- OneOf edge cases (ambiguous variants, no matches)
- Discriminator field resolution
- Default values applied correctly
- Full workflow document validation

### Dependencies
- Phase 4 (Validator MVP)

### Estimated Effort
1-2 weeks

## Phase 6: Error Reporting (1-2 weeks)

### Goals
- Integrate `miette` for rich error reporting
- Support JSON output for machine consumers
- Provide human-readable error messages

### Deliverables
- [ ] Excellent error messages with context
- [ ] JSON error output format
- [ ] GitHub Actions annotation format
- [ ] Snapshot tests for error output

### Tasks
1. Integrate `miette` into error types
2. Define error categories (syntax, semantic, validation)
3. Implement error formatting:
   - JSON output (`instancePath`, `schemaPath`, `code`, `message`, `hint`)
   - Human-readable output (multiline with context)
   - GitHub Actions format (`::error file=line,col::message`)
4. Add hints for common errors
5. Set up snapshot testing with `insta`
6. Write snapshot tests for all error types
7. Test error output against invalid documents

### Tests
- Snapshot tests for all error types
- JSON output validation
- Human-readable output review
- GitHub Actions format testing

### Dependencies
- Phase 5 (Full validation)

### Estimated Effort
1-2 weeks

## Phase 7: Registry & Caching (1-2 weeks)

### Goals
- Implement schema registries (local, HTTP, composite)
- Add disk and memory caching
- Support template-based schema discovery

### Deliverables
- [ ] Local registry (filesystem resolution)
- [ ] HTTP registry (fetch with timeout)
- [ ] Composite registry (try sources in order)
- [ ] Disk cache for remote schemas
- [ ] Memory cache for compiled IR

### Tasks
1. Define `Registry` trait
2. Implement `LocalRegistry`:
   - Read from filesystem
   - Support schema discovery template
3. Implement `HttpRegistry`:
   - Fetch via `reqwest`
   - Enforce timeouts and size limits
   - Optional domain allowlist
   - TLS verification options
4. Implement `CompositeRegistry`:
   - Try registries in order
   - Return first successful result
5. Implement disk cache:
   - Cache remote schemas to disk
   - Respect cache headers (optional)
6. Implement memory cache:
   - LRU cache for compiled IR
   - Configurable size
7. Write unit tests:
   - Local registry resolution
   - HTTP registry fetching
   - Cache behavior (hit vs miss)
8. Add integration tests with `mockito` or `wiremock`

### Tests
- Local filesystem resolution
- HTTP registry with mocked responses
- Composite registry tries sources in order
- Disk cache prevents duplicate fetches
- Memory cache eviction policy

### Dependencies
- Phase 3 (IR bundle)

### Estimated Effort
1-2 weeks

## Phase 8: CLI (1 week)

### Goals
- Build CLI binary with subcommands
- Implement flags and options
- Handle exit codes

### Deliverables
- [ ] Usable CLI binary
- [ ] Subcommands: validate, export-jtd, compile
- [ ] Flags: schema, document, output, strict, open, cache
- [ ] Proper exit codes (0=success, 1=validation_error, 2=internal_error)

### Tasks
1. Set up `cli` crate with `clap` derive
2. Implement subcommands:
   - `validate`: validate document against schema
   - `export-jtd`: export IR to JTD JSON
   - `compile`: compile SDL to IR (for debugging)
3. Add flags:
   - `--schema`: schema file or ID
   - `--document`: document to validate
   - `--output`: output file
   - `--strict`/`--open`: validation mode
   - `--cache`: cache directory
4. Implement exit codes:
   - 0: success
   - 1: validation errors
   - 2: internal error (missing file, parse error, etc.)
5. Write integration tests with `assert_cmd`
6. Test CLI with various inputs

### Tests
- CLI integration tests for each subcommand
- Exit code verification
- Flag handling
- Error output formatting

### Dependencies
- Phase 6 (Error reporting)
- Phase 7 (Registry)

### Estimated Effort
1 week

## Phase 9: Hardening (2-3 weeks)

### Goals
- Add property-based tests
- Implement fuzz targets
- Achieve coverage targets
- Establish benchmarks

### Deliverables
- [ ] >80% code coverage
- [ ] No panics on fuzz corpus
- [ ] Property tests for key invariants
- [ ] Benchmark suite for performance regression

### Tasks
1. Implement property tests with `proptest`:
   - Validator terminates for all (Value, Schema) pairs
   - Strict is subset of open validation
   - JsonPointer roundtrip
   - IR serialization roundtrip
2. Implement fuzz targets with `cargo-fuzz`:
   - SDL parser fuzz target
   - YAML parser fuzz target
   - Full pipeline fuzz target (SDL + document)
3. Run coverage with `cargo-llvm-cov`:
   - Aim for >80% line coverage
   - Identify and cover uncovered paths
4. Add benchmark suite with `criterion`:
   - SDL compilation vs schema size
   - Validation vs document size
   - Strict vs open mode
   - Registry cache hit vs miss
5. Set up CI for:
   - Fuzzing on cron schedule
   - Coverage reporting
   - Benchmark regression detection
6. Address all findings from property tests and fuzzing

### Tests
- Property tests for invariants
- Fuzz targets crash-free on corpus
- Coverage meets 80% target
- Benchmarks run successfully

### Dependencies
- All previous phases

### Estimated Effort
2-3 weeks

## Phase 10: Exports (1-2 weeks)

### Goals
- Implement IR → JTD JSON export
- Handle lossy exports with warnings
- Document export limitations

### Deliverables
- [ ] IR → JTD JSON export
- [ ] Export subcommand in CLI
- [ ] Warning/error reporting for unsupported features
- [ ] Documentation of JTD limitations

### Tasks
1. Implement IR → JTD mapping:
   - Map IR variants to JTD forms
   - Handle lossy features (pattern, @mapRest, @default, OneOf)
2. Implement error handling:
   - Errors: cannot export (e.g., OneOf)
   - Warnings: fidelity loss (e.g., pattern dropped)
3. Add export CLI subcommand
4. Write tests:
   - Roundtrip for JTD-representable subset
   - Error on non-representable features
   - Warnings for lossy exports
5. Document limitations in README

### Tests
- Roundtrip for JTD-representable IR
- Errors on non-representable features
- Warnings for lossy exports
- CLI integration

### Dependencies
- Phase 3 (IR)
- Phase 6 (Error reporting)

### Estimated Effort
1-2 weeks

## Dependencies Between Phases

### Dependency Graph

```
Phase 1 (Foundations)
└── Phase 2 (SDL Parsing)
    └── Phase 3 (Compiler / Lowering)
        ├── Phase 4 (Validator MVP)
        │   └── Phase 5 (Advanced Validation)
        │       └── Phase 6 (Error Reporting)
        └── Phase 7 (Registry & Caching)
            └── Phase 8 (CLI)
                └── Phase 9 (Hardening)
Phase 3 ────────────────────────└── Phase 10 (Exports)
```

### Critical Path

The critical path is:
1. Foundations → Parsing → Compiler → Validator → Advanced Validation → Error Reporting → CLI

**Parallelizable**:
- Registry can be implemented in parallel with Validator (after Compiler)
- Hardening runs after all features complete
- Exports can start after IR is defined (after Phase 3)

### Blocking Relationships

| Phase | Blocks | Reason |
|-------|---------|---------|
| 1 | 2-10 | All phases depend on workspace and types |
| 2 | 3 | Lowering requires AST |
| 3 | 4,7,10 | Validator, Registry, Exports need IR |
| 4 | 5 | Advanced validation builds on MVP |
| 5 | 6 | Error reporting needs full validation |
| 6 | 8 | CLI uses error formatting |
| 7 | 8 | CLI uses registry |

## Estimated Effort Summary

| Phase | Duration | Dependencies | Parallelizable? |
|--------|-----------|---------------|-----------------|
| 1: Foundations | 1-2 weeks | None | No |
| 2: SDL Parsing | 2-3 weeks | 1 | No |
| 3: Compiler / Lowering | 2-3 weeks | 2 | No |
| 4: Validator MVP | 2-3 weeks | 3 | No |
| 5: Advanced Validation | 1-2 weeks | 4 | No |
| 6: Error Reporting | 1-2 weeks | 5 | No |
| 7: Registry & Caching | 1-2 weeks | 3 | Yes (with 4-6) |
| 8: CLI | 1 week | 6,7 | No |
| 9: Hardening | 2-3 weeks | 8 | No |
| 10: Exports | 1-2 weeks | 3,6 | Yes (with 7-8) |

### Solo Developer Timeline

**Optimistic**: 15 weeks (3.75 months)
- Assume 1 week per phase minimum
- No delays, perfect parallelization

**Realistic**: 20-24 weeks (5-6 months)
- Allow for debugging, edge cases
- Some sequential work in parallelizable phases
- Buffer for unforeseen issues

**Conservative**: 30 weeks (7.5 months)
- Slower pace
- More testing and refinement
- Documentation and examples

## Cross-References

- **`01-ir-design.md`** - IR types (Phase 1, 3, 10)
- **`02-sdl-parsing.md`** - SDL parsing details (Phase 2)
- **`03-compiler-lowering.md`** - Lowering algorithm (Phase 3)
- **`04-validator-runtime.md`** - Validation algorithms (Phase 4, 5)
- **`05-registry-subsystem.md`** - Registry architecture (Phase 7)
- **`06-error-reporting.md`** - Error formatting (Phase 6)
- **`08-project-structure.md`** - Workspace layout (Phase 1)
- **`09-testing-strategy.md`** - Testing approach (Phase 9)
- **`10-jtd-export.md`** - Export details (Phase 10)

## Open Questions / Decisions Needed

1. **Phase 7 Parallelization**: Should Registry be implemented before or in parallel with Validator? Dependency suggests parallel, but Validator may be prioritized.

2. **Phase 10 Timing**: Should Exports start after Phase 3 (IR ready) or wait until after CLI? Starting earlier may reveal IR design issues.

3. **Hardening Duration**: Is 2-3 weeks sufficient for hardening? May need more time if fuzzing finds deep issues.

4. **Documentation**: When to write user documentation (README, examples, tutorial)? Can be done alongside implementation or as separate phase.

5. **Release Criteria**: What defines "done" for v0.1.0 release? All 10 phases, or MVP (Phases 1-8) with exports as v0.2.0?

## Research Links

### Implementation References
- [GraphQL Spec October 2021](https://spec.graphql.org/October2021/)
- [Apollo Parser docs.rs](https://docs.rs/apollo-parser/)
- [RFC 8927 - JTD](https://datatracker.ietf.org/doc/html/rfc8927)
- [RFC 6901 - JSON Pointer](https://datatracker.ietf.org/doc/html/rfc6901)

### Testing Tools
- [proptest docs.rs](https://docs.rs/proptest/)
- [cargo-fuzz book](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)
- [criterion docs.rs](https://bheisler.github.io/criterion.rs/book/)

### Project References
- [kubeconform](https://github.com/yannh/kubeconform) - Registry and cache patterns
- [thiserror docs.rs](https://docs.rs/thiserror/)
- [miette docs.rs](https://docs.rs/miette/)
- [clap docs.rs](https://docs.rs/clap/)
