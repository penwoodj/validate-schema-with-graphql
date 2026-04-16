# Public API Design Plan

**For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Design and implement a robust, user-friendly public API for validating YAML/JSON documents against GraphQL-ish schemas with excellent error reporting.

**Architecture:** Two primary functions (`validate_yaml_from_schema`, `validate_json_from_schema`) backed by configurable `ValidationOptions`, returning rich `ValidationResult` with detailed error diagnostics.

**Tech Stack:** Rust, serde, serde_json, serde_yaml_ng, tracing, thiserror, miette

---

## Context and Rationale

**Public API requirements from research:**
- Simple entry points: `validate_yaml_from_schema()` and `validate_json_from_schema()`
- Configuration via `ValidationOptions` struct
- Rich error reporting with `instance_path`, `schema_path`, codes, messages, hints
- Error resilience: catch panics, handle malformed input gracefully
- Support for both strict and open validation modes
- Performance metrics included in results

**Key design principles:**
1. **Ergonomic**: Simple common case, powerful advanced case
2. **Explicit**: Errors include all context needed for debugging
3. **Resilient**: Never panic on invalid input
4. **Performant**: Minimal allocations, fast validation

**References:**
- [01-initial-attempt/00-overview.md](../01-initial-attempt/00-overview.md) - Architecture overview
- [00-rename-and-restructure.md](./00-rename-and-restructure.md) - Crate restructure
- [02-logging-and-diagnostics.md](./02-logging-and-diagnostics.md) - Logging integration

---

## Task 1: Enhance ValidationOptions

**Files:**
- Modify: `crates/graphql-ish-schema-validator-validator/src/options.rs`

**Step 1: Add missing validation options**

Update `crates/graphql-ish-schema-validator-validator/src/options.rs`:

```rust
//! Validation options and configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    File(PathBuf),

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

/// Schema source location
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaSource {
    /// Schema provided as a string
    Inline(String),

    /// Schema loaded from a file
    File(PathBuf),

    /// Schema fetched from a URL
    Url(String),

    /// Schema from registry (schema_id, version)
    Registry { schema_id: String, version: String },
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

    /// Schema source location (if not inline)
    #[serde(skip)]
    pub schema_source: Option<SchemaSource>,

    /// Cache compiled schemas (default: true)
    #[serde(default = "default_cache_schemas")]
    pub cache_schemas: bool,

    /// Schema cache directory
    #[serde(skip)]
    pub cache_dir: Option<PathBuf>,

    /// Enable rich error formatting with miette
    #[serde(default = "default_rich_errors")]
    pub rich_errors: bool,

    /// Include source location in errors (line/column numbers)
    #[serde(default = "default_include_source")]
    pub include_source_location: bool,

    /// HTTP timeout for remote schema fetching (seconds)
    #[serde(default = "default_http_timeout")]
    pub http_timeout_secs: u64,

    /// Allow type coercion in open mode (default: false)
    #[serde(default)]
    pub allow_type_coercion: bool,

    /// Treat validation warnings as errors
    #[serde(default)]
    pub warnings_as_errors: bool,
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
            schema_source: None,
            cache_schemas: default_cache_schemas(),
            cache_dir: None,
            rich_errors: default_rich_errors(),
            include_source_location: default_include_source(),
            http_timeout_secs: default_http_timeout(),
            allow_type_coercion: false,
            warnings_as_errors: false,
        }
    }
}

impl ValidationOptions {
    /// Create a builder for ValidationOptions
    pub fn builder() -> ValidationOptionsBuilder {
        ValidationOptionsBuilder::default()
    }

    /// Set validation mode
    pub fn with_mode(mut self, mode: ValidationMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set log level
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.log_level = level;
        self
    }

    /// Set schema source
    pub fn with_schema_source(mut self, source: SchemaSource) -> Self {
        self.schema_source = Some(source);
        self
    }

    /// Enable/disable caching
    pub fn with_cache(mut self, cache: bool) -> Self {
        self.cache_schemas = cache;
        self
    }

    /// Set cache directory
    pub fn with_cache_dir(mut self, dir: PathBuf) -> Self {
        self.cache_dir = Some(dir);
        self
    }

    /// Enable rich errors
    pub fn with_rich_errors(mut self, rich: bool) -> Self {
        self.rich_errors = rich;
        self
    }

    /// Set HTTP timeout
    pub fn with_http_timeout(mut self, timeout_secs: u64) -> Self {
        self.http_timeout_secs = timeout_secs;
        self
    }
}

fn default_max_depth() -> usize { 100 }
fn default_cache_schemas() -> bool { true }
fn default_rich_errors() -> bool { true }
fn default_include_source() -> bool { true }
fn default_http_timeout() -> u64 { 30 }

/// Builder for ValidationOptions
#[derive(Debug, Clone, Default)]
pub struct ValidationOptionsBuilder {
    options: ValidationOptions,
}

impl ValidationOptionsBuilder {
    /// Set validation mode
    pub fn mode(mut self, mode: ValidationMode) -> Self {
        self.options.mode = mode;
        self
    }

    /// Set maximum nesting depth
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.options.max_depth = depth;
        self
    }

    /// Set root schema name
    pub fn root_schema_name(mut self, name: impl Into<String>) -> Self {
        self.options.root_schema_name = Some(name.into());
        self
    }

    /// Set log level
    pub fn log_level(mut self, level: LogLevel) -> Self {
        self.options.log_level = level;
        self
    }

    /// Set log output destination
    pub fn log_output(mut self, output: LogOutput) -> Self {
        self.options.log_output = output;
        self
    }

    /// Set schema format
    pub fn schema_format(mut self, format: SchemaFormat) -> Self {
        self.options.schema_format = format;
        self
    }

    /// Set schema source
    pub fn schema_source(mut self, source: SchemaSource) -> Self {
        self.options.schema_source = Some(source);
        self
    }

    /// Enable/disable schema caching
    pub fn cache_schemas(mut self, cache: bool) -> Self {
        self.options.cache_schemas = cache;
        self
    }

    /// Set cache directory
    pub fn cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.options.cache_dir = Some(dir.into());
        self
    }

    /// Enable rich error formatting
    pub fn rich_errors(mut self, rich: bool) -> Self {
        self.options.rich_errors = rich;
        self
    }

    /// Include source location in errors
    pub fn include_source_location(mut self, include: bool) -> Self {
        self.options.include_source_location = include;
        self
    }

    /// Set HTTP timeout
    pub fn http_timeout(mut self, timeout_secs: u64) -> Self {
        self.options.http_timeout_secs = timeout_secs;
        self
    }

    /// Allow type coercion in open mode
    pub fn allow_type_coercion(mut self, allow: bool) -> Self {
        self.options.allow_type_coercion = allow;
        self
    }

    /// Treat warnings as errors
    pub fn warnings_as_errors(mut self, treat: bool) -> Self {
        self.options.warnings_as_errors = treat;
        self
    }

    /// Build the ValidationOptions
    pub fn build(self) -> ValidationOptions {
        self.options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_default() {
        let options = ValidationOptions::default();
        assert_eq!(options.mode, ValidationMode::Strict);
        assert_eq!(options.max_depth, 100);
        assert_eq!(options.cache_schemas, true);
    }

    #[test]
    fn test_options_builder() {
        let options = ValidationOptions::builder()
            .mode(ValidationMode::Open)
            .max_depth(50)
            .log_level(LogLevel::Debug)
            .rich_errors(true)
            .build();

        assert_eq!(options.mode, ValidationMode::Open);
        assert_eq!(options.max_depth, 50);
        assert_eq!(options.log_level, LogLevel::Debug);
        assert!(options.rich_errors);
    }

    #[test]
    fn test_options_with_methods() {
        let options = ValidationOptions::default()
            .with_mode(ValidationMode::Open)
            .with_log_level(LogLevel::Trace);

        assert_eq!(options.mode, ValidationMode::Open);
        assert_eq!(options.log_level, LogLevel::Trace);
    }
}
```

