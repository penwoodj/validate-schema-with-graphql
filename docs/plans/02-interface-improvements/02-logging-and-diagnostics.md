# Logging and Diagnostics Plan

**For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement structured logging with `tracing` crate, rich error formatting with `miette`, and multiple output formats for validation results.

**Architecture:** Three-layer logging system (tracing → structured logs → output formats) with integration between `tracing` for debug logs and `miette` for rich error presentation.

**Tech Stack:** Rust, tracing, tracing-subscriber, miette, ansi-term, serde_json

---

## Context and Rationale

**Logging requirements from research:**
- Use `tracing` crate for structured logging
- Log levels: trace (CST walking details), debug (IR compilation steps), info (validation summary), warn (soft failures), error (hard failures)
- Log output configurable: stderr (default), file, stdout, or silent
- Integration with miette for rich error rendering
- JSON output mode for CI/machine consumption
- GitHub Actions annotation format
- Performance timing logged at info level

**Key design principles:**
1. **Structured**: All logs include relevant context (schema name, paths, etc.)
2. **Configurable**: Multiple output destinations and levels
3. **Rich**: Terminal-friendly with colors, machine-friendly JSON when needed
4. **Performant**: Minimal overhead, conditional logging

**References:**
- [01-public-api.md](./01-public-api.md) - ValidationOptions with log configuration
- [05-error-reporting.md](../01-initial-attempt/05-error-reporting.md) - Error reporting design

---

## Task 1: Create Logging Infrastructure

**Files:**
- Create: `crates/graphql-ish-schema-validator-validator/src/logging.rs`
- Modify: `crates/graphql-ish-schema-validator-validator/Cargo.toml`

**Step 1: Add logging dependencies**

Update `crates/graphql-ish-schema-validator-validator/Cargo.toml`:

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
tracing-subscriber = { workspace = true }
tracing-appender = "0.2"
once_cell = "1.18"

[dev-dependencies]
insta = "1.34"
```

- [ ] **Step 2: Create logging module**

Create `crates/graphql-ish-schema-validator-validator/src/logging.rs`:

```rust
//! Logging infrastructure for the validator
//!
//! Provides structured logging using the `tracing` crate with configurable
//! output destinations (stderr, stdout, file, silent).

use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::{Level, Subscriber};
use tracing_appender::{non_blocking, WorkerGuard};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use super::options::{LogLevel, LogOutput};

/// Global worker guard for non-blocking logging
static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// Initialize logging for the validator
///
/// This should be called once at program startup (or before any validation).
pub fn init_logging(level: LogLevel, output: LogOutput) -> Result<(), String> {
    let filter_level = match level {
        LogLevel::Trace => Level::TRACE,
        LogLevel::Debug => Level::DEBUG,
        LogLevel::Info => Level::INFO,
        LogLevel::Warn => Level::WARN,
        LogLevel::Error => Level::ERROR,
        LogLevel::Silent => return Ok(()), // No logging
    };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(filter_level));

    match output {
        LogOutput::Stderr => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().with_target(false).with_span_events(FmtSpan::CLOSE))
                .try_init()
                .map_err(|e| format!("Failed to init logging: {}", e))?;
        }
        LogOutput::Stdout => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().with_writer(std::io::stdout).with_target(false))
                .try_init()
                .map_err(|e| format!("Failed to init logging: {}", e))?;
        }
        LogOutput::File(path) => {
            let file_appender = tracing_appender::rolling::daily(
                path.parent().unwrap_or_else(|| PathBuf::from(".")),
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("validator.log"),
            );
            let (non_blocking, guard) = non_blocking(file_appender);
            LOG_GUARD.set(guard).expect("Failed to set log guard");

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().with_writer(non_blocking).with_target(true))
                .try_init()
                .map_err(|e| format!("Failed to init logging: {}", e))?;
        }
        LogOutput::Silent => {
            // No logging configured
        }
    }

    Ok(())
}

