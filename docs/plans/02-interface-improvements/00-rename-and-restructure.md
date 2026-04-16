# Rename and Restructure Plan

**For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform the existing gqlsdl-* crates into a unified graphql-ish-schema-validator library ecosystem with clear public API boundaries.

**Architecture:** Single top-level crate that re-exports sub-crates, each focused on a single responsibility (IR, parser, validator, registry, CLI).

**Tech Stack:** Rust, Cargo workspaces, serde, tracing, thiserror

---

## Context and Rationale

**Current state:**
- Multiple `gqlsdl-*` crates with unclear boundaries
- No single entry point for public API
- Naming doesn't reflect the project's mission
- Fragmented feature flags across crates

**Target state:**
- Unified `graphql-ish-schema-validator` brand across all crates
- Clear top-level public API for library users
- Sub-crates named with explicit purpose: `graphql-ish-schema-validator-<subsystem>`
- Default features enable YAML and CLI; optional features for advanced use cases

**References:**
- [01-initial-attempt/00-overview.md](../01-initial-attempt/00-overview.md) - Architecture overview
- [01-initial-attempt/07-cli-design.md](../01-initial-attempt/07-cli-design.md) - CLI naming conventions

---

## Task 1: Create Workspace Root Manifest

**Files:**
- Modify: `Cargo.toml`

**Step 1: Update workspace package name**