- [ ] **Step 2: Test options compile**

Run: `cargo check -p graphql-ish-schema-validator-validator`
Expected: No compilation errors

- [ ] **Step 3: Test options tests**

Run: `cargo test -p graphql-ish-schema-validator-validator options`
Expected: All options tests pass

- [ ] **Step 4: Commit enhanced options**

```bash
git add crates/graphql-ish-schema-validator-validator/src/options.rs
git commit -m "feat: enhance ValidationOptions with builder and additional options"
```

---

## Task 2: Enhance ValidationResult

**Files:**
- Modify: `crates/graphql-ish-schema-validator-validator/src/result.rs`

**Step 1: Add missing result fields**

Update `crates/graphql-ish-schema-validator-validator/src/result.rs`:

```rust
//! Validation result types

use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::collections::VecDeque;
use std::fmt;

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

    /// Schema version used for validation
    pub schema_version: Option<String>,

    /// Document type (YAML or JSON)
    pub document_type: DocumentType,

    /// Number of nodes validated
    pub node_count: usize,

    /// Maximum nesting depth encountered
    pub max_depth_encountered: usize,

    /// Cache hit/miss information
    pub cache_info: Option<CacheInfo>,
}

/// Cache information for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    /// Whether the schema was served from cache
    pub cache_hit: bool,

    /// Cache key used
    pub cache_key: String,
}

/// Document type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentType {
    /// YAML document
    Yaml,

    /// JSON document
}

impl fmt::Display for DocumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocumentType::Yaml => write!(f, "YAML"),
            DocumentType::Json => write!(f, "JSON"),
        }
    }
}

impl ValidationResult {
    /// Create a new successful validation result
    pub fn success(
        schema_name: Option<String>,
        document_type: DocumentType,
        duration: Duration,
    ) -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            duration,
            schema_name,
            schema_version: None,
            document_type,
            node_count: 0,
            max_depth_encountered: 0,
            cache_info: None,
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
            schema_version: None,
            document_type,
            node_count: 0,
            max_depth_encountered: 0,
            cache_info: None,
        }
    }

    /// Add a warning to the result
    pub fn add_warning(&mut self, warning: ValidationError) {
        self.warnings.push(warning);
    }

    /// Add multiple warnings to the result
    pub fn add_warnings(&mut self, warnings: impl IntoIterator<Item = ValidationError>) {
        self.warnings.extend(warnings);
    }

    /// Check if the result has any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if the result has any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get total number of issues (errors + warnings)
    pub fn total_issues(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }

    /// Set schema version
    pub fn set_schema_version(&mut self, version: String) {
        self.schema_version = Some(version);
    }

    /// Set node count
    pub fn set_node_count(&mut self, count: usize) {
        self.node_count = count;
    }

    /// Set max depth encountered
    pub fn set_max_depth(&mut self, depth: usize) {
        self.max_depth_encountered = depth;
    }

    /// Set cache info
    pub fn set_cache_info(&mut self, info: CacheInfo) {
        self.cache_info = Some(info);
    }

    /// Format validation summary for display
    pub fn format_summary(&self) -> String {
        let errors = self.errors.len();
        let warnings = self.warnings.len();
        let duration_ms = self.duration.as_millis();

        if self.valid {
            format!(
                "✓ Valid ({}) in {}ms ({} nodes, depth {}/{})",
                self.document_type,
                duration_ms,
                self.node_count,
                self.max_depth_encountered,
                self.schema_name.as_deref().unwrap_or("unknown")
            )
        } else {
            format!(
                "✗ Invalid ({}) in {}ms: {} error(s), {} warning(s)",
                self.document_type,
                duration_ms,
                errors,
                warnings
            )
        }
    }

    /// Format detailed error report
    pub fn format_errors(&self) -> String {
        if self.errors.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        for (i, error) in self.errors.iter().enumerate() {
            output.push_str(&format!("\nError {}:\n", i + 1));
            output.push_str(&format!("  Code: {}\n", error.code));
            output.push_str(&format!("  Instance path: {}\n", error.instance_path));
            output.push_str(&format!("  Schema path: {}\n", error.schema_path));
            output.push_str(&format!("  Message: {}\n", error.message));
            if let Some(hint) = &error.hint {
                output.push_str(&format!("  Hint: {}\n", hint));
            }
            if let Some(location) = &error.source_location {
                output.push_str(&format!(
                    "  Location: line {}, column {}\n",
                    location.line, location.column
                ));
            }
        }
        output
    }

    /// Format detailed warning report
    pub fn format_warnings(&self) -> String {
        if self.warnings.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        for (i, warning) in self.warnings.iter().enumerate() {
            output.push_str(&format!("\nWarning {}:\n", i + 1));
            output.push_str(&format!("  Code: {}\n", warning.code));
            output.push_str(&format!("  Instance path: {}\n", warning.instance_path));
            output.push_str(&format!("  Message: {}\n", warning.message));
            if let Some(hint) = &warning.hint {
                output.push_str(&format!("  Hint: {}\n", hint));
            }
        }
        output
    }

    /// Export validation result as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Export validation result as JSON with custom indentation
    pub fn to_json_indent(&self, indent: usize) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
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

    /// Get a human-readable path representation
    pub fn as_human_readable(&self) -> String {
        if self.segments.is_empty() {
            return "(root)".to_string();
        }

        self.segments
            .iter()
            .map(|s| {
                if s.parse::<usize>().is_ok() {
                    format!("[{}]", s)
                } else {
                    format!(".{}", s)
                }
            })
            .collect::<String>()
            .replacen(".", "", 1) // Remove leading dot
    }

    /// Clone with a new segment pushed
    pub fn with_segment(&self, segment: &str) -> Self {
        let mut clone = self.clone();
        clone.push(segment);
        clone
    }

    /// Clone with an index pushed
    pub fn with_index(&self, index: usize) -> Self {
        let mut clone = self.clone();
        clone.push_index(index);
        clone
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

    /// Get the current depth of the path
    pub fn depth(&self) -> usize {
        self.segments.len()
    }
}

impl Default for InstancePath {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for InstancePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_human_readable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_path_new() {
        let path = InstancePath::new();
        assert_eq!(path.pointer, "");
        assert_eq!(path.depth(), 0);
    }

    #[test]
    fn test_instance_path_push() {
        let mut path = InstancePath::new();
        path.push("users");
        path.push_index(0);
        path.push("name");

        assert_eq!(path.pointer, "/users/0/name");
        assert_eq!(path.depth(), 3);
    }

    #[test]
    fn test_instance_path_human_readable() {
        let mut path = InstancePath::new();
        path.push("agentic_workflow");
        path.push("steps");
        path.push_index(0);
        path.push("prompt");

        assert_eq!(path.as_human_readable(), "agentic_workflow.steps[0].prompt");
    }

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success(
            Some("test_schema".to_string()),
            DocumentType::Yaml,
            Duration::from_millis(10),
        );

        assert!(result.valid);
        assert!(!result.has_errors());
        assert_eq!(result.schema_name.as_deref(), Some("test_schema"));
    }

    #[test]
    fn test_validation_result_failure() {
        let error = ValidationError {
            instance_path: "/test".to_string(),
            schema_path: "/schema".to_string(),
            code: ValidationErrorCode::TypeMismatch,
            message: "Type mismatch".to_string(),
            hint: Some("Check type".to_string()),
            severity: crate::error::ErrorSeverity::Error,
            source_location: Some(crate::error::SourceLocation { line: 1, column: 1 }),
        };

        let result = ValidationResult::failure(
            vec![error],
            Some("test_schema".to_string()),
            DocumentType::Json,
            Duration::from_millis(5),
        );

        assert!(!result.valid);
        assert!(result.has_errors());
        assert_eq!(result.total_issues(), 1);
    }

    #[test]
    fn test_validation_result_format_summary() {
        let result = ValidationResult::success(
            Some("test".to_string()),
            DocumentType::Yaml,
            Duration::from_millis(42),
        );

        let summary = result.format_summary();
        assert!(summary.contains("Valid"));
        assert!(summary.contains("YAML"));
        assert!(summary.contains("42ms"));
    }

    #[test]
    fn test_validation_result_format_errors() {
        let error = ValidationError {
            instance_path: "/test/path".to_string(),
            schema_path: "/schema/path".to_string(),
            code: ValidationErrorCode::TypeMismatch,
            message: "Expected string, got number".to_string(),
            hint: Some("Use quotes for strings".to_string()),
            severity: crate::error::ErrorSeverity::Error,
            source_location: None,
        };

        let result = ValidationResult::failure(
            vec![error],
            None,
            DocumentType::Json,
            Duration::from_millis(1),
        );

        let errors = result.format_errors();
        assert!(errors.contains("Error 1"));
        assert!(errors.contains("TypeMismatch"));
        assert!(errors.contains("/test/path"));
        assert!(errors.contains("Expected string, got number"));
        assert!(errors.contains("Use quotes for strings"));
    }
}
```