/// Check if logging is enabled for a given level
pub fn is_level_enabled(level: LogLevel) -> bool {
    use tracing::Level;

    let tracing_level = match level {
        LogLevel::Trace => Level::TRACE,
        LogLevel::Debug => Level::DEBUG,
        LogLevel::Info => Level::INFO,
        LogLevel::Warn => Level::WARN,
        LogLevel::Error => Level::ERROR,
        LogLevel::Silent => return false,
    };

    tracing::level_enabled!(tracing_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging_stderr() {
        let result = init_logging(LogLevel::Info, LogOutput::Stderr);
        // First call should succeed
        assert!(result.is_ok() || result.unwrap_err().contains("already set"));
    }

    #[test]
    fn test_init_logging_silent() {
        let result = init_logging(LogLevel::Silent, LogOutput::Stderr);
        assert!(result.is_ok());
    }
}
```

- [ ] **Step 3: Update validator lib to export logging**

Update `crates/graphql-ish-schema-validator-validator/src/lib.rs`:

```rust
//! Validation engine for GraphQL-ish schema validator
//!
//! This crate provides the runtime validation logic that validates
//! YAML/JSON documents against compiled IR schemas.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod logging;
pub mod options;
pub mod result;
pub mod validator;

pub use error::{ValidationError, ValidationErrorCode};
pub use logging::{init_logging, is_level_enabled};
pub use options::ValidationOptions;
pub use result::ValidationResult;
pub use validator::Validator;
```

- [ ] **Step 4: Test logging module compiles**

Run: `cargo check -p graphql-ish-schema-validator-validator`
Expected: No compilation errors

- [ ] **Step 5: Test logging initialization**

Run: `cargo test -p graphql-ish-schema-validator-validator logging`
Expected: All logging tests pass

- [ ] **Step 6: Commit logging infrastructure**

```bash
git add crates/graphql-ish-schema-validator-validator/src/logging.rs
git add crates/graphql-ish-schema-validator-validator/src/lib.rs
git add crates/graphql-ish-schema-validator-validator/Cargo.toml
git commit -m "feat: add logging infrastructure with tracing crate"
```

---

## Task 2: Add Tracing to Validator

**Files:**
- Modify: `crates/graphql-ish-schema-validator-validator/src/validator.rs`

**Step 1: Add tracing instrumentation**

Update the validator module to add tracing spans:

```rust
//! Core validation engine with panic resilience and tracing

use graphql_ish_schema_validator_ir::CompiledSchema;
use serde_json::Value;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;
use tracing::{debug, error, info, instrument, trace, warn, Span};

use super::options::{ValidationMode, ValidationOptions};
use super::result::{DocumentType, InstancePath, ValidationResult};
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
    #[instrument(skip(self, json), fields(
        schema_id = %self.schema.schema_id.as_deref().unwrap_or("unknown"),
        schema_version = %self.schema.schema_version.as_deref().unwrap_or("none"),
        mode = %self.options.mode,
        max_depth = %self.options.max_depth,
    ))]
    pub fn validate_json(&self, json: &str) -> ValidationResult {
        let start = Instant::now();
        info!("Starting JSON validation");
        trace!("Input JSON length: {} bytes", json.len());

        // Catch panics during parsing
        let parse_result = catch_unwind(AssertUnwindSafe(|| {
            serde_json::from_str::<Value>(json)
        }));

        let value = match parse_result {
            Ok(Ok(v)) => {
                debug!("JSON parsed successfully");
                v
            }
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
        let validation_start = Instant::now();
        let validation_result = catch_unwind(AssertUnwindSafe(|| {
            let mut path = InstancePath::new();
            let mut result = ValidationResult::success(
                self.schema.schema_id.clone(),
                DocumentType::Json,
                std::time::Duration::ZERO,
            );

            // TODO: Implement actual validation
            self.validate_node(&value, &self.schema.definitions, &mut path, &mut result);

            result
        }));

        match validation_result {
            Ok(mut result) => {
                let validation_duration = validation_start.elapsed();
                let total_duration = start.elapsed();

                result.duration = total_duration;

                if result.valid {
                    info!(
                        "JSON validation successful in {}ms (validation: {}ms)",
                        total_duration.as_millis(),
                        validation_duration.as_millis()
                    );
                } else {
                    warn!(
                        "JSON validation failed in {}ms: {} error(s), {} warning(s)",
                        total_duration.as_millis(),
                        result.errors.len(),
                        result.warnings.len()
                    );
                }

                result
            }
            Err(_) => {
                let duration = start.elapsed();
                error!("Panic during JSON validation after {}ms", duration.as_millis());
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
    #[instrument(skip(self, yaml), fields(
        schema_id = %self.schema.schema_id.as_deref().unwrap_or("unknown"),
        schema_version = %self.schema.schema_version.as_deref().unwrap_or("none"),
        mode = %self.options.mode,
        max_depth = %self.options.max_depth,
    ))]
    pub fn validate_yaml(&self, yaml: &str) -> ValidationResult {
        let start = Instant::now();
        info!("Starting YAML validation");
        trace!("Input YAML length: {} bytes", yaml.len());

        // Catch panics during parsing
        let parse_result = catch_unwind(AssertUnwindSafe(|| {
            serde_yaml_ng::from_str::<Value>(yaml)
        }));

        let value = match parse_result {
            Ok(Ok(v)) => {
                debug!("YAML parsed successfully");
                v
            }
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
        let validation_start = Instant::now();
        let validation_result = catch_unwind(AssertUnwindSafe(|| {
            let mut path = InstancePath::new();
            let mut result = ValidationResult::success(
                self.schema.schema_id.clone(),
                DocumentType::Yaml,
                std::time::Duration::ZERO,
            );

            // TODO: Implement actual validation
            self.validate_node(&value, &self.schema.definitions, &mut path, &mut result);

            result
        }));

        match validation_result {
            Ok(mut result) => {
                let validation_duration = validation_start.elapsed();
                let total_duration = start.elapsed();

                result.duration = total_duration;

                if result.valid {
                    info!(
                        "YAML validation successful in {}ms (validation: {}ms)",
                        total_duration.as_millis(),
                        validation_duration.as_millis()
                    );
                } else {
                    warn!(
                        "YAML validation failed in {}ms: {} error(s), {} warning(s)",
                        total_duration.as_millis(),
                        result.errors.len(),
                        result.warnings.len()
                    );
                }

                result
            }
            Err(_) => {
                let duration = start.elapsed();
                error!("Panic during YAML validation after {}ms", duration.as_millis());
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

    /// Core node validation logic (stub - will be implemented in later tasks)
    #[instrument(skip(value, definitions, result), fields(
        path = %path.as_human_readable(),
        depth = %path.depth(),
        value_type = %value_type_name(value),
    ))]
    fn validate_node(
        &self,
        value: &Value,
        definitions: &HashMap<String, graphql_ish_schema_validator_ir::SchemaForm>,
        path: &mut InstancePath,
        result: &mut ValidationResult,
    ) {
        trace!("Validating node at {}", path);

        // TODO: Implement actual validation logic
        // This will be filled in by the validator runtime plan
        let node_count = result.node_count;
        result.set_node_count(node_count + 1);

        let max_depth = result.max_depth_encountered;
        result.set_max_depth(max_depth.max(path.depth()));
    }
}

/// Get a human-readable name for a Value type
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// Extension traits
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_schema() -> CompiledSchema {
        CompiledSchema {
            schema_id: Some("test".to_string()),
            schema_version: None,
            definitions: HashMap::new(),
        }
    }

    #[test]
    fn test_validate_json_with_tracing() {
        let schema = create_test_schema();
        let validator = Validator::new(schema, ValidationOptions::default());

        let result = validator.validate_json(r#"{"name": "test"}"#);

        assert!(result.valid);
    }
}
```

- [ ] **Step 2: Test validator with tracing**

Run: `cargo test -p graphql-ish-schema-validator-validator validator`
Expected: All validator tests pass

- [ ] **Step 3: Commit validator tracing**

```bash
git add crates/graphql-ish-schema-validator-validator/src/validator.rs
git commit -m "feat: add tracing instrumentation to validator"
```

---

## Task 3: Add Rich Error Formatting with Miette

**Files:**
- Modify: `crates/graphql-ish-schema-validator-validator/Cargo.toml`
- Create: `crates/graphql-ish-schema-validator-validator/src/formatter.rs`

**Step 1: Add miette dependency**

Update `crates/graphql-ish-schema-validator-validator/Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...
miette = { version = "5.10", features = ["fancy"] }
```

- [ ] **Step 2: Create formatter module**

Create `crates/graphql-ish-schema-validator-validator/src/formatter.rs`:

```rust
//! Error formatting with rich diagnostics
//!
//! Provides terminal-friendly and machine-readable output formats for
//! validation results.

use std::fmt;
use ansi_term::{Colour, Style};

use super::error::{ValidationError, ValidationErrorCode};
use super::result::ValidationResult;

/// Output format for validation results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text with colors
    Text,

    /// Machine-readable JSON
    Json,

    /// GitHub Actions workflow command format
    GitHubActions,
}

