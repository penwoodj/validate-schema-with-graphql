# SDL Grammar

## Overview

This document defines the subset of GraphQL SDL (Schema Definition Language) accepted by `graphql-ish-schema-validator`, including the complete directive specification.

The parser accepts full GraphQL syntax, but only validates and compiles the semantic subset defined here. All other GraphQL constructs are either ignored or produce errors.

## Supported SDL Subset

### What We Accept (Semantically)

The following SDL definitions are processed and compiled to IR:

1. **Scalar definitions**
   ```graphql
   scalar SemVer @pattern(regex: "^[0-9]+\\.[0-9]+\\.[0-9]+$")
   scalar Percent @pattern(regex: "^[0-9]+%$")
   ```

2. **Enum definitions**
   ```graphql
   enum BackoffStrategy {
     exponential
     linear
     fixed
   }
   ```

3. **Input object definitions** (primary record/object form)
   ```graphql
   input WorkflowDocument @closed {
     workflow_id: String!
     name: String!
     description: String
     models: ModelsSection!
   }
   ```

4. **Union definitions** (for input/validation unions)
   ```graphql
   union Step @oneOf = AgentStep | ToolStep | SubWorkflowStep
   union Workflow @discriminator(field: "kind") = AgentWorkflow | ToolWorkflow
   ```

5. **Descriptions** (docstrings)
   ```graphql
   """
   Root document matching unified-workflow-schema.yml.
   """
   input WorkflowDocument { ... }
   ```

6. **Directives** in const positions (on types and fields)

### What We Reject or Ignore (Do NOT Process)

The following GraphQL constructs are **not supported** and will produce errors:

- **Type/object/interface definitions** (output types)
  ```graphql
  # REJECTED - output types not supported
  type Query {
     workflow(id: ID!): Workflow
  }

  # REJECTED - interfaces not supported
  interface Node {
     id: ID!
  }
  ```

- **Query/Mutation/Subscription definitions**
  ```graphql
  # REJECTED - no resolver system
  schema {
     query: Query
     mutation: Mutation
  }
  ```

- **Extension types**
  ```graphql
  # REJECTED - extensions not supported
  extend input WorkflowDocument {
     extra_field: String
  }
  ```

- **Subscription/resolver types**: Not applicable (no runtime)

## Directive Specification

### Directive Definitions

All directives are defined with their argument types and valid attachment points.

```graphql
directive @closed on INPUT_OBJECT

directive @open on INPUT_OBJECT

directive @pattern(regex: String!) on SCALAR | INPUT_FIELD_DEFINITION

directive @default(value: String!) on INPUT_FIELD_DEFINITION

directive @oneOf on INPUT_OBJECT | UNION

directive @discriminator(field: String!) on UNION

directive @variant(tag: String!) on INPUT_OBJECT | UNION

directive @mapRest(value: TypeName!) on INPUT_OBJECT

directive @ref(name: String!) on INPUT_FIELD_DEFINITION | INPUT_OBJECT
```

### Directive Semantics

#### @closed

**Purpose**: Declares an object type closed to unknown keys (unless paired with `@mapRest`).

**Attachment**: `INPUT_OBJECT`

**Semantics**:
- In strict runtime mode: Unknown keys are rejected with error
- In open runtime mode: Still rejected (explicit override of default policy)

**Example**:
```graphql
input WorkflowDocument @closed {
  workflow_id: String!
  name: String!
}
# If document contains "unknown_key", validation fails.
```

**Relation to IR**: Maps to `Object { additional: Reject }`

---

#### @open

**Purpose**: Explicitly allow unknown keys in an object type.

**Attachment**: `INPUT_OBJECT`

**Semantics**:
- In strict runtime mode: Unknown keys allowed (exception to default)
- In open runtime mode: Unknown keys allowed (no change to default)

**Example**:
```graphql
input Config @open {
  known_field: String
}
# If document contains "unknown_key", validation passes.
```

**Relation to IR**: Maps to `Object { additional: AllowAny }`

---

#### @pattern(regex: String!)

**Purpose**: Apply regex constraint to scalar or string-like field.

**Attachment**: `SCALAR` or `INPUT_FIELD_DEFINITION`

**Semantics**:
- On scalars: All uses of that scalar must match the pattern
- On fields: Only that field's value must match the pattern
- Regex engine: Use `regex` crate (re2-compatible)

**Example**:
```graphql
scalar SemVer @pattern(regex: "^[0-9]+\\.[0-9]+\\.[0-9]+$")

input WorkflowDocument {
  # Pattern on field overrides scalar pattern
  version: String @pattern(regex: "^v[0-9]+\\.[0-9]+\\.[0-9]+$")
}
```