Create/update the root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/graphql-ish-schema-validator",
    "crates/graphql-ish-schema-validator-ir",
    "crates/graphql-ish-schema-validator-parser",
    "crates/graphql-ish-schema-validator-validator",
    "crates/graphql-ish-schema-validator-registry",
    "crates/graphql-ish-schema-validator-cli",
]
```

- [ ] **Step 2: Add workspace-level metadata**

```toml
[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Your Name <you@example.com>"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/your-org/graphql-ish-schema-validator"
description = "GraphQL-inspired schema validator for YAML/JSON documents"
keywords = ["graphql", "schema", "validation", "yaml", "json"]
categories = ["parser-implementations", "data-structures", "development-tools"]
rust-version = "1.70"

[workspace.dependencies]
# Common dependencies with version pinning
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 3: Test workspace setup**

Run: `cargo check`
Expected: Workspace configured, no crate compilation errors

- [ ] **Step 4: Commit workspace manifest**

```bash
git add Cargo.toml
git commit -m "feat: configure workspace for graphql-ish-schema-validator ecosystem"
```

---

## Task 2: Rename and Restructure gqlsdl-ir → graphql-ish-schema-validator-ir

**Files:**
- Create: `crates/graphql-ish-schema-validator-ir/Cargo.toml`
- Create: `crates/graphql-ish-schema-validator-ir/src/lib.rs`
- Move: Existing IR implementation files
- Delete: `crates/gqlsdl-ir/` (after migration complete)

**Step 1: Create new IR crate manifest**

```toml
[package]
name = "graphql-ish-schema-validator-ir"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Internal representation for GraphQL-ish schema validator"
keywords.workspace = true
categories.workspace = true
rust-version.workspace = true

[dependencies]
serde = { workspace = true, features = ["derive", "rc"] }
serde_json = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
insta = "1.34"
```

- [ ] **Step 2: Create IR library facade**

Create `crates/graphql-ish-schema-validator-ir/src/lib.rs`:

```rust
//! Internal Representation (IR) for GraphQL-ish schema validator.
//!
//! This crate defines the JTD-like IR that schemas compile to.
//! See the parent crate documentation for the complete architecture.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod forms;

pub use error::{IrError, IrResult};
pub use forms::*;
```

- [ ] **Step 3: Create error module**

Create `crates/graphql-ish-schema-validator-ir/src/error.rs`:

```rust
//! IR-specific errors

use thiserror::Error;

/// Errors that can occur when working with the IR
#[derive(Debug, Error)]
pub enum IrError {
    #[error("invalid type reference: {0}")]
    InvalidTypeReference(String),

    #[error("duplicate type name: {0}")]
    DuplicateTypeName(String),

    #[error("circular type reference detected")]
    CircularReference,

    #[error("schema validation failed: {0}")]
    ValidationFailed(String),
}

pub type IrResult<T> = Result<T, IrError>;
```

- [ ] **Step 4: Move existing IR implementation**

Copy/migrate existing IR enum definitions to `crates/graphql-ish-schema-validator-ir/src/forms.rs`:

```rust
//! JTD-like IR form definitions
//!
//! The IR consists of eight mutually-exclusive forms based on RFC 8927 (JTD),
//! plus pragmatic extensions needed for GraphQL-ish validation.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// The root compiled schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompiledSchema {
    /// Schema identifier (optional)
    pub schema_id: Option<String>,

    /// Schema version (optional)
    pub schema_version: Option<String>,

    /// Map of type definitions
    pub definitions: HashMap<String, SchemaForm>,
}

/// A schema form (mutually exclusive variants)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SchemaForm {
    /// Empty form (matches anything, usually ref only)
    Empty,

    /// Reference to another type
    Ref(RefForm),

    /// Type form (scalar type)
    Type(TypeForm),

    /// Enum form (fixed set of string values)
    Enum(EnumForm),

    /// Elements form (array validation)
    Elements(ElementsForm),

    /// Properties form (object validation)
    Properties(PropertiesForm),

    /// Values form (map validation)
    Values(ValuesForm),

    /// Discriminator form (tagged union)
    Discriminator(DiscriminatorForm),

    /// Extension: OneOf form (shape-based union)
    OneOf(OneOfForm),

    /// Extension: Any form (wildcard)
    Any,
}

/// Reference to another type by name
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefForm {
    #[serde(rename = "$ref")]
    pub type_name: String,
}

/// Type form for scalar values
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeForm {
    #[serde(rename = "type")]
    pub type_value: ScalarType,
}

/// Scalar types supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScalarType {
    String,
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
    Bool,
    Timestamp,
}

/// Enum form (fixed set of allowed string values)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnumForm {
    #[serde(rename = "enum")]
    pub values: Vec<String>,

    /// Optional regex constraint (GraphQL @pattern directive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<EnumMetadata>,
}

/// Enum metadata for constraints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnumMetadata {
    /// Pattern constraint from @pattern directive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

/// Elements form (array validation)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementsForm {
    #[serde(rename = "elements")]
    pub schema: Box<SchemaForm>,

    /// Minimum array length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_elements: Option<usize>,

    /// Maximum array length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_elements: Option<usize>,
}

/// Properties form (object validation)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropertiesForm {
    #[serde(rename = "properties")]
    pub required: BTreeMap<String, SchemaForm>,

    /// Optional properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<BTreeMap<String, SchemaForm>>,

    /// Additional properties behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<AdditionalProperties>,
}

/// Additional properties behavior for objects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AdditionalProperties {
    /// No additional properties allowed (closed object)
    #[serde(rename = "false")]
    Disallowed,

    /// Additional properties must match a schema
    #[serde(rename = "true")]
    Allowed(SchemaForm),

    /// All additional properties are allowed (open object)
    #[serde(rename = "any")]
    AnyAllowed,
}

/// Values form (map validation)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValuesForm {
    #[serde(rename = "values")]
    pub schema: Box<SchemaForm>,
}

/// Discriminator form (tagged union)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscriminatorForm {
    #[serde(rename = "discriminator")]
    pub tag: String,

    #[serde(rename = "mapping")]
    pub mapping: BTreeMap<String, SchemaForm>,
}

/// OneOf form (shape-based union, pragmatic extension)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OneOfForm {
    #[serde(rename = "oneOf")]
    pub variants: Vec<SchemaForm>,

    /// Optional hint message for ambiguous cases
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Default value metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum DefaultValue {
    String(String),
    Number(serde_json::Number),
    Bool(bool),
    Null,
    Array(Vec<DefaultValue>),
    Object(BTreeMap<String, DefaultValue>),
}
```

- [ ] **Step 5: Test IR crate compiles**

Run: `cargo check -p graphql-ish-schema-validator-ir`
Expected: Crate compiles without errors

- [ ] **Step 6: Add IR tests**

Create `crates/graphql-ish-schema-validator-ir/tests/form_serialization.rs`:

```rust
use graphql_ish_schema_validator_ir::forms::*;

#[test]
fn test_ref_form_serialization() {
    let form = SchemaForm::Ref(RefForm {
        type_name: "User".to_string(),
    });

    let json = serde_json::to_string(&form).unwrap();
    assert_eq!(json, r#"{"$ref":"User"}"#);
}

#[test]
fn test_properties_form_serialization() {
    let mut required = BTreeMap::new();
    required.insert("name".to_string(), SchemaForm::Type(TypeForm {
        type_value: ScalarType::String,
    }));

    let form = SchemaForm::Properties(PropertiesForm {
        required,
        optional: None,
        additional_properties: Some(AdditionalProperties::Disallowed),
    });

    let json = serde_json::to_string(&form).unwrap();
    assert!(json.contains(r#""properties""#));
    assert!(json.contains(r#""name""#));
}

#[test]
fn test_discriminator_form_roundtrip() {
    let mut mapping = BTreeMap::new();
    mapping.insert("agent".to_string(), SchemaForm::Ref(RefForm {
        type_name: "AgentStep".to_string(),
    }));

    let form = SchemaForm::Discriminator(DiscriminatorForm {
        tag: "step_type".to_string(),
        mapping,
    });

    let json = serde_json::to_string(&form).unwrap();
    let deserialized: SchemaForm = serde_json::from_str(&json).unwrap();
    assert_eq!(form, deserialized);
}
```

- [ ] **Step 7: Run IR tests**

Run: `cargo test -p graphql-ish-schema-validator-ir`
Expected: All tests pass

- [ ] **Step 8: Commit IR crate**

```bash
git add crates/graphql-ish-schema-validator-ir/
git commit -m "feat: add graphql-ish-schema-validator-ir crate with JTD-like forms"
```

---

## Task 3: Rename and Restructure gqlsdl-parser → graphql-ish-schema-validator-parser

**Files:**
- Create: `crates/graphql-ish-schema-validator-parser/Cargo.toml`
- Create: `crates/graphql-ish-schema-validator-parser/src/lib.rs`
- Move: Existing parser implementation files
- Delete: `crates/gqlsdl-parser/` (after migration complete)

**Step 1: Create parser crate manifest**

```toml
[package]
name = "graphql-ish-schema-validator-parser"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "GraphQL-ish SDL parser and AST builder"
keywords.workspace = true
categories.workspace = true
rust-version.workspace = true

[dependencies]
apollo-parser = "0.7"
graphql-ish-schema-validator-ir = { path = "../graphql-ish-schema-validator-ir" }
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
insta = "1.34"
```

- [ ] **Step 2: Create parser library facade**

Create `crates/graphql-ish-schema-validator-parser/src/lib.rs`:

```rust
//! GraphQL-ish SDL parser and AST builder
//!
//! This crate parses GraphQL SDL files and builds an AST for the
//! GraphQL-ish schema subset (scalars, enums, inputs, unions, directives).

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod ast;
pub mod error;
pub mod parser;

pub use ast::*;
pub use error::{ParseError, ParseResult};
pub use parser::Parser;
```

- [ ] **Step 3: Create error module**

Create `crates/graphql-ish-schema-validator-parser/src/error.rs`:

```rust
//! Parser errors

use thiserror::Error;
use apollo_parser::ParserError as ApolloParserError;

/// Errors that can occur during parsing
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("syntax error at {location}: {message}")]
    SyntaxError {
        location: Location,
        message: String,
    },

    #[error("invalid directive: {0}")]
    InvalidDirective(String),

    #[error("unsupported type definition: {0}")]
    UnsupportedTypeDefinition(String),

    #[error("duplicate type definition: {0}")]
    DuplicateType(String),

    #[error("invalid directive placement: {directive} on {type_name}")]
    InvalidDirectivePlacement {
        directive: String,
        type_name: String,
    },

    #[error("missing required directive argument: {0}")]
    MissingDirectiveArgument(String),

    #[error("circular reference detected: {0}")]
    CircularReference(String),

    #[error("invalid type reference: {0}")]
    InvalidTypeReference(String),

    #[error("invalid enum value: {0}")]
    InvalidEnumValue(String),

    #[error("invalid regex pattern: {0}")]
    InvalidRegex(String),

    #[error("schema validation error: {0}")]
    SchemaError(String),
}

/// Source location for errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl From<ApolloParserError> for ParseError {
    fn from(err: ApolloParserError) -> Self {
        ParseError::SyntaxError {
            location: Location {
                line: err.location().line() as usize,
                column: err.location().column() as usize,
            },
            message: err.message().to_string(),
        }
    }
}

pub type ParseResult<T> = Result<T, ParseError>;
```

- [ ] **Step 4: Create AST module**

Create `crates/graphql-ish-schema-validator-parser/src/ast.rs`:

```rust
//! Abstract Syntax Tree for GraphQL-ish SDL subset
//!
//! Represents the supported subset of GraphQL SDL:
//! - Scalar definitions with directives
//! - Enum definitions with values and directives
//! - Input object definitions with fields and directives
//! - Union definitions for input unions
//! - Directive definitions and applications

use std::collections::{HashMap, HashSet};
use std::fmt;

/// The root parsed document
#[derive(Debug, Clone, Default)]
pub struct Document {
    /// Scalar definitions
    pub scalars: Vec<ScalarDefinition>,

    /// Enum definitions
    pub enums: Vec<EnumDefinition>,

    /// Input object definitions
    pub inputs: Vec<InputDefinition>,

    /// Union definitions
    pub unions: Vec<UnionDefinition>,

    /// Directive definitions
    pub directives: Vec<DirectiveDefinition>,

    /// Document metadata
    pub metadata: DocumentMetadata,
}

/// Document-level metadata
#[derive(Debug, Clone, Default)]
pub struct DocumentMetadata {
    /// Schema identifier (from @schema_id directive)
    pub schema_id: Option<String>,

    /// Schema version (from @version directive)
    pub schema_version: Option<String>,
}

/// Scalar type definition
#[derive(Debug, Clone)]
pub struct ScalarDefinition {
    pub name: String,
    pub description: Option<String>,
    pub directives: Vec<Directive>,
}

/// Enum type definition
#[derive(Debug, Clone)]
pub struct EnumDefinition {
    pub name: String,
    pub description: Option<String>,
    pub values: Vec<EnumValue>,
    pub directives: Vec<Directive>,
}

/// Enum value definition
#[derive(Debug, Clone)]
pub struct EnumValue {
    pub name: String,
    pub description: Option<String>,
    pub directives: Vec<Directive>,
}

/// Input object type definition
#[derive(Debug, Clone)]
pub struct InputDefinition {
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<InputField>,
    pub directives: Vec<Directive>,
}

/// Input field definition
#[derive(Debug, Clone)]
pub struct InputField {
    pub name: String,
    pub description: Option<String>,
    pub r#type: Type,
    pub default_value: Option<Value>,
    pub directives: Vec<Directive>,
}

/// Union type definition
#[derive(Debug, Clone)]
pub struct UnionDefinition {
    pub name: String,
    pub description: Option<String>,
    pub members: Vec<String>,
    pub directives: Vec<Directive>,
}

/// Type reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Named type reference
    Named(String),

    /// Non-null wrapper
    NonNull(Box<Type>),

    /// List wrapper
    List(Box<Type>),
}

impl Type {
    /// Get the base named type
    pub fn base_type(&self) -> &str {
        match self {
            Type::Named(name) => name,
            Type::NonNull(inner) | Type::List(inner) => inner.base_type(),
        }
    }

    /// Check if this type is non-null
    pub fn is_non_null(&self) -> bool {
        matches!(self, Type::NonNull(_))
    }

    /// Check if this type is a list
    pub fn is_list(&self) -> bool {
        matches!(self, Type::List(_))
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Named(name) => write!(f, "{name}"),
            Type::NonNull(inner) => write!(f, "{inner}!"),
            Type::List(inner) => write!(f, "[{inner}]"),
        }
    }
}

/// Directive application
#[derive(Debug, Clone)]
pub struct Directive {
    pub name: String,
    pub arguments: Vec<Argument>,
}

/// Directive argument
#[derive(Debug, Clone)]
pub struct Argument {
    pub name: String,
    pub value: Value,
}

/// Literal value
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    List(Vec<Value>),
    Object(HashMap<String, Value>),
    Enum(String),
}

/// Directive definition
#[derive(Debug, Clone)]
pub struct DirectiveDefinition {
    pub name: String,
    pub description: Option<String>,
    pub locations: Vec<DirectiveLocation>,
    pub arguments: Vec<InputField>,
}

/// Directive locations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DirectiveLocation {
    Scalar,
    Enum,
    EnumValue,
    InputObject,
    InputField,
    Union,
}
```

- [ ] **Step 5: Create parser module**

Create `crates/graphql-ish-schema-validator-parser/src/parser.rs`:

```rust
//! SDL parser implementation
//!
//! Uses apollo-parser to parse GraphQL SDL and builds the AST.

use apollo_parser::Parser;
use tracing::{debug, trace, warn};

use super::ast::*;
use super::error::{ParseError, ParseResult, Location};
use super::error::ParseError::*;

/// Parses GraphQL SDL source into an AST
pub struct Parser {
    source: String,
    apollo_parser: Parser,
}

impl Parser {
    /// Create a new parser for the given source
    pub fn new(source: impl Into<String>) -> Self {
        let source = source.into();
        let apollo_parser = Parser::new(&source);
        Self {
            source,
            apollo_parser,
        }
    }

    /// Parse the SDL and return the AST
    pub fn parse(mut self) -> ParseResult<Document> {
        debug!("Starting SDL parse");
        let tree = self.apollo_parser.parse();
        let errors = tree.errors();

        if !errors.is_empty() {
            let first_error = errors.first().unwrap();
            warn!("Parse errors detected: {}", errors.len());
            return Err(SyntaxError {
                location: Location {
                    line: first_error.location().line() as usize,
                    column: first_error.location().column() as usize,
                },
                message: first_error.message().to_string(),
            });
        }

        let mut document = Document::default();
        let mut type_names = HashSet::new();

        for definition in tree.definitions() {
            match definition {
                apollo_parser::ast::Definition::ScalarTypeDefinition(scalar) => {
                    let def = self.parse_scalar(&scalar, &mut type_names)?;
                    document.scalars.push(def);
                }
                apollo_parser::ast::Definition::EnumTypeDefinition(enum_def) => {
                    let def = self.parse_enum(&enum_def, &mut type_names)?;
                    document.enums.push(def);
                }
                apollo_parser::ast::Definition::InputObjectTypeDefinition(input) => {
                    let def = self.parse_input(&input, &mut type_names)?;
                    document.inputs.push(def);
                }
                apollo_parser::ast::Definition::UnionTypeDefinition(union) => {
                    let def = self.parse_union(&union, &mut type_names)?;
                    document.unions.push(def);
                }
                _ => {
                    trace!("Skipping unsupported definition: {:?}", definition);
                }
            }
        }

        // Extract document metadata from top-level directives
        self.extract_metadata(&mut document)?;

        debug!("Parse complete: {} scalars, {} enums, {} inputs, {} unions",
            document.scalars.len(),
            document.enums.len(),
            document.inputs.len(),
            document.unions.len()
        );

        Ok(document)
    }

    fn parse_scalar(
        &self,
        scalar: &apollo_parser::ast::ScalarTypeDefinition,
        type_names: &mut HashSet<String>,
    ) -> ParseResult<ScalarDefinition> {
        let name = scalar
            .name()
            .expect("scalar name required")
            .text()
            .to_string();

        if !type_names.insert(name.clone()) {
            return Err(DuplicateType(name));
        }

        trace!("Parsing scalar: {}", name);

        let description = scalar.description().map(|d| d.to_string());
        let directives = self.parse_directives(scalar.directives())?;

        Ok(ScalarDefinition {
            name,
            description,
            directives,
        })
    }

    fn parse_enum(
        &self,
        enum_def: &apollo_parser::ast::EnumTypeDefinition,
        type_names: &mut HashSet<String>,
    ) -> ParseResult<EnumDefinition> {
        let name = enum_def
            .name()
            .expect("enum name required")
            .text()
            .to_string();

        if !type_names.insert(name.clone()) {
            return Err(DuplicateType(name));
        }

        trace!("Parsing enum: {}", name);

        let description = enum_def.description().map(|d| d.to_string());

        let mut values = Vec::new();
        for value_def in enum_def.enum_values_definition() {
            let enum_value_def = value_def.enum_value_definition().expect("enum value");
            let value_name = enum_value_def.enum_value().expect("enum value name").text().to_string();

            let value = EnumValue {
                name: value_name,
                description: enum_value_def.description().map(|d| d.to_string()),
                directives: self.parse_directives(enum_value_def.directives())?,
            };
            values.push(value);
        }

        let directives = self.parse_directives(enum_def.directives())?;

        Ok(EnumDefinition {
            name,
            description,
            values,
            directives,
        })
    }

    fn parse_input(
        &self,
        input: &apollo_parser::ast::InputObjectTypeDefinition,
        type_names: &mut HashSet<String>,
    ) -> ParseResult<InputDefinition> {
        let name = input
            .name()
            .expect("input name required")
            .text()
            .to_string();

        if !type_names.insert(name.clone()) {
            return Err(DuplicateType(name));
        }

        trace!("Parsing input: {}", name);

        let description = input.description().map(|d| d.to_string());

        let mut fields = Vec::new();
        for input_fields_def in input.input_fields_definition() {
            for field_def in input_fields_def.input_value_definitions() {
                let field = self.parse_input_field(field_def)?;
                fields.push(field);
            }
        }

        let directives = self.parse_directives(input.directives())?;

        Ok(InputDefinition {
            name,
            description,
            fields,
            directives,
        })
    }

    fn parse_input_field(
        &self,
        field: &apollo_parser::ast::InputValueDefinition,
    ) -> ParseResult<InputField> {
        let name = field
            .name()
            .expect("field name required")
            .text()
            .to_string();

        let description = field.description().map(|d| d.to_string());

        let r#type = self.parse_type(field.ty().expect("field type required"))?;

        let default_value = field.default_value().map(|v| self.parse_value(v));

        let directives = self.parse_directives(field.directives())?;

        Ok(InputField {
            name,
            description,
            r#type,
            default_value,
            directives,
        })
    }

    fn parse_union(
        &self,
        union_def: &apollo_parser::ast::UnionTypeDefinition,
        type_names: &mut HashSet<String>,
    ) -> ParseResult<UnionDefinition> {
        let name = union_def
            .name()
            .expect("union name required")
            .text()
            .to_string();

        if !type_names.insert(name.clone()) {
            return Err(DuplicateType(name));
        }

        trace!("Parsing union: {}", name);

        let description = union_def.description().map(|d| d.to_string());

        let mut members = Vec::new();
        if let Some(union_members) = union_def.union_member_types() {
            for member in union_members.named_types() {
                let member_name = member.name().expect("member name").text().to_string();
                members.push(member_name);
            }
        }

        let directives = self.parse_directives(union_def.directives())?;

        Ok(UnionDefinition {
            name,
            description,
            members,
            directives,
        })
    }

    fn parse_type(&self, ty: apollo_parser::ast::Type) -> ParseResult<Type> {
        use apollo_parser::ast::Type as ApolloType;

        match ty {
            ApolloType::Named(name) => {
                let type_name = name.name().expect("type name").text().to_string();
                Ok(Type::Named(type_name))
            }
            ApolloType::NonNull(ty) => {
                let inner = self.parse_type(*ty)?;
                Ok(Type::NonNull(Box::new(inner)))
            }
            ApolloType::List(ty) => {
                let inner = self.parse_type(*ty)?;
                Ok(Type::List(Box::new(inner)))
            }
        }
    }

    fn parse_value(&self, value: apollo_parser::ast::Value) -> Value {
        use apollo_parser::ast::Value as ApolloValue;

        match value {
            ApolloValue::StringValue(v) => Value::String(v.into()),
            ApolloValue::IntValue(v) => Value::Int(v.into()),
            ApolloValue::FloatValue(v) => Value::Float(v.into()),
            ApolloValue::BooleanValue(v) => Value::Bool(v.into()),
            ApolloValue::NullValue(_) => Value::Null,
            ApolloValue::EnumValue(v) => Value::Enum(v.into()),
            ApolloValue::ListValue(v) => {
                Value::List(v.values().map(|v| self.parse_value(v)).collect())
            }
            ApolloValue::ObjectValue(v) => {
                Value::Object(v.fields()
                    .map(|f| {
                        let name = f.name().expect("object key").text().to_string();
                        let value = self.parse_value(f.value().expect("object value"));
                        (name, value)
                    })
                    .collect())
            }
        }
    }

    fn parse_directives(
        &self,
        directives: apollo_parser::ast::DirectiveList,
    ) -> ParseResult<Vec<Directive>> {
        let mut result = Vec::new();

        for dir in directives.directives() {
            let name = dir.name().expect("directive name").text().to_string();

            let mut arguments = Vec::new();
            if let Some(args) = dir.arguments() {
                for arg in args.arguments() {
                    let arg_name = arg.name().expect("arg name").text().to_string();
                    let arg_value = self.parse_value(arg.value().expect("arg value"));
                    arguments.push(Argument {
                        name: arg_name,
                        value: arg_value,
                    });
                }
            }

            result.push(Directive {
                name,
                arguments,
            });
        }

        Ok(result)
    }

    fn extract_metadata(&self, document: &mut Document) -> ParseResult<()> {
        // Look for top-level directives in the AST
        // This would require traversing parsed definitions
        // For now, we'll skip this and return Ok
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_scalar() {
        let source = r#"
            scalar String
        "#;

        let parser = Parser::new(source);
        let document = parser.parse().unwrap();

        assert_eq!(document.scalars.len(), 1);
        assert_eq!(document.scalars[0].name, "String");
    }

    #[test]
    fn test_parse_simple_enum() {
        let source = r#"
            enum BackoffStrategy {
                exponential
                linear
                fixed
            }
        "#;

        let parser = Parser::new(source);
        let document = parser.parse().unwrap();

        assert_eq!(document.enums.len(), 1);
        assert_eq!(document.enums[0].name, "BackoffStrategy");
        assert_eq!(document.enums[0].values.len(), 3);
    }

    #[test]
    fn test_parse_simple_input() {
        let source = r#"
            input Provider @closed {
                name: String!
                config: ProviderConfig
            }
        "#;

        let parser = Parser::new(source);
        let document = parser.parse().unwrap();

        assert_eq!(document.inputs.len(), 1);
        assert_eq!(document.inputs[0].name, "Provider");
        assert_eq!(document.inputs[0].fields.len(), 2);
    }

    #[test]
    fn test_parse_simple_union() {
        let source = r#"
            union Step = AgentStep | ToolStep | ControlFlowStep
        "#;

        let parser = Parser::new(source);
        let document = parser.parse().unwrap();

        assert_eq!(document.unions.len(), 1);
        assert_eq!(document.unions[0].name, "Step");
        assert_eq!(document.unions[0].members.len(), 3);
    }
}
```

- [ ] **Step 6: Test parser crate compiles**

Run: `cargo check -p graphql-ish-schema-validator-parser`
Expected: Crate compiles without errors

- [ ] **Step 7: Run parser tests**

Run: `cargo test -p graphql-ish-schema-validator-parser`
Expected: All tests pass

- [ ] **Step 8: Commit parser crate**

```bash
git add crates/graphql-ish-schema-validator-parser/
git commit -m "feat: add graphql-ish-schema-validator-parser crate with SDL parsing"
```

---

## Task 4: Rename and Restructure Validator Crate

**Files:**
- Create: `crates/graphql-ish-schema-validator-validator/Cargo.toml`
- Create: `crates/graphql-ish-schema-validator-validator/src/lib.rs`
- Move: Existing validator implementation files
- Delete: `crates/gqlsdl-validator/` (after migration complete)

**Step 1: Create validator crate manifest**

```toml
[package]
name = "graphql-ish-schema-validator-validator"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Validation engine for GraphQL-ish schema validator"
keywords.workspace = true
categories.workspace = true
rust-version.workspace = true

[dependencies]
graphql-ish-schema-validator-ir = { path = "../graphql-ish-schema-validator-ir" }
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml_ng = "0.9"
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
insta = "1.34"
```

- [ ] **Step 2: Create validator library facade**

Create `crates/graphql-ish-schema-validator-validator/src/lib.rs`:

```rust
//! Validation engine for GraphQL-ish schema validator
//!
//! This crate provides the runtime validation logic that validates
//! YAML/JSON documents against compiled IR schemas.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod options;
pub mod result;
pub mod validator;

pub use error::{ValidationError, ValidationErrorCode};
pub use options::ValidationOptions;
pub use result::ValidationResult;
pub use validator::Validator;
```

- [ ] **Step 3: Create validation options module**

Create `crates/graphql-ish-schema-validator-validator/src/options.rs`:

```rust
//! Validation options and configuration

use serde::{Deserialize, Serialize};

/// Validation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ValidationMode {
    /// Strict mode: reject unknown keys, duplicate keys, no type coercion
    #[default]
    Strict,

    /// Open mode: allow unknown keys, limited type coercion
    Open,
}

/// Log level for validation output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LogLevel {
    /// Trace: detailed CST walking and IR compilation steps
    Trace,

    /// Debug: IR compilation and validation steps
    Debug,

    /// Info: validation summary and key events (default)
    #[default]
    Info,

    /// Warn: soft failures only
    Warn,

    /// Error: hard failures only
    Error,

    /// Silent: no logging output
    Silent,
}

/// Log output destination
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LogOutput {
    /// Log to stderr (default)
    #[default]
    Stderr,

    /// Log to stdout
    Stdout,

    /// Log to a file
    File(String),

    /// No logging
    Silent,
}

/// Schema format for input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SchemaFormat {
    /// Auto-detect format from file extension or content
    #[default]
    AutoDetect,

    /// GraphQL SDL format
    GraphQL,

    /// YAML format
    Yaml,

    /// JSON format
    Json,
}

/// Validation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationOptions {
    /// Validation mode (strict/open)
    #[serde(default)]
    pub mode: ValidationMode,

    /// Maximum nesting depth for validation
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    /// Root schema name (for error reporting)
    #[serde(default)]
    pub root_schema_name: Option<String>,

    /// Log level
    #[serde(default)]
    pub log_level: LogLevel,

    /// Log output destination
    #[serde(default)]
    pub log_output: LogOutput,

    /// Schema format
    #[serde(default)]
    pub schema_format: SchemaFormat,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            mode: ValidationMode::default(),
            max_depth: default_max_depth(),
            root_schema_name: None,
            log_level: LogLevel::default(),
            log_output: LogOutput::default(),
            schema_format: SchemaFormat::default(),
        }
    }
}

fn default_max_depth() -> usize {
    100
}
```

- [ ] **Step 4: Create validation result module**

Create `crates/graphql-ish-schema-validator-validator/src/result.rs`:

```rust
//! Validation result types

use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::collections::VecDeque;

use crate::error::{ValidationError, ValidationErrorCode};

/// Complete validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the document is valid
    pub valid: bool,

    /// Validation errors
    pub errors: Vec<ValidationError>,

    /// Validation warnings
    pub warnings: Vec<ValidationError>,

    /// Time taken to validate
    pub duration: Duration,

    /// Schema name used for validation
    pub schema_name: Option<String>,

    /// Document type (YAML or JSON)
    pub document_type: DocumentType,
}

/// Document type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentType {
    /// YAML document
    Yaml,

    /// JSON document
}

impl ValidationResult {
    /// Create a new successful validation result
    pub fn success(schema_name: Option<String>, document_type: DocumentType, duration: Duration) -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            duration,
            schema_name,
            document_type,
        }
    }

    /// Create a new failed validation result
    pub fn failure(
        errors: Vec<ValidationError>,
        schema_name: Option<String>,
        document_type: DocumentType,
        duration: Duration,
    ) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
            duration,
            schema_name,
            document_type,
        }
    }

    /// Add a warning to the result
    pub fn add_warning(&mut self, warning: ValidationError) {
        self.warnings.push(warning);
    }

    /// Check if the result has any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if the result has any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// JSON Pointer to a location in a document
pub type JsonPointer = String;

/// Path in the instance being validated
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstancePath {
    /// The JSON pointer string
    pub pointer: JsonPointer,

    /// Human-readable path segments
    pub segments: VecDeque<String>,
}

impl InstancePath {
    /// Create a new empty instance path
    pub fn new() -> Self {
        Self {
            pointer: String::new(),
            segments: VecDeque::new(),
        }
    }

    /// Push a new segment to the path
    pub fn push(&mut self, segment: &str) {
        self.segments.push_back(segment.to_string());
        self.update_pointer();
    }

    /// Push an array index to the path
    pub fn push_index(&mut self, index: usize) {
        self.push(&index.to_string());
    }

    /// Pop the last segment from the path
    pub fn pop(&mut self) -> Option<String> {
        let segment = self.segments.pop_back()?;
        self.update_pointer();
        Some(segment)
    }

    /// Get the current pointer string
    pub fn as_str(&self) -> &str {
        &self.pointer
    }

    /// Update the pointer string from segments
    fn update_pointer(&mut self) {
        self.pointer = self.segments
            .iter()
            .map(|s| {
                if s.parse::<usize>().is_ok() {
                    format!("/{s}")
                } else {
                    s.replace("~", "~0")
                        .replace("/", "~1")
                }
            })
            .collect::<String>();
    }
}

impl Default for InstancePath {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 5: Create error module (stub)**

Create `crates/graphql-ish-schema-validator-validator/src/error.rs`:

```rust
//! Validation error types

use serde::{Deserialize, Serialize};
use std::fmt;

use super::result::JsonPointer;

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// JSON Pointer to location in the validated document
    pub instance_path: JsonPointer,

    /// JSON Pointer to location in the schema
    pub schema_path: JsonPointer,

    /// Error code
    pub code: ValidationErrorCode,

    /// Human-readable error message
    pub message: String,

    /// Optional hint for remediation
    pub hint: Option<String>,

    /// Error severity
    pub severity: ErrorSeverity,

    /// Source location in the document (line/column)
    pub source_location: Option<SourceLocation>,
}