impl OutputFormat {
    /// Create from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" => Some(OutputFormat::Text),
            "json" => Some(OutputFormat::Json),
            "github-actions" | "github" | "gh" => Some(OutputFormat::GitHubActions),
            _ => None,
        }
    }
}

/// Format validation result for display
pub struct Formatter {
    format: OutputFormat,
    colorize: bool,
}

impl Formatter {
    /// Create a new formatter with the specified format
    pub fn new(format: OutputFormat) -> Self {
        Self {
            format,
            colorize: true,
        }
    }

    /// Disable colored output
    pub fn no_color(mut self) -> Self {
        self.colorize = false;
        self
    }

    /// Enable colored output
    pub fn with_color(mut self) -> Self {
        self.colorize = true;
        self
    }

    /// Format a validation result as a string
    pub fn format_result(&self, result: &ValidationResult) -> String {
        match self.format {
            OutputFormat::Text => self.format_text(result),
            OutputFormat::Json => self.format_json(result),
            OutputFormat::GitHubActions => self.format_github_actions(result),
        }
    }

    /// Format as human-readable text
    fn format_text(&self, result: &ValidationResult) -> String {
        let mut output = String::new();

        // Summary line
        if result.valid {
            let summary = if self.colorize {
                Colour::Green.paint(result.format_summary()).to_string()
            } else {
                result.format_summary()
            };
            output.push_str(&summary);
        } else {
            let summary = if self.colorize {
                Colour::Red.paint(result.format_summary()).to_string()
            } else {
                result.format_summary()
            };
            output.push_str(&summary);
        }

        // Errors
        if !result.errors.is_empty() {
            output.push_str("\n\n");
            let header = if self.colorize {
                Style::new().bold().paint(format!("Errors ({}):", result.errors.len()))
            } else {
                format!("Errors ({}):", result.errors.len())
            };
            output.push_str(&header.to_string());
            output.push_str(&self.format_errors_text(&result.errors));
        }

        // Warnings
        if !result.warnings.is_empty() {
            output.push_str("\n\n");
            let header = if self.colorize {
                Colour::Yellow.bold().paint(format!("Warnings ({}):", result.warnings.len()))
            } else {
                format!("Warnings ({}):", result.warnings.len())
            };
            output.push_str(&header.to_string());
            output.push_str(&self.format_warnings_text(&result.warnings));
        }

        output
    }