- [ ] **Step 2: Test result module compiles**

Run: `cargo check -p graphql-ish-schema-validator-validator`
Expected: No compilation errors

- [ ] **Step 3: Run result module tests**

Run: `cargo test -p graphql-ish-schema-validator-validator result`
Expected: All result tests pass

- [ ] **Step 4: Commit enhanced result module**

```bash
git add crates/graphql-ish-schema-validator-validator/src/result.rs
git commit -m "feat: enhance ValidationResult with detailed reporting and formatting"
```

---

## Task 3: Enhance ValidationError with Source Location

**Files:**
- Modify: `crates/graphql-ish-schema-validator-validator/src/error.rs`

**Step 1: Enhance error types**

Update `crates/graphql-ish-schema-validator-validator/src/error.rs`:

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

    /// Additional context data
    pub context: Option<ErrorContext>,
}

/// Additional error context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Expected value (if applicable)
    pub expected: Option<String>,

    /// Actual value found (if applicable)
    pub actual: Option<String>,

    /// Allowed values (for enum errors)
    pub allowed_values: Option<Vec<String>>,

    /// Minimum value (for range errors)
    pub min: Option<serde_json::Number>,

    /// Maximum value (for range errors)
    pub max: Option<serde_json::Number>,

    /// Pattern (for regex errors)
    pub pattern: Option<String>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(
        code: ValidationErrorCode,
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        message: String,
    ) -> Self {
        Self {
            instance_path,
            schema_path,
            code,
            message,
            hint: None,
            severity: ErrorSeverity::Error,
            source_location: None,
            context: None,
        }
    }

    /// Create a new validation error with hint
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Create a new validation error with source location
    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.source_location = Some(SourceLocation { line, column });
        self
    }

    /// Create a new validation error with context
    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Set error severity
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Create a type mismatch error
    pub fn type_mismatch(
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::new(
            ValidationErrorCode::TypeMismatch,
            instance_path,
            schema_path,
            format!("Expected {}, got {}", expected.into(), actual.into()),
        )
        .with_context(ErrorContext {
            expected: Some(expected.into()),
            actual: Some(actual.into()),
            allowed_values: None,
            min: None,
            max: None,
            pattern: None,
        })
    }

    /// Create a required property missing error
    pub fn required_property_missing(
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        property_name: impl Into<String>,
    ) -> Self {
        Self::new(
            ValidationErrorCode::RequiredPropertyMissing,
            instance_path,
            schema_path,
            format!("Required property '{}' is missing", property_name.into()),
        )
    }

    /// Create an additional property error
    pub fn additional_property_not_allowed(
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        property_name: impl Into<String>,
    ) -> Self {
        Self::new(
            ValidationErrorCode::AdditionalPropertyNotAllowed,
            instance_path,
            schema_path,
            format!("Additional property '{}' is not allowed", property_name.into()),
        )
    }

    /// Create an invalid enum value error
    pub fn invalid_enum_value(
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        value: impl Into<String>,
        allowed_values: Vec<String>,
    ) -> Self {
        Self::new(
            ValidationErrorCode::InvalidEnumValue,
            instance_path,
            schema_path,
            format!("Invalid enum value '{}'", value.into()),
        )
        .with_context(ErrorContext {
            expected: None,
            actual: Some(value.into()),
            allowed_values: Some(allowed_values),
            min: None,
            max: None,
            pattern: None,
        })
        .with_hint(format!("Allowed values: {}", allowed_values.join(", ")))
    }

    /// Create a pattern mismatch error
    pub fn pattern_mismatch(
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        value: impl Into<String>,
        pattern: impl Into<String>,
    ) -> Self {
        Self::new(
            ValidationErrorCode::PatternMismatch,
            instance_path,
            schema_path,
            format!("Value '{}' does not match pattern", value.into()),
        )
        .with_context(ErrorContext {
            expected: None,
            actual: Some(value.into()),
            allowed_values: None,
            min: None,
            max: None,
            pattern: Some(pattern.into()),
        })
        .with_hint(format!("Pattern: {}", pattern.into()))
    }

    /// Create a minimum length violation error
    pub fn minimum_length_violation(
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        actual: usize,
        minimum: usize,
    ) -> Self {
        Self::new(
            ValidationErrorCode::MinimumLengthViolation,
            instance_path,
            schema_path,
            format!("Length {} is less than minimum {}", actual, minimum),
        )
        .with_context(ErrorContext {
            expected: Some(minimum.to_string()),
            actual: Some(actual.to_string()),
            allowed_values: None,
            min: None,
            max: None,
            pattern: None,
        })
    }

    /// Create a maximum length violation error
    pub fn maximum_length_violation(
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        actual: usize,
        maximum: usize,
    ) -> Self {
        Self::new(
            ValidationErrorCode::MaximumLengthViolation,
            instance_path,
            schema_path,
            format!("Length {} exceeds maximum {}", actual, maximum),
        )
        .with_context(ErrorContext {
            expected: Some(maximum.to_string()),
            actual: Some(actual.to_string()),
            allowed_values: None,
            min: None,
            max: None,
            pattern: None,
        })
    }

    /// Create a minimum value violation error
    pub fn minimum_value_violation(
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        actual: impl Into<String>,
        minimum: impl Into<String>,
    ) -> Self {
        Self::new(
            ValidationErrorCode::MinimumValueViolation,
            instance_path,
            schema_path,
            format!("Value {} is less than minimum {}", actual.into(), minimum.into()),
        )
    }

    /// Create a maximum value violation error
    pub fn maximum_value_violation(
        instance_path: JsonPointer,
        schema_path: JsonPointer,
        actual: impl Into<String>,
        maximum: impl Into<String>,
    ) -> Self {
        Self::new(
            ValidationErrorCode::MaximumValueViolation,
            instance_path,
            schema_path,
            format!("Value {} exceeds maximum {}", actual.into(), maximum.into()),
        )
    }
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

    /// Duplicate key detected
    DuplicateKey,

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
            ValidationErrorCode::DuplicateKey => write!(f, "duplicate_key"),
            ValidationErrorCode::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for ValidationErrorCode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "type_mismatch" => Ok(ValidationErrorCode::TypeMismatch),
            "required_property_missing" => Ok(ValidationErrorCode::RequiredPropertyMissing),
            "additional_property_not_allowed" => Ok(ValidationErrorCode::AdditionalPropertyNotAllowed),
            "invalid_enum_value" => Ok(ValidationErrorCode::InvalidEnumValue),
            "pattern_mismatch" => Ok(ValidationErrorCode::PatternMismatch),
            "minimum_length_violation" => Ok(ValidationErrorCode::MinimumLengthViolation),
            "maximum_length_violation" => Ok(ValidationErrorCode::MaximumLengthViolation),
            "minimum_value_violation" => Ok(ValidationErrorCode::MinimumValueViolation),
            "maximum_value_violation" => Ok(ValidationErrorCode::MaximumValueViolation),
            "circular_reference" => Ok(ValidationErrorCode::CircularReference),
            "invalid_type_reference" => Ok(ValidationErrorCode::InvalidTypeReference),
            "schema_error" => Ok(ValidationErrorCode::SchemaError),
            "parse_error" => Ok(ValidationErrorCode::ParseError),
            "duplicate_key" => Ok(ValidationErrorCode::DuplicateKey),
            "unknown" => Ok(ValidationErrorCode::Unknown),
            _ => Err(format!("unknown error code: {}", s)),
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

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Error => write!(f, "error"),
            ErrorSeverity::Warning => write!(f, "warning"),
        }
    }
}