/// Validation error code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationErrorCode {
    /// Type mismatch
    TypeMismatch,

    /// Required property missing
    RequiredPropertyMissing,

    /// Additional property not allowed
    AdditionalPropertyNotAllowed,

    /// Invalid enum value
    InvalidEnumValue,

    /// Pattern mismatch
    PatternMismatch,

    /// Minimum length violation
    MinimumLengthViolation,

    /// Maximum length violation
    MaximumLengthViolation,

    /// Minimum value violation
    MinimumValueViolation,

    /// Maximum value violation
    MaximumValueViolation,

    /// Circular reference
    CircularReference,

    /// Invalid type reference
    InvalidTypeReference,

    /// Schema validation error
    SchemaError,

    /// Parse error
    ParseError,

    /// Unknown error
    Unknown,
}

impl fmt::Display for ValidationErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationErrorCode::TypeMismatch => write!(f, "type_mismatch"),
            ValidationErrorCode::RequiredPropertyMissing => write!(f, "required_property_missing"),
            ValidationErrorCode::AdditionalPropertyNotAllowed => write!(f, "additional_property_not_allowed"),
            ValidationErrorCode::InvalidEnumValue => write!(f, "invalid_enum_value"),
            ValidationErrorCode::PatternMismatch => write!(f, "pattern_mismatch"),
            ValidationErrorCode::MinimumLengthViolation => write!(f, "minimum_length_violation"),
            ValidationErrorCode::MaximumLengthViolation => write!(f, "maximum_length_violation"),
            ValidationErrorCode::MinimumValueViolation => write!(f, "minimum_value_violation"),
            ValidationErrorCode::MaximumValueViolation => write!(f, "maximum_value_violation"),
            ValidationErrorCode::CircularReference => write!(f, "circular_reference"),
            ValidationErrorCode::InvalidTypeReference => write!(f, "invalid_type_reference"),
            ValidationErrorCode::SchemaError => write!(f, "schema_error"),
            ValidationErrorCode::ParseError => write!(f, "parse_error"),
            ValidationErrorCode::Unknown => write!(f, "unknown"),
        }
    }
}