**Relation to IR**: Maps to `ScalarKind::String { pattern: Some(...) }` or `ScalarConstraints { pattern: Some(...) }`

---

#### @default(value: String!)

**Purpose**: Provide default value for optional fields.

**Attachment**: `INPUT_FIELD_DEFINITION`

**Semantics**:
- Field becomes optional (even if non-null type)
- Default is applied during canonicalization if field is missing
- Value must be parseable as the field's type

**Example**:
```graphql
input RequestDefaults {
  connection_timeout_secs: Int @default(value: "30")
  retry: RetryPolicy @default(value: "exponential")
}
```

**Relation to IR**: Field moves to `optional` map with default metadata

---

#### @oneOf

**Purpose**: Exactly one field must be set (on inputs) or exactly one variant matches (on unions).

**Attachment**: `INPUT_OBJECT` or `UNION`

**Semantics on INPUT_OBJECT**:
- Exactly one field must have a non-null value
- All other fields must be null/missing
- GraphQL-style oneOf semantics for input objects

**Semantics on UNION**:
- Validate instance against all union variants
- Exactly one variant must succeed (zero errors)
- If zero or multiple variants match → error

**Example on INPUT_OBJECT**:
```graphql
input RetryPolicy @oneOf {
  exponential: ExponentialBackoff
  linear: LinearBackoff
  fixed: FixedBackoff
}
# Valid: { exponential: { max_attempts: 3, ... } }
# Invalid: { exponential: {...}, linear: {...} } (two fields set)
```

**Example on UNION**:
```graphql
union Step @oneOf = AgentStep | ToolStep | SubWorkflowStep

# Valid instance matches exactly one of AgentStep/ToolStep/SubWorkflowStep
```

**Relation to IR**:
- On inputs: Wrapped in `OneOf` with per-field variants
- On unions: Compiles to `OneOf([variant_schemas...])`

---

#### @discriminator(field: String!)

**Purpose**: Build a discriminated union (tagged union) using a field value.

**Attachment**: `UNION`

**Semantics**:
- Instance must contain the discriminator field
- Discriminator value must be a string
- Value determines which variant schema to use

**Example**:
```graphql
union Workflow @discriminator(field: "kind")
  = AgentWorkflow
  | ToolWorkflow
  | ControlWorkflow

# Valid: { kind: "agent", ... } → validates as AgentWorkflow
# Invalid: { kind: "unknown", ... } → error (unknown tag)
```

**Relation to IR**: Maps to `DiscriminatedUnion { discriminator, mapping }`

---

#### @variant(tag: String!)

**Purpose**: Associate a tag value with a variant in a discriminated union.

**Attachment**: `INPUT_OBJECT` or `UNION`

**Semantics**:
- Used with `@discriminator` on the parent union
- Defines which tag value maps to which variant

**Example**:
```graphql
input AgentWorkflow @variant(tag: "agent") {
  model: String!
  prompt: String!
}

input ToolWorkflow @variant(tag: "tool") {
  tool: String!
  input: Any
}

union Workflow @discriminator(field: "kind")
  = AgentWorkflow
  | ToolWorkflow

# Mapping: "agent" → AgentWorkflow, "tool" → ToolWorkflow
```

**Relation to IR**: Populates `DiscriminatedUnion.mapping[tag]`

---

#### @mapRest(value: TypeName!)

**Purpose**: Validate unknown keys (the "rest") against a schema.

**Attachment**: `INPUT_OBJECT`

**Semantics**:
- Known keys (required/optional) validate normally
- Unknown keys validate against the specified schema type
- This is the KEY extension beyond JTD

**Example**:
```graphql
input ModelDefinition @closed {
  name: String!
  provider: String!
}

input ModelsSection @closed @mapRest(value: ModelDefinition) {
  global_config_path: String
  default_router: String
}
# Valid:
# models:
#   global_config_path: "./config.yml"
#   gpt-4:                    # Unknown key → validates as ModelDefinition
#     name: gpt-4
#     provider: lmstudio
#   llama-3.2:                # Unknown key → validates as ModelDefinition
#     name: llama-3.2
#     provider: ollama
```

**Relation to IR**: Maps to `Object { additional: AllowSchema(Box::new(Schema::Ref { name: "ModelDefinition" })) }`

---

#### @ref(name: String!)

**Purpose**: Explicit schema reference (advanced feature).

