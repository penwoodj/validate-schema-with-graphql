# apollo-parser Dependency Analysis

> Research by OpenCode librarian, 2026-04-15

## Verdict: USE apollo-parser (not apollo-compiler, not graphql-parser)

## Comparison Table

| Feature | apollo-parser | apollo-compiler | graphql-parser |
|---|---|---|---|
| **Version** | 0.8.5 (2026-02-25) | 1.31.1 (2026-02-25) | 0.4.1 (2024-05-19) |
| **Maintenance** | ✅ Active (Apollo team) | ✅ Active (Apollo team) | ⚠️ Stale (last commit 2025-01) |
| **Downloads (90d)** | 77,274 | 51,991 | Lower |
| **Error Recovery** | ✅ Always returns CST + errors | ✅ Same (uses apollo-parser) | ❌ Returns Result, fails fast |
| **Output Type** | CST (Rowan-based) | Schema/AST (high-level) | AST (enum-based) |
| **Custom Directives** | ✅ Preserved in CST | ✅ Preserved in Schema | ✅ Preserved in AST |
| **Spec Compliance** | October 2021 | October 2021 + validation | Full + extensions |
| **Binary Size** | ~200KB | ~330KB | Unknown |

## Key API Types

- `Parser<'input>` — Parse GraphQL schemas or queries
- `SyntaxTree<T>` — Result of parsing with `errors()` and `document()` methods
- `Error` — Error type with `data()`, `index()`, `message()`
- `cst::Document` — Root document node

### CST Structure (Rowan-based)

- `Definition` enum: All definitions (DirectiveDefinition, ObjectTypeDefinition, ScalarTypeDefinition, etc.)
- `Type` enum: NamedType, ListType, NonNullType
- `Value` enum: IntValue, StringValue, BooleanValue, NullValue, etc.
- `Directive`, `FieldDefinition`, `InputValueDefinition` — Full type system support

## Error Recovery

- `parser.parse()` **never panics** on errors
- Returns `SyntaxTree` with **partial tree + errors** via `cst.errors()`
- Designed for error resilience (editor-like UX)
- Check `syntax_tree.errors().is_empty()` for clean parse

## Why apollo-parser > apollo-compiler

1. **Pure parsing** — apollo-compiler adds validation layer we don't need
2. **Custom directives** — apollo-compiler's validation is for spec compliance; we want raw CST for our custom directive schemas
3. **Lighter weight** — ~200KB vs ~330KB
4. **CST flexibility** — Rowan-based CST enables powerful tree transformations

## Why NOT graphql-parser

- Stale maintenance (last commit 2025-01, fork archived 2026-01)
- No error recovery (returns `Result`, fails fast on any error)
- Less rich API surface than apollo-parser

## Schema-Only Parsing

apollo-parser works perfectly for schema-only parsing:
- ✅ All type system definitions: Object, Interface, Union, Scalar, Enum, Input Object, Schema
- ✅ Custom directives preserved in CST nodes
- ✅ Descriptions supported (`Description` node type)
- ⚠️ Must walk CST manually to extract type info (no pre-built type registry)
- ⚠️ No semantic validation (we build that ourselves)

## References

- apollo-parser on crates.io: https://crates.io/crates/apollo-parser
- apollo-parser on docs.rs: https://docs.rs/apollo-parser/latest/apollo_parser/
- GitHub repo: https://github.com/apollographql/apollo-rs
- apollo-compiler on docs.rs: https://docs.rs/apollo-compiler/latest/apollo_compiler/