/// Error severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Error: validation failed
    Error,

    /// Warning: soft failure, document may still be valid
    Warning,
}

/// Source location in a document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// Line number (1-indexed)
    pub line: usize,

    /// Column number (1-indexed)
    pub column: usize,
}
```

- [ ] **Step 6: Create validator module stub**

Create `crates/graphql-ish-schema-validator-validator/src/validator.rs`:

```rust
//! Core validation engine

use graphql_ish_schema_validator_ir::CompiledSchema;
use serde_json::Value;
use std::time::Instant;

use super::options::ValidationOptions;
use super::result::{DocumentType, ValidationResult};

/// The validator
pub struct Validator {
    schema: CompiledSchema,
    options: ValidationOptions,
}

impl Validator {
    /// Create a new validator with the given schema and options
    pub fn new(schema: CompiledSchema, options: ValidationOptions) -> Self {
        Self {
            schema,
            options,
        }
    }

    /// Validate a JSON value against the schema
    pub fn validate_json(&self, json: &str) -> ValidationResult {
        let start = Instant::now();

        match serde_json::from_str::<Value>(json) {
            Ok(value) => {
                // TODO: Implement actual validation logic
                let duration = start.elapsed();
                ValidationResult::success(
                    self.schema.schema_id.clone(),
                    DocumentType::Json,
                    duration,
                )
            }
            Err(e) => {
                let duration = start.elapsed();
                let error = crate::error::ValidationError {
                    instance_path: String::new(),
                    schema_path: String::new(),
                    code: crate::error::ValidationErrorCode::ParseError,
                    message: format!("Failed to parse JSON: {}", e),
                    hint: Some("Check JSON syntax".to_string()),
                    severity: crate::error::ErrorSeverity::Error,
                    source_location: None,
                };
                ValidationResult::failure(
                    vec![error],
                    self.schema.schema_id.clone(),
                    DocumentType::Json,
                    duration,
                )
            }
        }
    }