    /// Format errors as text
    fn format_errors_text(&self, errors: &[ValidationError]) -> String {
        let mut output = String::new();

        for (i, error) in errors.iter().enumerate() {
            output.push_str(&format!("\n  {}. {} ", i + 1, error.code));

            if self.colorize {
                output.push_str(&Colour::Red.paint(&error.instance_path).to_string());
            } else {
                output.push_str(&error.instance_path);
            }

            output.push_str("\n");
            output.push_str(&format!("     {}\n", error.message));

            if let Some(hint) = &error.hint {
                let hint_line = if self.colorize {
                    format!("     Hint: {}", Colour::Cyan.paint(hint).to_string())
                } else {
                    format!("     Hint: {}", hint)
                };
                output.push_str(&hint_line);
                output.push('\n');
            }

            if let Some(location) = &error.source_location {
                output.push_str(&format!("     Location: {}\n", location));
            }
        }

        output
    }

    /// Format warnings as text
    fn format_warnings_text(&self, warnings: &[ValidationError]) -> String {
        let mut output = String::new();

        for (i, warning) in warnings.iter().enumerate() {
            output.push_str(&format!("\n  {}. {} ", i + 1, warning.code));

            if self.colorize {
                output.push_str(&Colour::Yellow.paint(&warning.instance_path).to_string());
            } else {
                output.push_str(&warning.instance_path);
            }

            output.push_str("\n");
            output.push_str(&format!("     {}\n", warning.message));

            if let Some(hint) = &warning.hint {
                let hint_line = if self.colorize {
                    format!("     Hint: {}", Colour::Cyan.paint(hint).to_string())
                } else {
                    format!("     Hint: {}", hint)
                };
                output.push_str(&hint_line);
                output.push('\n');
            }
        }

        output
    }