/// Source location in a document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// Line number (1-indexed)
    pub line: usize,

    /// Column number (1-indexed)
    pub column: usize,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_new() {
        let error = ValidationError::new(
            ValidationErrorCode::TypeMismatch,
            "/test/path".to_string(),
            "/schema/path".to_string(),
            "Test error".to_string(),
        );

        assert_eq!(error.instance_path, "/test/path");
        assert_eq!(error.schema_path, "/schema/path");
        assert_eq!(error.code, ValidationErrorCode::TypeMismatch);
        assert_eq!(error.message, "Test error");
    }

    #[test]
    fn test_validation_error_type_mismatch() {
        let error = ValidationError::type_mismatch(
            "/value".to_string(),
            "/schema".to_string(),
            "string",
            "123",
        );

        assert_eq!(error.code, ValidationErrorCode::TypeMismatch);
        assert!(error.message.contains("Expected string, got 123"));
        assert!(error.context.is_some());
        assert_eq!(error.context.unwrap().expected, Some("string".to_string()));
    }

    #[test]
    fn test_validation_error_required_property() {
        let error = ValidationError::required_property_missing(
            "/object".to_string(),
            "/schema".to_string(),
            "name",
        );

        assert_eq!(error.code, ValidationErrorCode::RequiredPropertyMissing);
        assert!(error.message.contains("Required property 'name' is missing"));
    }

    #[test]
    fn test_validation_error_invalid_enum() {
        let error = ValidationError::invalid_enum_value(
            "/value".to_string(),
            "/schema".to_string(),
            "other",
            vec!["one".to_string(), "two".to_string(), "three".to_string()],
        );

        assert_eq!(error.code, ValidationErrorCode::InvalidEnumValue);
        assert!(error.message.contains("Invalid enum value 'other'"));
        assert!(error.hint.is_some());
        assert!(error.hint.unwrap().contains("one, two, three"));
    }

    #[test]
    fn test_validation_error_code_display() {
        assert_eq!(ValidationErrorCode::TypeMismatch.to_string(), "type_mismatch");
        assert_eq!(ValidationErrorCode::PatternMismatch.to_string(), "pattern_mismatch");
    }

    #[test]
    fn test_validation_error_code_from_str() {
        assert_eq!(
            "type_mismatch".parse::<ValidationErrorCode>().unwrap(),
            ValidationErrorCode::TypeMismatch
        );
        assert_eq!(
            "pattern_mismatch".parse::<ValidationErrorCode>().unwrap(),
            ValidationErrorCode::PatternMismatch
        );
    }
}
```

- [ ] **Step 2: Test error module compiles**

Run: `cargo check -p graphql-ish-schema-validator-validator`
Expected: No compilation errors

- [ ] **Step 3: Run error module tests**

Run: `cargo test -p graphql-ish-schema-validator-validator error`
Expected: All error tests pass

- [ ] **Step 4: Commit enhanced error module**

```bash
git add crates/graphql-ish-schema-validator-validator/src/error.rs
git commit -m "feat: enhance ValidationError with context and convenience methods"
```

---

## Task 4: Implement Panic-Resilient Validation Wrapper

**Files:**
- Modify: `crates/graphql-ish-schema-validator-validator/src/validator.rs`

**Step 1: Add panic-catching wrapper**

Update `crates/graphql-ish-schema-validator-validator/src/validator.rs`:

```rust
//! Core validation engine with panic resilience

