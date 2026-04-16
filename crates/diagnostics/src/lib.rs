//! gqlsdl-diagnostics: Error types and diagnostic reporting
//!
//! Provides structured error types with JSON Pointer paths for both
//! instance (document) and schema locations, plus miette integration
//! for rich terminal output.

use serde::{Deserialize, Serialize};

/// A single validation error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationError {
    /// JSON Pointer path to the rejected value in the instance document.
    pub instance_path: String,
    /// JSON Pointer path to the rejecting schema node.
    pub schema_path: String,
    /// Machine-readable error code.
    pub code: ErrorCode,
    /// Human-readable error message.
    pub message: String,
    /// Optional hint for fixing the error.
    pub hint: Option<String>,
}

/// Machine-readable validation error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCode {
    // Type mismatches
    TypeMismatch,
    InvalidScalar,

    // Object errors
    RequiredPropertyMissing,
    UnknownProperty,
    DuplicateKey,

    // Array errors
    InvalidElement,

    // Enum errors
    InvalidEnumValue,

    // Union errors
    NoMatchingVariant,
    AmbiguousVariant,
    InvalidDiscriminatorValue,

    // Reference errors
    UnresolvedRef,
    MaxDepthExceeded,

    // Scalar constraint errors
    PatternMismatch,
    ValueOutOfRange,

    // Parse errors
    InvalidSDL,
    InvalidYAML,
    InvalidJSON,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ErrorCode::TypeMismatch => "type_mismatch",
            ErrorCode::InvalidScalar => "invalid_scalar",
            ErrorCode::RequiredPropertyMissing => "required_property_missing",
            ErrorCode::UnknownProperty => "unknown_property",
            ErrorCode::DuplicateKey => "duplicate_key",
            ErrorCode::InvalidElement => "invalid_element",
            ErrorCode::InvalidEnumValue => "invalid_enum_value",
            ErrorCode::NoMatchingVariant => "no_matching_variant",
            ErrorCode::AmbiguousVariant => "ambiguous_variant",
            ErrorCode::InvalidDiscriminatorValue => "invalid_discriminator_value",
            ErrorCode::UnresolvedRef => "unresolved_ref",
            ErrorCode::MaxDepthExceeded => "max_depth_exceeded",
            ErrorCode::PatternMismatch => "pattern_mismatch",
            ErrorCode::ValueOutOfRange => "value_out_of_range",
            ErrorCode::InvalidSDL => "invalid_sdl",
            ErrorCode::InvalidYAML => "invalid_yaml",
            ErrorCode::InvalidJSON => "invalid_json",
        };
        write!(f, "{s}")
    }
}

/// SDL parsing error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SdlError {
    #[error("parse error at line {line}, column {col}: {message}")]
    ParseError {
        line: usize,
        col: usize,
        message: String,
    },
    #[error("unknown type '{name}' referenced")]
    UnknownType { name: String },
    #[error("duplicate type definition '{name}'")]
    DuplicateType { name: String },
    #[error("invalid directive '@{name}' on {target}: {reason}")]
    InvalidDirective {
        name: String,
        target: String,
        reason: String,
    },
    #[error("cycle detected: {path}")]
    CycleDetected { path: String },
}

/// Compiler lowering error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum LoweringError {
    #[error("unresolved type reference '{name}'")]
    UnresolvedRef { name: String },
    #[error("invalid directive '{directive}' on {target}: {reason}")]
    InvalidDirective {
        directive: String,
        target: String,
        reason: String,
    },
    #[error("recursive cycle: {path}")]
    RecursiveCycle { path: String },
    #[error("unsupported SDL construct: {detail}")]
    UnsupportedConstruct { detail: String },
    #[error("conflicting directives on {target}: {detail}")]
    ConflictingDirectives { target: String, detail: String },
}

/// Source location for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl SourceSpan {
    pub fn new(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    pub fn point(line: usize, col: usize) -> Self {
        Self {
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
        }
    }
}

/// Validation result with structured errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn from_errors(errors: Vec<ValidationError>) -> Self {
        let valid = errors.is_empty();
        Self { valid, errors }
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.valid = false;
        self.errors.push(error);
    }
}

/// Validation mode: strict vs open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValidationMode {
    /// Reject unknown fields, duplicate keys, type mismatches.
    #[default]
    Strict,
    /// Ignore unknown fields, coerce types where safe.
    Open,
}