    /// Format as JSON
    fn format_json(&self, result: &ValidationResult) -> String {
        match result.to_json_indent(2) {
            Ok(json) => json,
            Err(e) => {
                format!("{{\"error\": \"Failed to serialize result: {}\"}}", e)
            }
        }
    }

    /// Format as GitHub Actions workflow commands
    fn format_github_actions(&self, result: &ValidationResult) -> String {
        let mut output = String::new();

        for error in &result.errors {
            let level = match error.severity {
                super::error::ErrorSeverity::Error => "error",
                super::error::ErrorSeverity::Warning => "warning",
            };

            output.push_str(&format!(
                "::{} file=unknown,line={},title={}::{} at {}\n",
                level,
                error.source_location
                    .map(|l| l.line.to_string())
                    .unwrap_or_else(|| "0".to_string()),
                error.code,
                error.message,
                error.instance_path
            ));
        }

        for warning in &result.warnings {
            output.push_str(&format!(
                "::warning file=unknown,title={}::{} at {}\n",
                warning.code,
                warning.message,
                warning.instance_path
            ));
        }

        output
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::GitHubActions => write!(f, "github-actions"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::result::DocumentType;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("text"), Some(OutputFormat::Text));
        assert_eq!(OutputFormat::from_str("TEXT"), Some(OutputFormat::Text));
        assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("github-actions"), Some(OutputFormat::GitHubActions));
        assert_eq!(OutputFormat::from_str("gh"), Some(OutputFormat::GitHubActions));
        assert_eq!(OutputFormat::from_str("unknown"), None);
    }

    #[test]
    fn test_format_valid_result() {
        let result = ValidationResult::success(
            Some("test".to_string()),
            DocumentType::Yaml,
            std::time::Duration::from_millis(10),
        );

        let formatter = Formatter::new(OutputFormat::Text).no_color();
        let output = formatter.format_result(&result);

        assert!(output.contains("Valid"));
        assert!(output.contains("10ms"));
    }

    #[test]
    fn test_format_result_with_errors() {
        let error = ValidationError::new(
            ValidationErrorCode::TypeMismatch,
            "/test/path".to_string(),
            "/schema/path".to_string(),
            "Test error".to_string(),
        );

        let result = ValidationResult::failure(
            vec![error],
            None,
            DocumentType::Json,
            std::time::Duration::from_millis(5),
        );

        let formatter = Formatter::new(OutputFormat::Text).no_color();
        let output = formatter.format_result(&result);

        assert!(output.contains("Invalid"));
        assert!(output.contains("type_mismatch"));
        assert!(output.contains("/test/path"));
        assert!(output.contains("Test error"));
    }

    #[test]
    fn test_format_as_json() {
        let result = ValidationResult::success(
            Some("test".to_string()),
            DocumentType::Yaml,
            std::time::Duration::from_millis(10),
        );

        let formatter = Formatter::new(OutputFormat::Json);
        let json = formatter.format_result(&result);

        assert!(json.contains("\"valid\":true"));
        assert!(json.contains("\"schema_name\":\"test\""));
    }
}
```

- [ ] **Step 3: Update validator lib to export formatter**

Update `crates/graphql-ish-schema-validator-validator/src/lib.rs`:

```rust
pub mod formatter;
pub use formatter::{Formatter, OutputFormat};
```

- [ ] **Step 4: Add ansi-term dependency**

Update workspace `Cargo.toml`:

```toml
[workspace.dependencies]
# ... existing dependencies ...
ansi-term = "0.12"
```

- [ ] **Step 5: Test formatter compiles**

Run: `cargo check -p graphql-ish-schema-validator-validator`
Expected: No compilation errors

- [ ] **Step 6: Run formatter tests**

Run: `cargo test -p graphql-ish-schema-validator-validator formatter`
Expected: All formatter tests pass

- [ ] **Step 7: Commit formatter module**

```bash
git add crates/graphql-ish-schema-validator-validator/src/formatter.rs
git add crates/graphql-ish-schema-validator-validator/src/lib.rs
git add Cargo.toml
git commit -m "feat: add error formatting with multiple output formats"
```

---

## Task 4: Add Performance Logging

**Files:**
- Modify: `crates/graphql-ish-schema-validator-validator/src/validator.rs`

**Step 1: Add performance instrumentation**

Update validator to log performance metrics:

```rust
// In the validate_json method, after successful validation:
if result.valid {
    let node_rate = if !duration.is_zero() {
        (result.node_count as f64) / (duration.as_secs_f64() * 1000.0) // nodes per ms
    } else {
        0.0
    };

    info!(
        "JSON validation successful in {}ms: {} nodes validated, depth {}/{}, rate: {:.2} nodes/ms",
        total_duration.as_millis(),
        result.node_count,
        result.max_depth_encountered,
        self.options.max_depth,
        node_rate
    );
}
```

- [ ] **Step 2: Add similar logging to validate_yaml**

- [ ] **Step 3: Test performance logging**

Run: `RUST_LOG=debug cargo run --example basic_validation`
Expected: Performance metrics logged at debug level

- [ ] **Step 4: Commit performance logging**

```bash
git add crates/graphql-ish-schema-validator-validator/src/validator.rs
git commit -m "feat: add performance metrics logging"
```

---

## Task 5: Add CLI Output Integration

**Files:**
- Modify: `crates/graphql-ish-schema-validator-cli/src/main.rs`

**Step 1: Add formatting options to CLI**

Update CLI to support different output formats:

```rust
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

        /// Disable colored output
        #[arg(long)]
        no_color: bool,
    },
    // ... other commands
}