use graphql_ish_schema_validator_ir::CompiledSchema;
use serde_json::Value;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;
use tracing::{debug, error, instrument, trace};

use super::options::ValidationOptions;
use super::result::{DocumentType, ValidationResult};
use super::error::{ValidationError, ValidationErrorCode};

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

    /// Validate a JSON value against the schema (panic-resilient)
    #[instrument(skip(self, json))]
    pub fn validate_json(&self, json: &str) -> ValidationResult {
        let start = Instant::now();

        // Catch panics during parsing
        let parse_result = catch_unwind(AssertUnwindSafe(|| {
            serde_json::from_str::<Value>(json)
        }));

        let value = match parse_result {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                let duration = start.elapsed();
                error!("JSON parse error: {}", e);
                return ValidationError::new(
                    ValidationErrorCode::ParseError,
                    String::new(),
                    String::new(),
                    format!("Failed to parse JSON: {}", e),
                )
                .with_hint("Check JSON syntax")
                .into_result(
                    self.schema.schema_id.clone(),
                    DocumentType::Json,
                    duration,
                );
            }
            Err(_) => {
                let duration = start.elapsed();
                error!("Panic during JSON parsing");
                return ValidationError::new(
                    ValidationErrorCode::ParseError,
                    String::new(),
                    String::new(),
                    "Internal error during JSON parsing (caught panic)".to_string(),
                )
                .into_result(
                    self.schema.schema_id.clone(),
                    DocumentType::Json,
                    duration,
                );
            }
        };

        // Catch panics during validation
        let validation_result = catch_unwind(AssertUnwindSafe(|| {
            self.validate_value(&value)
        }));

        match validation_result {
            Ok(result) => {
                let duration = start.elapsed();
                debug!("JSON validation completed in {:?}", duration);
                result.with_duration(duration)
            }
            Err(_) => {
                let duration = start.elapsed();
                error!("Panic during JSON validation");
                ValidationError::new(
                    ValidationErrorCode::SchemaError,
                    String::new(),
                    String::new(),
                    "Internal error during validation (caught panic)".to_string(),
                )
                .with_hint("This is a bug - please report it")
                .into_result(
                    self.schema.schema_id.clone(),
                    DocumentType::Json,
                    duration,
                )
            }
        }
    }

    /// Validate a YAML value against the schema (panic-resilient)
    #[instrument(skip(self, yaml))]
    pub fn validate_yaml(&self, yaml: &str) -> ValidationResult {
        let start = Instant::now();

        // Catch panics during parsing
        let parse_result = catch_unwind(AssertUnwindSafe(|| {
            serde_yaml_ng::from_str::<Value>(yaml)
        }));

        let value = match parse_result {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                let duration = start.elapsed();
                error!("YAML parse error: {}", e);
                return ValidationError::new(
                    ValidationErrorCode::ParseError,
                    String::new(),
                    String::new(),
                    format!("Failed to parse YAML: {}", e),
                )
                .with_hint("Check YAML syntax and indentation")
                .into_result(
                    self.schema.schema_id.clone(),
                    DocumentType::Yaml,
                    duration,
                );
            }
            Err(_) => {
                let duration = start.elapsed();
                error!("Panic during YAML parsing");
                return ValidationError::new(
                    ValidationErrorCode::ParseError,
                    String::new(),
                    String::new(),
                    "Internal error during YAML parsing (caught panic)".to_string(),
                )
                .into_result(
                    self.schema.schema_id.clone(),
                    DocumentType::Yaml,
                    duration,
                );
            }
        };

        // Catch panics during validation
        let validation_result = catch_unwind(AssertUnwindSafe(|| {
            self.validate_value(&value)
        }));

        match validation_result {
            Ok(result) => {
                let duration = start.elapsed();
                debug!("YAML validation completed in {:?}", duration);
                result.with_duration(duration)
            }
            Err(_) => {
                let duration = start.elapsed();
                error!("Panic during YAML validation");
                ValidationError::new(
                    ValidationErrorCode::SchemaError,
                    String::new(),
                    String::new(),
                    "Internal error during validation (caught panic)".to_string(),
                )
                .with_hint("This is a bug - please report it")
                .into_result(
                    self.schema.schema_id.clone(),
                    DocumentType::Yaml,
                    duration,
                )
            }
        }
    }

    /// Core validation logic (implemented in later tasks)
    fn validate_value(&self, value: &Value) -> ValidationResult {
        // TODO: Implement actual validation logic
        // For now, always succeed
        ValidationResult::success(
            self.schema.schema_id.clone(),
            DocumentType::Yaml, // Will be set correctly by caller
            std::time::Duration::ZERO,
        )
    }
}

// Extension trait to convert ValidationError to ValidationResult
trait IntoValidationResult {
    fn into_result(
        self,
        schema_name: Option<String>,
        document_type: DocumentType,
        duration: std::time::Duration,
    ) -> ValidationResult;
}

impl IntoValidationResult for ValidationError {
    fn into_result(
        self,
        schema_name: Option<String>,
        document_type: DocumentType,
        duration: std::time::Duration,
    ) -> ValidationResult {
        ValidationResult::failure(
            vec![self],
            schema_name,
            document_type,
            duration,
        )
    }
}

// Extension trait to set duration on ValidationResult
trait WithDuration {
    fn with_duration(self, duration: std::time::Duration) -> ValidationResult;
}