**Attachment**: `INPUT_FIELD_DEFINITION` or `INPUT_OBJECT`

**Semantics**:
- References a named schema in the bundle
- Enables registry lookups or JSON Pointer references
- Most use cases covered by SDL type names + union

**Example**:
```graphql
input Config @ref(name: "$schema_registry/production/v1")
```

**Relation to IR**: Compiles to `Schema::Ref { name: "..." }`

---

## Full SDL Example: Workflow YAML

The following complete SDL models the unified workflow YAML, demonstrating all supported features:

```graphql
"""
Root document matching unified-workflow-schema.yml.
"""
input WorkflowDocument @closed {
  workflow_id: String!
  name: String!
  description: String!

  version: SemVer!
  author: String!
  tags: [String!]!

  providers: Providers!

  """
  models has fixed keys plus arbitrary model entries.
  Implement with @mapRest.
  """
  models: ModelsSection!

  sub_workflows: SubWorkflows!

  agentic_workflow: AgenticWorkflow!
  workflow_execution_strategy: ExecutionStrategy!
  tool_permissions: ToolPermissions!
  memory: MemoryConfig!
  workspace: WorkspaceConfig!

  schema_version: SemVer!
  min_schema_version: SemVer!
}

scalar SemVer @pattern(regex: "^[0-9]+\\.[0-9]+\\.[0-9]+$")
scalar Percent @pattern(regex: "^[0-9]+%$")
scalar HumanSize @pattern(regex: "^[0-9]+(\\.[0-9]+)?(KB|MB|GB|TB)$")
scalar Duration @pattern(regex: "^[0-9]+(ms|s|m|h|d)$")
scalar TemplateRef @pattern(regex: "^\\$\\{[^}]+\\}$")

enum BackoffStrategy {
  exponential
  linear
  fixed
}

input Providers @closed {
  lmstudio: LmstudioProvider
  ollama: FileRefProvider
  llama_cpp_with_vulkan: FileRefProvider
}

input FileRefProvider @closed {
  config_file: String!
}

input LmstudioProvider @closed {
  config: LmstudioConfig
}

input LmstudioConfig @closed {
  host: String
  requests: RequestDefaults
}

input RequestDefaults @closed {
  connection_timeout_secs: Int @default(value: "30")
  retry: RetryPolicy
}

input RetryPolicy @closed {
  max_attempts: Int @default(value: "3")
  initial_delay_secs: Int @default(value: "1")
  backoff: BackoffStrategy @default(value: "exponential")
}

"""
Fixed keys plus arbitrary model entries:
everything not explicitly listed must validate as ModelDefinition.
"""
input ModelsSection @closed @mapRest(value: ModelDefinition) {
  global_config_path: String
  default_router: String
}

input ModelDefinition @closed {
  name: String!
  provider: String!

  cache_policy: String
  ram_allocation: Percent
  vram_allocation: HumanSize

  # Reference to another model by template string
  parent_model: TemplateRef
}

input SubWorkflows @closed {
  analyze_workflow: String!
  validate_workflow: InlineWorkflow
  fix_workflow: ExternalWorkflowWithDefaults
}

input InlineWorkflow @closed {
  inputs: [InputDef!]!
  steps: StepMap!
}

input ExternalWorkflowWithDefaults @closed {
  path: String!
  default_inputs: [InputDef!]
}

input InputDef @closed {
  name: String!
  type: String!
  required: Boolean @default(value: "false")
  default: String
}

"""
A map keyed by step name. Implement as a map/dictionary in IR.
"""
input StepMap @closed @mapRest(value: Step) {
}

"""
Step is a shape-based union. Exactly one variant should match.
"""
union Step @oneOf = AgentStep | ToolStep | SubWorkflowStep | LoopStep | ControlFlowStep

input AgentStep @closed {
  generative_entity: String!
  prompt: String!
  model_overrides: ModelOverrides
  user_input: UserInput
  retry: RetryPolicy
  depends_on: [String!]
  when: StepWhen
  requires: [Requirement!]
  parallel_group: String
}

input ToolStep @closed {
  tool: String!
  input: Any
  depends_on: [String!]
  when: StepWhen
}

input SubWorkflowStep @closed {
  sub_workflow: String!
  input: Any
  when: StepWhen
}

input LoopStep @closed {
  loop: LoopSpec!
  sub_workflow: String!
  input: Any
  when: StepWhen
}

input ControlFlowStep @closed {
  when: StepWhen!
  requires: [Requirement!]
  on_requires_failed: ControlAction
}

input ModelOverrides @closed {
  model: TemplateRef
  temperature: Float
  top_p: Float
}

input UserInput @closed {
  prompt_user: String!
  input_type: String
  required: Boolean @default(value: "false")
}

input StepWhen @closed {
  after_step_fails: String
  after_step_succeeds: String
  always: Boolean @default(value: "false")
}

input Requirement @closed {
  # Example union: ref string or inline criteria object
  ref: String
  exact_criteria: Criteria
}

input Criteria @closed {
  operator: String @pattern(regex: "^(==|!=|>=|<=|>|<)$")
  value: String
}

enum ControlAction {
  abort
  skip
  continue
}

input LoopSpec @closed {
  collection_var: String
  item_var: String
  max_iterations: Int @default(value: "100")
}

scalar Any
```