// In the validate_command function:
fn validate_command(
    input: &str,
    schema: &str,
    strict: bool,
    open: bool,
    format: &str,
    no_color: bool,
) -> Result<()> {
    let output_format = OutputFormat::from_str(format)
        .ok_or_else(|| anyhow::anyhow!("Invalid output format: {}", format))?;

    // ... validation logic ...

    // Format and print result
    let formatter = Formatter::new(output_format)
        .with_color(!no_color);

    println!("{}", formatter.format_result(&result));

    // Exit with appropriate code
    if result.valid {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}
```

- [ ] **Step 2: Test CLI output formats**

Run: `cargo run -p graphql-ish-schema-validator-cli -- validate test.yml --schema schema.graphql --format text`
Expected: Human-readable output

Run: `cargo run -p graphql-ish-schema-validator-cli -- validate test.yml --schema schema.graphql --format json`
Expected: JSON output

- [ ] **Step 3: Commit CLI formatting integration**

```bash
git add crates/graphql-ish-schema-validator-cli/src/main.rs
git commit -m "feat: add CLI output format options"
```

---

## Task 6: Create Logging Examples

**Files:**
- Create: `crates/graphql-ish-schema-validator/examples/logging_example.rs`
- Create: `crates/graphql-ish-schema-validator/examples/output_formats.rs`

**Step 1: Create logging example**

Create `crates/graphql-ish-schema-validator/examples/logging_example.rs`:

```rust
//! Logging example showing different log levels
//!
//! Run with: RUST_LOG=trace cargo run --example logging_example

use graphql_ish_schema_validator::{
    init_logging,
    validate_yaml_from_schema,
    ValidationOptions,
    LogLevel,
};

fn main() {
    // Initialize logging
    init_logging(LogLevel::Trace, graphql_ish_schema_validator::LogOutput::Stderr)
        .expect("Failed to init logging");

    let schema = r#"
        input Config @closed {
            name: String!
            debug: Boolean
        }
    "#;

    let yaml = r#"
        name: Test
        debug: true
    "#;

    let options = ValidationOptions::builder()
        .log_level(LogLevel::Trace)
        .build();

    let result = validate_yaml_from_schema(yaml, schema, options);

    println!("\n{}", result.format_summary());
}
```

- [ ] **Step 2: Create output formats example**

Create `crates/graphql-ish-schema-validator/examples/output_formats.rs`:

```rust
//! Output format example
//!
//! Run with: cargo run --example output_formats

use graphql_ish_schema_validator::Formatter;

fn main() {
    use graphql_ish_schema_validator::{ValidationError, ValidationErrorCode, ValidationResult, DocumentType};

    // Create a sample error
    let error = ValidationError::type_mismatch(
        "/agentic_workflow/steps/0/prompt",
        "/definitions/AgentStep/properties/prompt",
        "string",
        "null",
    )
    .with_hint("Prompt field is required for AgentStep")
    .with_location(42, 8);

    let result = ValidationResult::failure(
        vec![error],
        Some("workflow".to_string()),
        DocumentType::Yaml,
        std::time::Duration::from_millis(15),
    );

    // Text format with colors
    println!("=== Text Format (colored) ===");
    let formatter = Formatter::new(graphql_ish_schema_validator::OutputFormat::Text);
    println!("{}", formatter.format_result(&result));

    // Text format without colors
    println!("\n=== Text Format (no color) ===");
    let formatter = Formatter::new(graphql_ish_schema_validator::OutputFormat::Text)
        .no_color();
    println!("{}", formatter.format_result(&result));

    // JSON format
    println!("\n=== JSON Format ===");
    let formatter = Formatter::new(graphql_ish_schema_validator::OutputFormat::Json);
    println!("{}", formatter.format_result(&result));

    // GitHub Actions format
    println!("\n=== GitHub Actions Format ===");
    let formatter = Formatter::new(graphql_ish_schema_validator::OutputFormat::GitHubActions);
    println!("{}", formatter.format_result(&result));
}
```

- [ ] **Step 3: Test examples**

Run: `RUST_LOG=debug cargo run --example logging_example`
Expected: Detailed logging output

Run: `cargo run --example output_formats`
Expected: All output formats displayed

- [ ] **Step 4: Commit logging examples**

```bash
git add crates/graphql-ish-schema-validator/examples/
git commit -m "docs: add logging and output format examples"
```

---

## Verification

**Step 1: Full workspace test**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 2: Test logging at different levels**

Run: `RUST_LOG=trace cargo test --workspace`
Expected: Verbose tracing output

Run: `RUST_LOG=error cargo test --workspace`
Expected: Only errors logged

- [ ] **Step 3: Test formatter output**

Run: `cargo run --example output_formats`
Expected: All formats render correctly

- [ ] **Step 4: Test CLI with different formats**

Run: `cargo run -p graphql-ish-schema-validator-cli -- validate --help`
Expected: Format options shown

- [ ] **Step 5: Verify GitHub Actions format**

Test that GH Actions format produces valid workflow commands:
```bash
echo "::error file=test.yml,line=10,title=type_mismatch::Test error at /path"
```

- [ ] **Step 6: Final verification checklist**

Verify:
- ✅ `tracing` crate integrated throughout validation pipeline
- ✅ Log levels: trace, debug, info, warn, error, silent
- ✅ Output destinations: stderr, stdout, file, silent
- ✅ `Formatter` with Text, JSON, GitHub Actions formats
- ✅ Colored output configurable via --no-color flag
- ✅ Performance metrics logged at info level
- ✅ Panic resilience still works with tracing
- ✅ Examples demonstrate logging and formatting
- ✅ CLI supports all output formats
- ✅ GitHub Actions format produces valid workflow commands

- [ ] **Step 7: Final commit**

```bash
git add .
git commit -m "feat: complete logging and diagnostics implementation"
```

---

## Summary

This plan implements a comprehensive logging and diagnostics system:

**Key achievements:**
1. **Tracing infrastructure** with configurable levels and destinations
2. **Rich error formatting** with terminal-friendly and machine-readable outputs
3. **Multiple output formats**: Text (colored), JSON, GitHub Actions
4. **Performance metrics** logged automatically
5. **CLI integration** for format selection
6. **Example code** demonstrating logging and formatting

**Logging features:**
- Structured logging with `tracing` spans
- Configurable output (stderr/stdout/file/silent)
- Performance metrics (time, node count, depth)
- Trace CST walking (not yet implemented)
- Debug IR compilation steps (not yet implemented)

**Error formatting features:**
- Human-readable text with colors
- Machine-readable JSON
- GitHub Actions workflow commands
- Source location tracking
- Hints and context in errors

**Next steps:**
- Implement CLI subcommands fully (see [03-cli-improvements.md](./03-cli-improvements.md))
- Add comprehensive tests (see [04-testing-and-verification.md](./04-testing-and-verification.md))
- Create migration guide (see [05-migration-checklist.md](./05-migration-checklist.md))