    /// Validate a YAML value against the schema
    pub fn validate_yaml(&self, yaml: &str) -> ValidationResult {
        let start = Instant::now();

        match serde_yaml_ng::from_str::<Value>(yaml) {
            Ok(value) => {
                // TODO: Implement actual validation logic
                let duration = start.elapsed();
                ValidationResult::success(
                    self.schema.schema_id.clone(),
                    DocumentType::Yaml,
                    duration,
                )
            }
            Err(e) => {
                let duration = start.elapsed();
                let error = crate::error::ValidationError {
                    instance_path: String::new(),
                    schema_path: String::new(),
                    code: crate::error::ValidationErrorCode::ParseError,
                    message: format!("Failed to parse YAML: {}", e),
                    hint: Some("Check YAML syntax".to_string()),
                    severity: crate::error::ErrorSeverity::Error,
                    source_location: None,
                };
                ValidationResult::failure(
                    vec![error],
                    self.schema.schema_id.clone(),
                    DocumentType::Yaml,
                    duration,
                )
            }
        }
    }
}
```

- [ ] **Step 7: Test validator crate compiles**

Run: `cargo check -p graphql-ish-schema-validator-validator`
Expected: Crate compiles without errors

- [ ] **Step 8: Commit validator crate**

```bash
git add crates/graphql-ish-schema-validator-validator/
git commit -m "feat: add graphql-ish-schema-validator-validator crate with validation engine"
```

---

## Task 5: Create Top-Level Public API Crate

**Files:**
- Create: `crates/graphql-ish-schema-validator/Cargo.toml`
- Create: `crates/graphql-ish-schema-validator/src/lib.rs`

**Step 1: Create top-level crate manifest**

```toml
[package]
name = "graphql-ish-schema-validator"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "GraphQL-inspired schema validator for YAML/JSON documents"
keywords.workspace = true
categories.workspace = true
rust-version.workspace = true