## Error Messages for Invalid SDL Usage

The following error messages should be produced for common SDL mistakes:

### Unknown Directive
```
error: Unknown directive '@invalid_directive'
  --> schema.graphql:5:3
   |
 5 | input Foo @invalid_directive { }
   |              ^^^^^^^^^^^^^^^^^ unknown directive

Valid directives: @closed, @open, @pattern, @default, @oneOf, @discriminator, @variant, @mapRest, @ref
```

### Directive Wrong Attachment Point
```
error: Directive '@closed' can only be used on INPUT_OBJECT definitions
  --> schema.graphql:3:8
   |
 3 | scalar String @closed
   |               ^^^^^^ invalid attachment point for '@closed'

Expected attachment points: INPUT_OBJECT
```

### Directive Missing Required Argument
```
error: Directive '@pattern' requires argument 'regex: String!'
  --> schema.graphql:7:18
   |
 7 |   pattern: String @pattern
   |                     ^^^^^^^ missing required argument 'regex'
```

### Duplicate Type Name
```
error: Duplicate type definition 'WorkflowDocument'
  --> schema.graphql:10:1
   |
10 | input WorkflowDocument { }
    | ^^^^^^^^^^^^^^^^^^^^^ 'WorkflowDocument' already defined at line 1
```

### Undefined Type Reference
```
error: Type 'UndefinedType' is not defined in this schema
  --> schema.graphql:15:25
   |
15 |   field: UndefinedType!
    |                  ^^^^^^^^^^^^^^^^ type not found
```

### Union Member Not Defined
```
error: Union member 'MissingType' is not a defined type
  --> schema.graphql:20:20
   |
20 | union Example = DefinedType | MissingType
    |                     ^^^^^^^^^^^^^^ type not found
```

### Discriminator on Non-Union
```
error: Directive '@discriminator' can only be used on UNION definitions
  --> schema.graphql:25:1
   |
25 | input Input @discriminator(field: "kind") { }
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ invalid attachment point
```

### OneOf with Required Fields
```
warning: @oneOf input object has required fields; this may cause validation to always fail
  --> schema.graphql:30:7
   |
30 |   field: String!
    |         ^^^^^^^^ non-null required field in @oneOf input

In a @oneOf input, exactly one field must be set. Required fields make this impossible.
```

### Recursive Type Without Ref
```
error: Direct recursion without @ref is not allowed
  --> schema.graphql:35:1
   |
35 | input Node {
   | ^^^^^^^^^^^^ 'Node' references itself directly

Use @ref to create recursive references: field: Node @ref(name: "Node")
```

## Cross-Reference Links

- **[01-ir-design.md](./01-ir-design.md)**: IR types that SDL constructs map to (e.g., `@closed` → `Reject`, `@mapRest` → `AllowSchema`)
- **[03-compiler-lowering.md](./03-compiler-lowering.md)**: Grammar → IR lowering rules (e.g., input → Object, union → DiscriminatedUnion/OneOf)
- **[05-error-reporting.md](./05-error-reporting.md)**: SDL error message formatting and UX

## Open Questions and Decisions Needed

1. **GraphQL default literal syntax vs @default directive**: Support both `field: Type = "value"` and `@default(value: "value")`? (recommended: pick one canonical form, prefer GraphQL default syntax)
2. **Directive argument type constraints**: Should we validate regex patterns at parse time? (suggested: yes, try to compile regex; error if invalid)
3. **@oneOf on unions vs input objects**: Should semantics be identical or slightly different? (recommendation: identical - exactly one must match)
4. **Description propagation**: Should GraphQL descriptions (docstrings) be preserved in IR for diagnostics? (recommended: yes, include in IR metadata)
5. **Custom scalar naming**: Should custom scalars be namespaced or global? (recommendation: global in schema bundle, collision detection)
6. **Directive repetition**: Can directives be repeated (e.g., multiple `@pattern`)? (recommendation: no, error on repetition)

