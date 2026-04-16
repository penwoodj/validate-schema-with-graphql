pub use graphql_ish_schema_validator_diagnostics::{
    ErrorCode, LoweringError, SdlError, ValidationError, ValidationMode, ValidationResult,
};
pub use graphql_ish_schema_validator_ir::{
    AdditionalPolicy, JsonPointer, ScalarKind, Schema, SchemaBundle,
};

use graphql_ish_schema_validator_compiler as compiler;
use graphql_ish_schema_validator_parser as parser;
use graphql_ish_schema_validator_validator as validator;
use std::panic::{catch_unwind, AssertUnwindSafe};
use tracing::{debug, info, instrument, warn};

/// Options for validation.
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    pub mode: ValidationMode,
    pub root_schema: Option<String>,
    pub max_depth: usize,
    pub log_level: LogLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Silent,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            mode: ValidationMode::Strict,
            root_schema: None,
            max_depth: 64,
            log_level: LogLevel::Warn,
        }
    }
}

/// Enhanced validation result with metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnhancedValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
    pub schema_types_count: usize,
    pub root_schema: Option<String>,
}

impl EnhancedValidationResult {
    pub fn from_result(result: ValidationResult, types_count: usize, root: Option<String>) -> Self {
        Self {
            valid: result.valid,
            errors: result.errors,
            warnings: Vec::new(),
            schema_types_count: types_count,
            root_schema: root,
        }
    }
}

/// Validate a YAML document against a GraphQL SDL schema.
///
/// # Arguments
/// * `yaml` - YAML document as string
/// * `schema` - GraphQL SDL schema as string
/// * `options` - Validation options
///
/// # Returns
/// Enhanced validation result with errors, warnings, and metadata.
///
/// # Errors
/// Returns a `ValidationError` if the YAML cannot be parsed or the schema is invalid.
/// Never panics — all errors are caught and returned.
#[instrument(skip_all)]
pub fn validate_yaml_from_schema(
    yaml: &str,
    schema: &str,
    options: &ValidationOptions,
) -> Result<EnhancedValidationResult, String> {
    catch_unwind(AssertUnwindSafe(|| {
        validate_from_schema_impl(yaml, schema, options, InputFormat::Yaml)
    }))
    .map_err(|e| {
        let reason = if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else {
            format!("{:?}", e)
        };
        format!("Internal panic during validation: {}", reason)
    })?
}

/// Validate a JSON document against a GraphQL SDL schema.
#[instrument(skip_all)]
pub fn validate_json_from_schema(
    json: &str,
    schema: &str,
    options: &ValidationOptions,
) -> Result<EnhancedValidationResult, String> {
    catch_unwind(AssertUnwindSafe(|| {
        validate_from_schema_impl(json, schema, options, InputFormat::Json)
    }))
    .map_err(|e| {
        let reason = if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else {
            format!("{:?}", e)
        };
        format!("Internal panic during validation: {}", reason)
    })?
}

#[derive(Debug)]
enum InputFormat {
    Yaml,
    Json,
}

#[instrument(skip(input, schema), fields(format = ?format))]
fn validate_from_schema_impl(
    input: &str,
    schema: &str,
    options: &ValidationOptions,
    format: InputFormat,
) -> Result<EnhancedValidationResult, String> {
    catch_unwind(AssertUnwindSafe(|| {
        // Step 1: Parse SDL
        debug!("Parsing SDL schema ({} bytes)", schema.len());
        let ast = parser::extract_ast(schema).map_err(|errs| {
            format!(
                "SDL parse errors: {}",
                errs.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;

        // Step 2: Compile to IR
        let mut bundle = compiler::compile(&ast).map_err(|errs| {
            format!(
                "Compile errors: {}",
                errs.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;

        debug!("Compiled {} schema types", bundle.schemas.len());

        if let Some(ref root) = options.root_schema {
            bundle.set_root(root);
        }

        let types_count = bundle.schemas.len();
        let root_name = bundle.root_name.clone();

        // Step 3: Parse input document
        let value = match format {
            InputFormat::Yaml => {
                #[cfg(feature = "yaml")]
                {
                    validator::parse_yaml_with_mode(input, options.mode)
                        .map_err(|e| format!("YAML parse error: {e}"))?
                }
                #[cfg(not(feature = "yaml"))]
                {
                    return Err(
                        "YAML support not compiled in. Enable the 'yaml' feature.".to_string()
                    );
                }
            }
            InputFormat::Json => {
                validator::parse_json(input).map_err(|e| format!("JSON parse error: {e}"))?
            }
        };

        // Step 4: Validate
        let v = validator::Validator::new(&bundle)
            .with_mode(options.mode)
            .with_max_depth(options.max_depth);
        let result = v.validate(&value);

        info!(
            "Validation complete: valid={}, errors={}",
            result.valid,
            result.errors.len()
        );

        if !result.valid {
            for e in &result.errors {
                warn!("Validation error: {} at {}", e.message, e.instance_path);
            }
        }

        Ok(EnhancedValidationResult::from_result(
            result,
            types_count,
            root_name,
        ))
    }))
    .map_err(|e| {
        let reason = if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else {
            format!("{:?}", e)
        };
        format!("Internal panic during validation: {}", reason)
    })?
}