[features]
default = ["yaml", "cli"]

yaml = ["graphql-ish-schema-validator-validator"]
json-schema-export = []
http-registry = []
cli = ["graphql-ish-schema-validator-cli"]

[dependencies]
graphql-ish-schema-validator-ir = { path = "../graphql-ish-schema-validator-ir" }
graphql-ish-schema-validator-parser = { path = "../graphql-ish-schema-validator-parser" }
graphql-ish-schema-validator-validator = { path = "../graphql-ish-schema-validator-validator", optional = true }
graphql-ish-schema-validator-registry = { path = "../graphql-ish-schema-validator-registry", optional = true }
graphql-ish-schema-validator-cli = { path = "../graphql-ish-schema-validator-cli", optional = true }

thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
insta = "1.34"
```

- [ ] **Step 2: Create public API facade**

Create `crates/graphql-ish-schema-validator/src/lib.rs`:

```rust
//! GraphQL-inspired schema validator for YAML/JSON documents
//!
//! This library provides a GraphQL-like SDL for authoring schemas,
//! compiles them to a JTD-like IR, and validates YAML/JSON documents
//! with excellent diagnostics and optional registry support.
//!
//! # Quick Start
//!
//! ```rust
//! use graphql_ish_schema_validator::{validate_yaml_from_schema, ValidationOptions};
//!
//! let schema = r#"
//!     input Person @closed {
//!         name: String!
//!         age: Int!
//!     }
//! "#;
//!
//! let yaml = r#"
//!     name: Alice
//!     age: 30
//! "#;
//!
//! let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());
//!
//! if result.valid {
//!     println!("Valid!");
//! } else {
//!     for error in result.errors {
//!         println!("Error at {}: {}", error.instance_path, error.message);
//!     }
//! }
//! ```
//!
//! # Architecture
//!
//! The system follows a multi-stage pipeline:
//!
//! 1. **Parse**: GraphQL SDL → AST (using `apollo-parser`)
//! 2. **Validate**: Semantic validation of the schema
//! 3. **Lower**: AST → JTD-like IR
//! 4. **Runtime**: Validate YAML/JSON against IR
//!
//! # Crate Organization
//!
//! - `graphql-ish-schema-validator`: Top-level public API (this crate)
//! - `graphql-ish-schema-validator-ir`: Internal representation
//! - `graphql-ish-schema-validator-parser`: SDL parser
//! - `graphql-ish-schema-validator-validator`: Validation engine
//! - `graphql-ish-schema-validator-registry`: Schema registry system
//! - `graphql-ish-schema-validator-cli`: Command-line tool

#![warn(missing_docs)]
#![warn(clippy::all)]

// Re-export the public API
pub use graphql_ish_schema_validator_validator::{
    ValidationError,
    ValidationErrorCode,
    ValidationOptions,
    ValidationResult,
};

// Re-export IR types for advanced usage
pub use graphql_ish_schema_validator_ir::{
    CompiledSchema,
    SchemaForm,
    ScalarType,
};

// Re-export parser for schema compilation
pub use graphql_ish_schema_validator_parser::{
    Parser,
    Document,
};

use std::time::Instant;

use graphql_ish_schema_validator_parser::ParseResult;
use graphql_ish_schema_validator_validator::{
    DocumentType, Validator,
};

/// Validate a YAML document against a GraphQL-ish schema
///
/// # Arguments
///
/// * `yaml` - The YAML document to validate (as a string)
/// * `schema` - The GraphQL-ish SDL schema (as a string)
/// * `options` - Validation options
///
/// # Returns
///
/// A `ValidationResult` containing validation outcome and any errors
///
/// # Example
///
/// ```rust
/// use graphql_ish_schema_validator::{validate_yaml_from_schema, ValidationOptions};
///
/// let schema = r#"
///     input Workflow @closed {
///         name: String!
///         steps: [Step!]!
///     }
///
///     union Step = AgentStep | ToolStep
///
///     input AgentStep @closed {
///         prompt: String!
///         model: String!
///     }
///
///     input ToolStep @closed {
///         tool: String!
///         input: Any
///     }
/// "#;
///
/// let yaml = r#"
///     name: My Workflow
///     steps:
///       - prompt: Generate code
///         model: gpt-4
/// "#;
///
/// let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());
/// assert!(result.valid);
/// ```
pub fn validate_yaml_from_schema(
    yaml: &str,
    schema: &str,
    options: ValidationOptions,
) -> ValidationResult {
    let start = Instant::now();

    // Parse the schema
    let document = match Parser::new(schema).parse() {
        Ok(doc) => doc,
        Err(e) => {
            let duration = start.elapsed();
            let error = ValidationError {
                instance_path: String::new(),
                schema_path: String::new(),
                code: ValidationErrorCode::ParseError,
                message: format!("Failed to parse schema: {}", e),
                hint: Some("Check GraphQL SDL syntax".to_string()),
                severity: crate::error::ErrorSeverity::Error,
                source_location: None,
            };
            return ValidationResult::failure(
                vec![error],
                None,
                DocumentType::Yaml,
                duration,
            );
        }
    };

    // TODO: Lower AST to IR (will be implemented in subsequent tasks)
    // For now, create a dummy compiled schema
    let compiled_schema = CompiledSchema {
        schema_id: None,
        schema_version: None,
        definitions: Default::default(),
    };

    // Create validator and validate
    let validator = Validator::new(compiled_schema, options);
    validator.validate_yaml(yaml)
}