## Gap Fix: CST Walking Strategy

The apollo-parser CST (Concrete Syntax Tree) uses Rowan-based `SyntaxNode<TokenTree>` which requires cursor-based traversal, not simple struct field access.

### CST Walker Pattern

**Basic pattern**: Parse SDL, get `SyntaxTree`, iterate `document.definitions()`:

```rust
use apollo_parser::Parser;

fn walk_cst(sdl: &str) -> Result<(), Error> {
    let parser = Parser::new(sdl);
    let tree = parser.parse();

    // Check for parse errors first
    if !tree.errors().is_empty() {
        // Report parse errors before walking
        return Err(format_parse_errors(tree.errors()));
    }

    // Iterate through all top-level definitions
    for def in tree.document().definitions() {
        match def {
            Definition::ScalarTypeDefinition(scalar_def) => {
                let name = scalar_def.name();
                let directives = scalar_def.directives();
                // Process scalar...
            }
            Definition::EnumTypeDefinition(enum_def) => {
                let name = enum_def.name();
                let values = enum_def.values();
                for val in values {
                    let enum_value = val.value();
                    // Process enum value...
                }
            }
            Definition::InputObjectTypeDefinition(input_def) => {
                let name = input_def.name();
                let description = input_def.description(); // Option<Description>
                let fields = input_def.fields();
                for field in fields {
                    let field_name = field.name();
                    let field_ty = field.ty(); // Type reference
                    let default_value = field.default_value();
                    let field_directives = field.directives();
                    // Process field...
                }
                let directives = input_def.directives();
                for dir in directives {
                    let dir_name = dir.name();
                    let args = dir.arguments();
                    for arg in args {
                        let arg_name = arg.name();
                        let arg_value = arg.value();
                        // Process directive argument...
                    }
                }
            }
            Definition::UnionTypeDefinition(union_def) => {
                let name = union_def.name();
                let members = union_def.members();
                for member in members {
                    let member_name = member.name();
                    // Process union member...
                }
            }
            _ => {} // Ignore other definition types (output types, etc.)
        }
    }

    Ok(())
}
```

### Key CST Operations

**For InputObjectTypeDefinition**:
- `def.name()` → type name as string
- `def.description()` → `Option<Description>` for docstrings
- `def.fields()` → iterator over `InputValueDefinition`
- For each field: `field.name()`, `field.ty()`, `field.default_value()`, `field.directives()`

**For Directive extraction**:
- `definition.directives()` → iterator over directives
- For each directive: `dir.name()`, `dir.arguments()` → iterator over arguments
- For each argument: `arg.name()`, `arg.value()`

**For Description extraction**:
- `definition.description()` → `Option<Description>` (use `.map(|d| d.text())` to get string)

**For Source positions**:
- `node.text_range()` gives byte range for error reporting
- Or use `node.syntax().text_range()` for CST nodes
- Convert to line/column for diagnostics

**For Scalar definitions**:
- `Definition::ScalarTypeDefinition(scalar_def)` variant
- Extract: `scalar_def.name()`, `scalar_def.directives()`

**For Enum definitions**:
- `Definition::EnumTypeDefinition(enum_def)` variant
- `enum_def.values()` → iterator over enum values
- For each: `val.value()` → string enum value

**For Union definitions**:
- `Definition::UnionTypeDefinition(union_def)` variant
- `union_def.members()` → iterator over union members
- For each: `member.name()` → member type name

### Error Handling

**Critical**: Check `tree.errors()` first before walking:

```rust
let parser = Parser::new(sdl);
let tree = parser.parse();

// Report parse errors before walking
if !tree.errors().is_empty() {
    for error in tree.errors() {
        eprintln!("Parse error at {}: {}", error.location(), error.message());
    }
    return Err(CompileError::ParseFailed);
}

// Safe to walk CST after verifying no parse errors
walk_document(tree.document())?;
```

## Research Links

### SDL and GraphQL
- GraphQL Spec (October 2021): https://spec.graphql.org/October2021/
- GraphQL Spec (Draft, includes OneOf input objects): https://spec.graphql.org/draft/
- Apollo Parser docs.rs: https://docs.rs/apollo-parser/

### SDL to IR Directives
- See "Directive set and semantics" section in second research report for full directive specification.
- See "Example-driven SDL using your workflow YAML" section for complete SDL example.

### Error Reporting
- See miette diagnostics library: https://docs.rs/miette/ (for structured error formatting)
