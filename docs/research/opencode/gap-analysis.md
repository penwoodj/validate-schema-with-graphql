# Gap Analysis: Upstream Factors for One-Shot Implementation Success

> OpenCode analysis, 2026-04-15. Based on ChatGPT reports + OpenCode research.

## Methodology

Reviewed all plan files (00-11) against ChatGPT research reports and OpenCode librarian findings. Identified gaps that could block or slow a one-shot implementation.

---

## CRITICAL Gaps (Must Address Before Implementation)

### Gap 1: No apollo-parser CST Walking Strategy
**Status**: Plan 02-sdl-grammar references apollo-parser but no plan file specifies HOW to walk the CST.

**Risk**: apollo-parser returns a Rowan-based CST. Walking it to extract type definitions, directives, and descriptions is non-trivial. The CST uses `SyntaxNode<TokenTree>` which requires cursor-based traversal, not simple struct field access.

**Missing**: 
- CST walker pattern (use apollo-parser's `api::Document` or raw cursor?)
- How to extract directive arguments (they're nested CST nodes)
- How to extract descriptions (StringValue node preceding type definition)
- How to handle source positions for error reporting

**Recommendation**: Plan 02-sdl-grammar needs a new section "CST Walking Strategy" with:
- Pattern: iterate `document.definitions()`, match on `Definition` enum variants
- For each input type: extract fields via `input_type_def.fields()`, then for each field extract type reference via `field.ty()`
- For directives: extract from `definition.directives()` on each definition node
- Source positions: available via `node.text_range()` converted to line/column

**Reference**: apollo-parser docs.rs has examples of CST traversal.

---

### Gap 2: Number Type Coercion Undefined
**Status**: Plan 04-validator-runtime defines `Number { Integer(i64), Float(f64) }` but doesn't specify coercion rules.

**Risk**: YAML parses `1` as integer and `1.0` as float. JSON same. But what happens when schema expects `Int` and YAML provides `1.0`? Or schema expects `Float` and YAML provides `1`?

**Missing**:
- Strict mode: `Int` schema + `1.0` value → error or coerce?
- Open mode: allow `1` for `Float` schema?
- What about large numbers (>i64)?
- What about scientific notation (`1e10`)?

**Recommendation**: Define explicitly:
- Strict: `Int` accepts only integer values; `Float` accepts both integer and float
- Open: `Int` coerces float-with-zero-fraction to int (e.g., `1.0` → `1`); `Float` accepts both
- Large numbers (>i64 max): reject in strict, accept as float in open

---

### Gap 3: @mapRest Validation Order Undefined
**Status**: Plan 03-compiler-lowering defines `@mapRest → AllowSchema` but doesn't specify validation order.

**Risk**: When validating `input ModelsSection @mapRest(value: ModelDefinition)`, do we validate known properties first, then rest? Or interleave? What if a known property and a rest key have the same name?

**Missing**:
- Priority: known properties override rest keys (or error on collision)?
- Order of validation errors: known properties first, then rest
- How to report which key triggered @mapRest validation

**Recommendation**: 
- Known properties always take priority
- If a rest key collides with a known property name → error (ambiguous)
- Validate known properties first, then validate remaining keys against @mapRest schema

---

### Gap 4: OneOf Ambiguity Detection Algorithm Undefined
**Status**: Plan 04-validator-runtime says "if 2+ match → ambiguous error" but doesn't define the algorithm for complex nested objects.

**Risk**: A step with `{generative_entity: "...", prompt: "..."}` clearly matches AgentStep. But what about `{generative_entity: "...", tool: "..."}` — is this ambiguous or does the longer match win?

**Missing**:
- Scoring algorithm: count matched required fields? Weight by field specificity?
- How to break ties: longest match? First match in variant list?
- Performance: OneOf with N variants requires N full validations. How to optimize for large N?

**Recommendation**:
- Score = count of validated fields / total fields in variant schema
- If multiple variants score 100% → ambiguous error with all matching variant names
- If one variant scores highest and >= 100% → that variant
- Short-circuit: if any required field is missing, skip variant early (before full validation)

---

### Gap 5: Recursive Type Cycle Detection Strategy
**Status**: Plan 03-compiler-lowering mentions "recursive type detection" but no algorithm.

**Risk**: A type like `input Node { children: [Node!]! }` is valid recursion. But `input A { b: B! } input B { a: A! }` creates a cycle. How to detect? How to handle during validation?

**Missing**:
- Detection algorithm: DFS with coloring (white/gray/black)?
- Are mutual recursion cycles allowed? (A→B→A)
- How to handle during validation: max depth? Error on cycle?

**Recommendation**:
- Use DFS with coloring during IR lowering
- Self-recursion (Node→Node) allowed — implement as Ref
- Mutual recursion cycles allowed — implement as Ref pair, detect during validation with max depth
- Max validation depth: configurable, default 64

---

## MODERATE Gaps (Should Address)

### Gap 6: CLI --schema-location Template Syntax
**Status**: Plan 07-cli-design mentions template-based URLs but doesn't define the template syntax.

**Missing**: Which template engine? Handlebars? Simple string replacement? What variables are available?

**Recommendation**: Use simple `{schema_id}` and `{schema_version}` placeholders (no template engine dependency). Kubeconform uses Go templates but that's overkill for our use case.

### Gap 7: Feature Flags Not Defined
**Status**: Plan 08-project-structure mentions `[features]` but doesn't specify them.

**Missing**: What features should exist? Examples:
- `yaml` — enable YAML parsing (default)
- `json` — enable JSON parsing (default)  
- `cli` — CLI binary (default for cli crate)
- `jtd-export` — JTD JSON export
- `http-registry` — HTTP schema fetching
- `miette` — rich diagnostics

**Recommendation**: Define feature flags in plan 08-project-structure.

### Gap 8: serde-saphyr Value → Canonical Value Conversion
**Status**: Plan 04-validator-runtime defines canonical `Value` but doesn't specify conversion from `serde_saphyr::Value` and `serde_json::Value`.

**Missing**: Conversion function signatures, loss handling, error propagation.

**Recommendation**: Add `From<serde_json::Value>` and `From<serde_saphyr::Value>` impls for canonical `Value`.

---

## LOW Gaps (Nice to Have)

### Gap 9: No Plan for Schema Versioning in SDL
**Status**: ChatGPT report mentions `schema_version` field in documents but no SDL syntax for declaring schema version.

**Recommendation**: Add a `@schemaVersion` directive or top-level comment convention. Low priority — can be handled by the registry layer.

### Gap 10: No Benchmark Targets Defined
**Status**: Plan 09-testing-strategy mentions criterion but no specific benchmark scenarios.

**Recommendation**: Define benchmarks for:
- SDL compile time (100-type schema)
- Validation time (10K-node document)
- Registry fetch + cache hit latency
- Strict vs open mode overhead

### Gap 11: No Minimum Rust Version (MSRV) Policy
**Status**: Plan 08-project-structure doesn't specify MSRV.

**Recommendation**: Set MSRV to Rust 1.75 (2023-12) — stable, widely available, supports all needed features (let chains in 1.77 if desired).

---

## Corrections Already Applied

1. ✅ YAML parser: yaml-rust2 → serde-saphyr (being revised in plan files)
2. ✅ JTD crate: not suitable as primary validator (documented in research)
3. ✅ apollo-compiler: accepts custom directives (documented in research)
4. ✅ serde_yaml: confirmed deprecated (documented in research)

## Summary

| Priority | Gaps | Action |
|---|---|---|
| **Critical** | 5 gaps | Must address before implementation starts |
| **Moderate** | 3 gaps | Should address during initial implementation |
| **Low** | 3 gaps | Can defer to post-MVP |

**Biggest risk**: CST walking strategy (Gap 1). This is the foundation of the entire SDL parsing pipeline and has the most uncertainty. Recommend prototyping before full implementation.