/// Validate a JSON document against a GraphQL-ish schema
///
/// # Arguments
///
/// * `json` - The JSON document to validate (as a string)
/// * `schema` - The GraphQL-ish GraphQL-ish schema (as a string)
/// * `options` - Validation options
///
/// # Returns
///
/// A `ValidationResult` containing validation outcome and any errors
///
/// # Example
///
/// ```rust
/// use graphql_ish_schema_validator::{validate_json_from_schema, ValidationOptions};
///
/// let schema = r#"
///     input User @closed {
///         id: String!
///         email: String!
///     }
/// "#;
///
/// let json = r#"{
///     "id": "123",
///     "email": "user@example.com"
/// }"#;
///
/// let result = validate_json_from_schema(json, schema, ValidationOptions::default());
/// assert!(result.valid);
/// ```
pub fn validate_json_from_schema(
    json: &str,
    schema: &str,
    options: ValidationOptions,
) -> ValidationResult {
    let start = Instant::now();

    // Parse the schema
    let document = match Parser::new(schema).parse() {
        Ok(doc) => doc,
        Err(e) => {
            let duration = start.elapsed();
            let error = ValidationError {
                instance_path: String::new(),
                schema_path: String::new(),
                code: ValidationErrorCode::ParseError,
                message: format!("Failed to parse schema: {}", e),
                hint: Some("Check GraphQL SDL syntax".to_string()),
                severity: crate::error::ErrorSeverity::Error,
                source_location: None,
            };
            return ValidationResult::failure(
                vec![error],
                None,
                DocumentType::Json,
                duration,
            );
        }
    };

    // TODO: Lower AST to IR (will be implemented in subsequent tasks)
    let compiled_schema = CompiledSchema {
        schema_id: None,
        schema_version: None,
        definitions: Default::default(),
    };

    // Create validator and validate
    let validator = Validator::new(compiled_schema, options);
    validator.validate_json(json)
}

/// Parse a GraphQL-ish SDL schema into an AST
///
/// # Arguments
///
/// * `schema` - The GraphQL-ish SDL schema (as a string)
///
/// # Returns
///
/// A `ParseResult` containing the parsed `Document` AST
///
/// # Example
///
/// ```rust
/// use graphql_ish_schema_validator::parse_schema;
///
/// let schema = r#"
///     input Workflow @closed {
///         name: String!
///     }
/// "#;
///
/// let document = parse_schema(schema).unwrap();
/// assert_eq!(document.inputs.len(), 1);
/// assert_eq!(document.inputs[0].name, "Workflow");
/// ```
pub fn parse_schema(schema: &str) -> ParseResult<Document> {
    Parser::new(schema).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_yaml_valid() {
        let schema = r#"
            input Simple @closed {
                value: String!
            }
        "#;

        let yaml = r#"
            value: test
        "#;

        let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());
        // For now, we expect it to succeed (validation not yet implemented)
        assert!(result.valid);
    }

    #[test]
    fn test_validate_json_valid() {
        let schema = r#"
            input Simple @closed {
                value: String!
            }
        "#;

        let json = r#"{
            "value": "test"
        }"#;

        let result = validate_json_from_schema(json, schema, ValidationOptions::default());
        // For now, we expect it to succeed (validation not yet implemented)
        assert!(result.valid);
    }

    #[test]
    fn test_parse_schema_simple() {
        let schema = r#"
            input Test @closed {
                name: String!
                count: Int!
            }
        "#;

        let document = parse_schema(schema).unwrap();
        assert_eq!(document.inputs.len(), 1);
        assert_eq!(document.inputs[0].fields.len(), 2);
    }
}
```

- [ ] **Step 3: Test top-level crate compiles**

Run: `cargo check -p graphql-ish-schema-validator`
Expected: Crate compiles without errors

- [ ] **Step 4: Run top-level crate tests**

Run: `cargo test -p graphql-ish-schema-validator`
Expected: All tests pass

- [ ] **Step 5: Commit top-level crate**

```bash
git add crates/graphql-ish-schema-validator/
git commit -m "feat: add graphql-ish-schema-validator top-level public API"
```

---

## Task 6: Update CLI Crate

**Files:**
- Create: `crates/graphql-ish-schema-validator-cli/Cargo.toml`
- Create: `crates/graphql-ish-schema-validator-cli/src/main.rs`
- Move: Existing CLI implementation files
- Delete: `crates/gqlsdl-cli/` (after migration complete)

**Step 1: Create CLI crate manifest**

```toml
[package]
name = "graphql-ish-schema-validator-cli"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "CLI tool for GraphQL-ish schema validator"
keywords.workspace = true
categories.workspace = true
rust-version.workspace = true

[[bin]]
name = "graphql-ish-schema-validator"
path = "src/main.rs"

[[bin]]
name = "gqlsdl"
path = "src/main.rs"

[dependencies]
graphql-ish-schema-validator = { path = "../graphql-ish-schema-validator", features = ["yaml"] }
anyhow = "1.0"
clap = { version = "4.4", features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
```

- [ ] **Step 2: Create CLI main stub**

Create `crates/graphql-ish-schema-validator-cli/src/main.rs`:

```rust
//! GraphQL-ish Schema Validator CLI
//!
//! Binary names:
//! - `graphql-ish-schema-validator` (full name)
//! - `gqlsdl` (short alias)

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::Level;

/// GraphQL-inspired schema validator for YAML/JSON documents
#[derive(Parser, Debug)]
#[command(
    name = "graphql-ish-schema-validator",
    bin_name = "graphql-ish-schema-validator",
    author,
    version,
    about,
    long_about = "Validate YAML/JSON documents against GraphQL-inspired schemas with excellent diagnostics."
)]
struct Cli {
    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: Level,

    /// Suppress all output
    #[arg(short, long)]
    quiet: bool,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Available subcommands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Validate a YAML/JSON document
    Validate {
        /// Input file or directory
        input: String,

        /// Schema file or URI
        #[arg(short, long)]
        schema: String,

        /// Enable strict mode
        #[arg(long)]
        strict: bool,

        /// Enable open mode
        #[arg(long)]
        open: bool,

        /// Output format (text, json, github-actions)
        #[arg(short = 'F', long, default_value = "text")]
        format: String,
    },