impl WithDuration for ValidationResult {
    fn with_duration(mut self, duration: std::time::Duration) -> ValidationResult {
        self.duration = duration;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphql_ish_schema_validator_ir::*;

    fn create_test_schema() -> CompiledSchema {
        CompiledSchema {
            schema_id: Some("test".to_string()),
            schema_version: None,
            definitions: HashMap::new(),
        }
    }

    #[test]
    fn test_validate_json_success() {
        let schema = create_test_schema();
        let validator = Validator::new(schema, ValidationOptions::default());

        let result = validator.validate_json(r#"{"name": "test"}"#);

        // For now, we expect success (validation not implemented)
        assert!(result.valid);
    }

    #[test]
    fn test_validate_json_invalid_syntax() {
        let schema = create_test_schema();
        let validator = Validator::new(schema, ValidationOptions::default());

        let result = validator.validate_json(r#"{"name": "test""#); // Missing closing brace

        assert!(!result.valid);
        assert!(result.has_errors());
        assert_eq!(result.errors[0].code, ValidationErrorCode::ParseError);
    }

    #[test]
    fn test_validate_yaml_success() {
        let schema = create_test_schema();
        let validator = Validator::new(schema, ValidationOptions::default());

        let result = validator.validate_yaml("name: test");

        // For now, we expect success (validation not implemented)
        assert!(result.valid);
    }

    #[test]
    fn test_validate_yaml_invalid_syntax() {
        let schema = create_test_schema();
        let validator = Validator::new(schema, ValidationOptions::default());

        let result = validator.validate_yaml("name: test\n  bad: indent"); // Bad indentation

        assert!(!result.valid);
        assert!(result.has_errors());
        assert_eq!(result.errors[0].code, ValidationErrorCode::ParseError);
    }

    #[test]
    fn test_validator_options() {
        let schema = create_test_schema();
        let options = ValidationOptions::builder()
            .mode(ValidationMode::Open)
            .max_depth(50)
            .build();

        let validator = Validator::new(schema, options);

        // Just verify validator was created with options
        assert_eq!(validator.options.max_depth, 50);
    }
}
```

- [ ] **Step 2: Add missing imports and fix compilation**

Add missing imports at the top of the file:

```rust
use graphql_ish_schema_validator_ir::CompiledSchema;
use serde_json::Value;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;
use tracing::{debug, error, instrument, trace};

use super::options::{ValidationMode, ValidationOptions};
use super::result::{DocumentType, ValidationResult};
use super::error::{ValidationError, ValidationErrorCode};
```

- [ ] **Step 3: Test validator compiles**

Run: `cargo check -p graphql-ish-schema-validator-validator`
Expected: No compilation errors

- [ ] **Step 4: Run validator tests**

Run: `cargo test -p graphql-ish-schema-validator-validator validator`
Expected: All validator tests pass

- [ ] **Step 5: Commit panic-resilient validator**

```bash
git add crates/graphql-ish-schema-validator-validator/src/validator.rs
git commit -m "feat: add panic resilience to validator with catch_unwind"
```

---

## Task 5: Update Top-Level Public API Functions

**Files:**
- Modify: `crates/graphql-ish-schema-validator/src/lib.rs`

**Step 1: Update public API with error resilience**

Update `crates/graphql-ish-schema-validator/src/lib.rs`:

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
//!
//! # Error Resilience
//!
//! The library is designed to never panic on invalid input:
//! - Parse errors are caught and returned as `ValidationResult`
//! - Validation panics are caught with `catch_unwind` and converted to errors
//! - Malformed schemas are handled gracefully with clear error messages

#![warn(missing_docs)]
#![warn(clippy::all)]

// Re-export the public API
pub use graphql_ish_schema_validator_validator::{
    ValidationError,
    ValidationErrorCode,
    ValidationOptions,
    ValidationResult,
    ErrorSeverity,
    DocumentType,
    ValidationMode,
    LogLevel,
    LogOutput,
    SchemaFormat,
    SchemaSource,
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
    ParseError,
    ParseResult,
};

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;
use tracing::{debug, error, info, instrument};

use graphql_ish_schema_validator_parser::ParseResult;
use graphql_ish_schema_validator_validator::{
    DocumentType, Validator,
};

/// Validate a YAML document against a GraphQL-ish schema
///
/// This function is panic-resilient and will catch any panics during
/// parsing or validation, converting them to error results.
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
///
/// # Error Handling
///
/// The function handles various error cases:
/// - **Parse errors**: Invalid YAML syntax, returns `ParseError` code
/// - **Schema errors**: Invalid SDL, returns `SchemaError` code
/// - **Validation errors**: Document doesn't match schema
/// - **Panics**: Internal panics are caught and returned as errors
#[instrument(skip(yaml, schema))]
pub fn validate_yaml_from_schema(
    yaml: &str,
    schema: &str,
    options: ValidationOptions,
) -> ValidationResult {
    let start = Instant::now();

    // Catch panics during schema parsing
    let parse_result = catch_unwind(AssertUnwindSafe(|| {
        Parser::new(schema).parse()
    }));

    let document = match parse_result {
        Ok(Ok(doc)) => {
            info!("Schema parsed successfully");
            doc
        }
        Ok(Err(e)) => {
            let duration = start.elapsed();
            error!("Schema parse error: {}", e);
            return ValidationError::new(
                ValidationErrorCode::ParseError,
                String::new(),
                String::new(),
                format!("Failed to parse schema: {}", e),
            )
            .with_hint("Check GraphQL SDL syntax")
            .into_result(None, DocumentType::Yaml, duration);
        }
        Err(_) => {
            let duration = start.elapsed();
            error!("Panic during schema parsing");
            return ValidationError::new(
                ValidationErrorCode::ParseError,
                String::new(),
                String::new(),
                "Internal error during schema parsing (caught panic)".to_string(),
            )
            .with_hint("This is a bug - please report it")
            .into_result(None, DocumentType::Yaml, duration);
        }
    };

    // TODO: Lower AST to IR (will be implemented in subsequent tasks)
    debug!("Lowering AST to IR");
    let compiled_schema = catch_unwind(AssertUnwindSafe(|| {
        CompiledSchema {
            schema_id: document.metadata.schema_id.clone(),
            schema_version: document.metadata.schema_version.clone(),
            definitions: Default::default(),
        }
    }));

    let compiled_schema = match compiled_schema {
        Ok(schema) => schema,
        Err(_) => {
            let duration = start.elapsed();
            error!("Panic during IR lowering");
            return ValidationError::new(
                ValidationErrorCode::SchemaError,
                String::new(),
                String::new(),
                "Internal error during schema compilation (caught panic)".to_string(),
            )
            .with_hint("This is a bug - please report it")
            .into_result(None, DocumentType::Yaml, duration);
        }
    };

    // Set schema name from options or document
    let schema_name = options.root_schema_name.clone()
        .or(document.metadata.schema_id.clone());

    debug!("Creating validator with schema: {:?}", schema_name);

    // Create validator and validate
    let validator_result = catch_unwind(AssertUnwindSafe(|| {
        Validator::new(compiled_schema, options).validate_yaml(yaml)
    }));

    match validator_result {
        Ok(mut result) => {
            result.set_schema_version(document.metadata.schema_version.unwrap_or_default());
            info!("Validation completed: {}", result.format_summary());
            result
        }
        Err(_) => {
            let duration = start.elapsed();
            error!("Panic during validation");
            ValidationError::new(
                ValidationErrorCode::SchemaError,
                String::new(),
                String::new(),
                "Internal error during validation (caught panic)".to_string(),
            )
            .with_hint("This is a bug - please report it")
            .into_result(schema_name, DocumentType::Yaml, duration)
        }
    }
}

/// Validate a JSON document against a GraphQL-ish schema
///
/// This function is panic-resilient and will catch any panics during
/// parsing or validation, converting them to error results.
///
/// # Arguments
///
/// * `json` - The JSON document to validate (as a string)
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
///
/// # Error Handling
///
/// The function handles various error cases:
/// - **Parse errors**: Invalid JSON syntax, returns `ParseError` code
/// - **Schema errors**: Invalid SDL, returns `SchemaError` code
/// - **Validation errors**: Document doesn't match schema
/// - **Panics**: Internal panics are caught and returned as errors
#[instrument(skip(json, schema))]
pub fn validate_json_from_schema(
    json: &str,
    schema: &str,
    options: ValidationOptions,
) -> ValidationResult {
    let start = Instant::now();

    // Catch panics during schema parsing
    let parse_result = catch_unwind(AssertUnwindSafe(|| {
        Parser::new(schema).parse()
    }));

    let document = match parse_result {
        Ok(Ok(doc)) => {
            info!("Schema parsed successfully");
            doc
        }
        Ok(Err(e)) => {
            let duration = start.elapsed();
            error!("Schema parse error: {}", e);
            return ValidationError::new(
                ValidationErrorCode::ParseError,
                String::new(),
                String::new(),
                format!("Failed to parse schema: {}", e),
            )
            .with_hint("Check GraphQL SDL syntax")
            .into_result(None, DocumentType::Json, duration);
        }
        Err(_) => {
            let duration = start.elapsed();
            error!("Panic during schema parsing");
            return ValidationError::new(
                ValidationErrorCode::ParseError,
                String::new(),
                String::new(),
                "Internal error during schema parsing (caught panic)".to_string(),
            )
            .with_hint("This is a bug - please report it")
            .into_result(None, DocumentType::Json, duration);
        }
    };

    // TODO: Lower AST to IR (will be implemented in subsequent tasks)
    debug!("Lowering AST to IR");
    let compiled_schema = catch_unwind(AssertUnwindSafe(|| {
        CompiledSchema {
            schema_id: document.metadata.schema_id.clone(),
            schema_version: document.metadata.schema_version.clone(),
            definitions: Default::default(),
        }
    }));

    let compiled_schema = match compiled_schema {
        Ok(schema) => schema,
        Err(_) => {
            let duration = start.elapsed();
            error!("Panic during IR lowering");
            return ValidationError::new(
                ValidationErrorCode::SchemaError,
                String::new(),
                String::new(),
                "Internal error during schema compilation (caught panic)".to_string(),
            )
            .with_hint("This is a bug - please report it")
            .into_result(None, DocumentType::Json, duration);
        }
    };

    // Set schema name from options or document
    let schema_name = options.root_schema_name.clone()
        .or(document.metadata.schema_id.clone());

    debug!("Creating validator with schema: {:?}", schema_name);

    // Create validator and validate
    let validator_result = catch_unwind(AssertUnwindSafe(|| {
        Validator::new(compiled_schema, options).validate_json(json)
    }));

    match validator_result {
        Ok(mut result) => {
            result.set_schema_version(document.metadata.schema_version.unwrap_or_default());
            info!("Validation completed: {}", result.format_summary());
            result
        }
        Err(_) => {
            let duration = start.elapsed();
            error!("Panic during validation");
            ValidationError::new(
                ValidationErrorCode::SchemaError,
                String::new(),
                String::new(),
                "Internal error during validation (caught panic)".to_string(),
            )
            .with_hint("This is a bug - please report it")
            .into_result(schema_name, DocumentType::Json, duration)
        }
    }
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
///         steps: [Step!]!
///     }
/// "#;
///
/// let document = parse_schema(schema).unwrap();
/// assert_eq!(document.inputs.len(), 1);
/// assert_eq!(document.inputs[0].name, "Workflow");
/// ```
#[instrument(skip(schema))]
pub fn parse_schema(schema: &str) -> ParseResult<Document> {
    Parser::new(schema).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_yaml_valid_simple() {
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
    fn test_validate_yaml_malformed() {
        let schema = r#"
            input Simple @closed {
                value: String!
            }
        "#;

        let yaml = "value: test\n  bad: indent";

        let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());
        assert!(!result.valid);
        assert!(result.has_errors());
        assert_eq!(result.errors[0].code, ValidationErrorCode::ParseError);
    }

    #[test]
    fn test_validate_json_valid_simple() {
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
    fn test_validate_json_malformed() {
        let schema = r#"
            input Simple @closed {
                value: String!
            }
        "#;

        let json = r#"{"value": "test""#; // Missing closing brace

        let result = validate_json_from_schema(json, schema, ValidationOptions::default());
        assert!(!result.valid);
        assert!(result.has_errors());
        assert_eq!(result.errors[0].code, ValidationErrorCode::ParseError);
    }

    #[test]
    fn test_validate_schema_malformed() {
        let yaml = r#"
            value: test
        "#;

        let schema = "input Simple"; // Malformed schema

        let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());
        assert!(!result.valid);
        assert!(result.has_errors());
        assert_eq!(result.errors[0].code, ValidationErrorCode::ParseError);
    }

    #[test]
    fn test_validate_with_options() {
        let schema = r#"
            input Simple @closed {
                value: String!
            }
        "#;

        let yaml = r#"
            value: test
        "#;

        let options = ValidationOptions::builder()
            .mode(ValidationMode::Open)
            .log_level(LogLevel::Debug)
            .build();

        let result = validate_yaml_from_schema(yaml, schema, options);
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

    #[test]
    fn test_parse_schema_error() {
        let schema = "input Test"; // Malformed

        let result = parse_schema(schema);
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Test top-level API compiles**

Run: `cargo check -p graphql-ish-schema-validator`
Expected: No compilation errors

- [ ] **Step 3: Run top-level API tests**

Run: `cargo test -p graphql-ish-schema-validator`
Expected: All tests pass

- [ ] **Step 4: Commit updated public API**

```bash
git add crates/graphql-ish-schema-validator/src/lib.rs
git commit -m "feat: update public API with panic resilience and error handling"
```

---

## Task 6: Add Public API Documentation Examples

**Files:**
- Create: `crates/graphql-ish-schema-validator/examples/basic_validation.rs`
- Create: `crates/graphql-ish-schema-validator/examples/advanced_options.rs`
- Create: `crates/graphql-ish-schema-validator/examples/error_handling.rs`

**Step 1: Create basic validation example**

Create `crates/graphql-ish-schema-validator/examples/basic_validation.rs`:

```rust
//! Basic validation example
//!
//! Run with: cargo run --example basic_validation

use graphql_ish_schema_validator::{validate_yaml_from_schema, ValidationOptions};

fn main() {
    let schema = r#"
        input Workflow @closed {
            name: String!
            steps: [Step!]!
        }

        union Step = AgentStep | ToolStep

        input AgentStep @closed {
            prompt: String!
            model: String!
        }

        input ToolStep @closed {
            tool: String!
            input: Any
        }
    "#;

    let yaml = r#"
        name: My Workflow
        steps:
          - prompt: Generate code
            model: gpt-4
          - tool: execute-command
            input:
              command: echo hello
    "#;

    let result = validate_yaml_from_schema(yaml, schema, ValidationOptions::default());

    if result.valid {
        println!("✓ Document is valid!");
        println!("  Validated in {}ms", result.duration.as_millis());
    } else {
        println!("✗ Document is invalid!");
        println!("  Errors: {}", result.errors.len());
        println!("  Warnings: {}", result.warnings.len());

        for error in &result.errors {
            println!("\n  Error at {}:", error.instance_path);
            println!("    Code: {}", error.code);
            println!("    Message: {}", error.message);
            if let Some(hint) = &error.hint {
                println!("    Hint: {}", hint);
            }
        }
    }
}
```

- [ ] **Step 2: Create advanced options example**

Create `crates/graphql-ish-schema-validator/examples/advanced_options.rs`:

```rust
//! Advanced validation options example
//!
//! Run with: cargo run --example advanced_options

use graphql_ish_schema_validator::{
    validate_yaml_from_schema,
    ValidationOptions,
    ValidationMode,
    LogLevel,
    LogOutput,
};

fn main() {
    let schema = r#"
        input Config @open {
            name: String!
            debug: Boolean
        }
    "#;

    let yaml = r#"
        name: My Config
        debug: true
        extra_key: ignored in open mode
    "#;

    // Configure validation with advanced options
    let options = ValidationOptions::builder()
        .mode(ValidationMode::Open)  // Allow extra keys
        .max_depth(50)
        .log_level(LogLevel::Debug)
        .rich_errors(true)
        .include_source_location(true)
        .allow_type_coercion(true)
        .build();

    let result = validate_yaml_from_schema(yaml, schema, options);

    println!("{}", result.format_summary());

    if result.has_warnings() {
        println!("\nWarnings:");
        for warning in &result.warnings {
            println!("  - {}: {}", warning.instance_path, warning.message);
        }
    }
}
```

- [ ] **Step 3: Create error handling example**

Create `crates/graphql-ish-schema-validator/examples/error_handling.rs`:

```rust
//! Error handling example
//!
//! Run with: cargo run --example error_handling

use graphql_ish_schema_validator::{
    validate_yaml_from_schema,
    ValidationOptions,
    ValidationErrorCode,
};

fn main() {
    let schema = r#"
        input User @closed {
            id: String!
            email: String!
            role: UserRole
        }

        enum UserRole {
            admin
            user
            guest
        }
    "#;

    // Test 1: Valid document
    let valid_yaml = r#"
        id: "123"
        email: "user@example.com"
        role: admin
    "#;

    println!("Test 1: Valid document");
    let result = validate_yaml_from_schema(valid_yaml, schema, ValidationOptions::default());
    println!("  Result: {}\n", result.format_summary());

    // Test 2: Missing required field
    let missing_field_yaml = r#"
        id: "123"
        role: admin
    "#;

    println!("Test 2: Missing required field");
    let result = validate_yaml_from_schema(missing_field_yaml, schema, ValidationOptions::default());
    println!("  Result: {}", result.format_summary());
    if let Some(error) = result.errors.first() {
        println!("  Expected: {}", error.code == ValidationErrorCode::RequiredPropertyMissing);
    }
    println!();

    // Test 3: Invalid enum value
    let invalid_enum_yaml = r#"
        id: "123"
        email: "user@example.com"
        role: superuser
    "#;

    println!("Test 3: Invalid enum value");
    let result = validate_yaml_from_schema(invalid_enum_yaml, schema, ValidationOptions::default());
    println!("  Result: {}", result.format_summary());
    if let Some(error) = result.errors.first() {
        println!("  Expected: {}", error.code == ValidationErrorCode::InvalidEnumValue);
        if let Some(hint) = &error.hint {
            println!("  Hint: {}", hint);
        }
    }
    println!();

    // Test 4: Malformed YAML
    let malformed_yaml = r#"
        id: "123"
        email: "user@example.com
        role: admin
    "#;  // Missing closing quote

    println!("Test 4: Malformed YAML");
    let result = validate_yaml_from_schema(malformed_yaml, schema, ValidationOptions::default());
    println!("  Result: {}", result.format_summary());
    if let Some(error) = result.errors.first() {
        println!("  Expected: {}", error.code == ValidationErrorCode::ParseError);
    }
}
```

- [ ] **Step 4: Test examples compile and run**

Run: `cargo build --examples`
Expected: All examples compile

Run: `cargo run --example basic_validation`
Expected: Example runs successfully

Run: `cargo run --example error_handling`
Expected: Example runs successfully

- [ ] **Step 5: Commit examples**

```bash
git add crates/graphql-ish-schema-validator/examples/
git commit -m "docs: add public API examples for validation and error handling"
```

---

## Verification

**Step 1: Full workspace test**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 2: Build all examples**

Run: `cargo build --examples`
Expected: All examples build successfully

- [ ] **Step 3: Run basic validation example**

Run: `cargo run --example basic_validation`
Expected: Example completes without errors

- [ ] **Step 4: Run error handling example**

Run: `cargo run --example error_handling`
Expected: Example shows different error cases

- [ ] **Step 5: Verify public API documentation**

Run: `cargo doc --no-deps --open`
Expected: Documentation builds and displays in browser

- [ ] **Step 6: Final verification checklist**

Verify:
- ✅ `validate_yaml_from_schema()` function documented with examples
- ✅ `validate_json_from_schema()` function documented with examples
- ✅ `ValidationOptions` struct with builder pattern
- ✅ `ValidationResult` with rich error reporting
- ✅ `ValidationError` with context and convenience methods
- ✅ Panic resilience with `catch_unwind` wrapper
- ✅ All error codes defined and parseable
- ✅ Source location tracking in errors
- ✅ Examples compile and run successfully
- ✅ Documentation builds without warnings

- [ ] **Step 7: Final commit**

```bash
git add .
git commit -m "feat: complete public API design with examples and documentation"
```

---

## Summary

This plan implements a robust, user-friendly public API for the GraphQL-ish schema validator:

**Key achievements:**
1. **Enhanced ValidationOptions** with builder pattern and comprehensive configuration
2. **Rich ValidationResult** with detailed metrics, formatting, and JSON export
3. **Comprehensive ValidationError** with context, convenience methods, and source location
4. **Panic-resilient validation** using `catch_unwind` to prevent crashes
5. **Documented public API** with clear examples for common use cases

**Error resilience:**
- Parse errors are caught and returned as `ValidationResult`
- Validation panics are caught and converted to errors
- Malformed schemas and documents are handled gracefully
- Clear error messages with hints for remediation

**Developer experience:**
- Simple entry points for common cases
- Builder pattern for advanced configuration
- Detailed error reporting with context
- Example code for common scenarios

**Next steps:**
- Implement AST → IR lowering (see [03-compiler-lowering.md](../01-initial-attempt/03-compiler-lowering.md))
- Implement full validation engine (see [04-validator-runtime.md](../01-initial-attempt/04-validator-runtime.md))
- Add logging integration (see [02-logging-and-diagnostics.md](./02-logging-and-diagnostics.md))
- Implement CLI subcommands (see [03-cli-improvements.md](./03-cli-improvements.md))