    /// Compile an SDL schema to IR
    Compile {
        /// Schema file to compile
        schema: String,

        /// Output file
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Check a schema for internal consistency
    CheckSchema {
        /// Schema file to check
        schema: String,
    },

    /// Export a schema to JTD JSON
    ExportJtd {
        /// Schema file to export
        schema: String,

        /// Output file
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup tracing
    let log_level = if cli.quiet {
        Level::ERROR
    } else {
        cli.log_level
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_ansi(!cli.no_color)
        .init();

    match cli.command {
        Some(Commands::Validate { input, schema, strict, open, format }) => {
            validate_command(&input, &schema, strict, open, &format)
        }
        Some(Commands::Compile { schema, output }) => {
            compile_command(&schema, output.as_deref())
        }
        Some(Commands::CheckSchema { schema }) => {
            check_schema_command(&schema)
        }
        Some(Commands::ExportJtd { schema, output }) => {
            export_jtd_command(&schema, output.as_deref())
        }
        None => {
            // Show help if no subcommand provided
            println!("{}", Cli::command().render_long_help());
            Ok(())
        }
    }
}

fn validate_command(
    input: &str,
    schema: &str,
    strict: bool,
    open: bool,
    format: &str,
) -> Result<()> {
    tracing::info!("Validating {} against {}", input, schema);
    tracing::info!("Strict mode: {}, Open mode: {}, Format: {}", strict, open, format);

    // TODO: Implement actual validation
    println!("Validation command (not yet implemented)");
    Ok(())
}

fn compile_command(schema: &str, output: Option<&str>) -> Result<()> {
    tracing::info!("Compiling schema {}", schema);

    // TODO: Implement actual compilation
    println!("Compile command (not yet implemented)");
    Ok(())
}

fn check_schema_command(schema: &str) -> Result<()> {
    tracing::info!("Checking schema {}", schema);

    // TODO: Implement actual schema check
    println!("Check schema command (not yet implemented)");
    Ok(())
}

fn export_jtd_command(schema: &str, output: Option<&str>) -> Result<()> {
    tracing::info!("Exporting schema {} to JTD", schema);

    // TODO: Implement actual JTD export
    println!("Export JTD command (not yet implemented)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test that CLI parsing doesn't panic
        use std::env;
        use std::ffi::OsString;

        let cli = Cli::try_parse_from([
            "graphql-ish-schema-validator",
            "--log-level",
            "debug",
            "validate",
            "test.yml",
            "--schema",
            "schema.graphql",
        ]);

        assert!(cli.is_ok());
    }
}
```

- [ ] **Step 3: Test CLI crate compiles**

Run: `cargo check -p graphql-ish-schema-validator-cli`
Expected: Crate compiles without errors

- [ ] **Step 4: Build CLI binary**

Run: `cargo build --release -p graphql-ish-schema-validator-cli`
Expected: Binary builds successfully

- [ ] **Step 5: Test CLI help**

Run: `cargo run -p graphql-ish-schema-validator-cli -- --help`
Expected: Help text displayed

- [ ] **Step 6: Commit CLI crate**

```bash
git add crates/graphql-ish-schema-validator-cli/
git commit -m "feat: add graphql-ish-schema-validator-cli with stubbed subcommands"
```

---

## Task 7: Clean Up Old Crate Names

**Files:**
- Delete: `crates/gqlsdl-ir/`
- Delete: `crates/gqlsdl-parser/`
- Delete: `crates/gqlsdl-validator/`
- Delete: `crates/gqlsdl-cli/`
- Delete: Any other `gqlsdl-*` directories

**Step 1: Verify all new crates work**

Run: `cargo test --workspace`
Expected: All tests pass in new crates

- [ ] **Step 2: Remove old crate directories**

```bash
# Remove old crates after verification
rm -rf crates/gqlsdl-ir
rm -rf crates/gqlsdl-parser
rm -rf crates/gqlsdl-validator
rm -rf crates/gqlsdl-cli
```

- [ ] **Step 3: Update workspace manifest**

Update `Cargo.toml` to remove old crate references (if any).

- [ ] **Step 4: Verify workspace is clean**

Run: `cargo check`
Expected: Workspace compiles, no references to old crates

- [ ] **Step 5: Commit cleanup**

```bash
git add Cargo.toml
git commit -m "refactor: remove old gqlsdl-* crate directories"
```

---

## Task 8: Update Documentation References

**Files:**
- Modify: `README.md` (if exists)
- Modify: `docs/plans/01-initial-attempt/*.md`
- Modify: Any other documentation referencing old names

**Step 1: Update README**

Update any README files to use new crate names and binary names.

- [ ] **Step 2: Update plan documentation references**

Update cross-references in `01-initial-attempt` plans to reference new crate names.

- [ ] **Step 3: Update examples**

Update code examples in documentation to use new public API names.

- [ ] **Step 4: Verify documentation builds**

Run: `cargo doc --no-deps --workspace`
Expected: Documentation builds successfully

- [ ] **Step 5: Commit documentation updates**

```bash
git add README.md docs/plans/01-initial-attempt/
git commit -m "docs: update references to new crate names and API"
```

---

## Verification

**Step 1: Full workspace test**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 2: Check workspace compiles**

Run: `cargo check --workspace`
Expected: No compilation errors

- [ ] **Step 3: Build release binary**

Run: `cargo build --release --workspace`
Expected: All crates build successfully

- [ ] **Step 4: Test public API**

Create a quick test file `test_api.rs`:

```rust
use graphql_ish_schema_validator::{validate_yaml_from_schema, ValidationOptions};

fn main() {
    let schema = r#"
        input Test @closed {
            value: String!
        }
    "#;

    let yaml = r#"
        value: test
    "#;

    let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());
    assert!(result.valid);
}
```

Run: `cargo run --example test_api`
Expected: Example runs successfully

- [ ] **Step 5: Final verification summary**

Verify:
- ✅ All crates renamed to `graphql-ish-schema-validator-*` pattern
- ✅ Top-level `graphql-ish-schema-validator` crate with public API
- ✅ Binary names: `graphql-ish-schema-validator` and `gqlsdl`
- ✅ Default features enable YAML and CLI
- ✅ Optional features: `json-schema-export`, `http-registry`
- ✅ No references to old `gqlsdl-*` names
- ✅ All tests pass
- ✅ Documentation updated

- [ ] **Step 6: Final commit**

```bash
git add .
git commit -m "feat: complete migration to graphql-ish-schema-validator ecosystem"
```

---

## Summary

This plan transforms the existing `gqlsdl-*` crates into a unified `graphql-ish-schema-validator` library ecosystem:

**Created crates:**
1. `graphql-ish-schema-validator-ir` - JTD-like internal representation
2. `graphql-ish-schema-validator-parser` - SDL parser and AST
3. `graphql-ish-schema-validator-validator` - Validation engine
4. `graphql-ish-schema-validator-registry` - Schema registry (not yet implemented in this plan)
5. `graphql-ish-schema-validator-cli` - CLI tool
6. `graphql-ish-schema-validator` - Top-level public API

**Key changes:**
- Unified naming across all crates
- Clear public API: `validate_yaml_from_schema()` and `validate_json_from_schema()`
- Feature flags: default = ["yaml", "cli"], optional = ["json-schema-export", "http-registry"]
- Binary names: `graphql-ish-schema-validator` and short alias `gqlsdl`
- Workspace-level dependency version pinning

**Next steps:**
- Implement AST → IR lowering (see [03-compiler-lowering.md](../01-initial-attempt/03-compiler-lowering.md))
- Implement full validation engine (see [04-validator-runtime.md](../01-initial-attempt/04-validator-runtime.md))
- Add registry system (see [06-registry-system.md](../01-initial-attempt/06-registry-system.md))
- Implement CLI subcommands fully (see [03-cli-improvements.md](./03-cli-improvements.md))
