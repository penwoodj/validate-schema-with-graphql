//! graphql-ish-schema-validator-validator: IR-based YAML/JSON document validation

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use validate_schema_with_graphql_diagnostics::{
    ErrorCode, ValidationError, ValidationMode, ValidationResult,
};
use validate_schema_with_graphql_ir::{
    AdditionalPolicy, JsonPointer, ScalarKind, Schema, SchemaBundle,
};

/// Canonical value model — source-agnostic representation of YAML/JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(IndexMap<String, Value>),
}

/// Number type distinguishing integer from float.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Number {
    Integer(i64),
    Float(f64),
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Number(Number::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Value::Number(Number::Float(f))
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Self::from).collect())
            }
            serde_json::Value::Object(map) => {
                Value::Object(map.into_iter().map(|(k, v)| (k, Self::from(v))).collect())
            }
        }
    }
}

/// Parse JSON string into canonical Value.
pub fn parse_json(input: &str) -> Result<Value, String> {
    let v: serde_json::Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    Ok(Value::from(v))
}

/// Parse YAML string into canonical Value.
/// Requires `yaml` feature (serde-saphyr).
#[cfg(feature = "yaml")]
pub fn parse_yaml(input: &str) -> Result<Value, String> {
    parse_yaml_with_mode(input, ValidationMode::Strict)
}

/// Parse YAML string with configurable duplicate key policy.
#[cfg(feature = "yaml")]
pub fn parse_yaml_with_mode(input: &str, mode: ValidationMode) -> Result<Value, String> {
    use validate_schema_with_graphql_diagnostics::ValidationMode;

    let opts = serde_saphyr::options! {
        duplicate_keys: match mode {
            ValidationMode::Strict => serde_saphyr::options::DuplicateKeyPolicy::Error,
            ValidationMode::Open => serde_saphyr::options::DuplicateKeyPolicy::LastWins,
        },
    };

    let json_val: serde_json::Value =
        serde_saphyr::from_str_with_options(input, opts).map_err(|e| e.to_string())?;
    Ok(Value::from(json_val))
}

/// Schema validator with recursive descent.
pub struct Validator<'a> {
    bundle: &'a SchemaBundle,
    mode: ValidationMode,
    max_depth: usize,
}

impl<'a> Validator<'a> {
    pub fn new(bundle: &'a SchemaBundle) -> Self {
        Self {
            bundle,
            mode: ValidationMode::Strict,
            max_depth: 64,
        }
    }

    pub fn with_mode(mut self, mode: ValidationMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Validate a value against the root schema in the bundle.
    pub fn validate(&self, value: &Value) -> ValidationResult {
        let mut ctx = ValidationContext {
            instance_path: JsonPointer::root(),
            schema_path: JsonPointer::root(),
            errors: Vec::new(),
            depth: 0,
        };

        if let Some(root) = self.bundle.root() {
            self.validate_schema(root, value, &mut ctx);
        } else {
            ctx.errors.push(ValidationError {
                instance_path: "/".to_string(),
                schema_path: "/".to_string(),
                code: ErrorCode::UnresolvedRef,
                message: "no root schema defined in bundle".to_string(),
                hint: None,
            });
        }

        ValidationResult::from_errors(ctx.errors)
    }

    /// Validate a value against a specific named schema.
    pub fn validate_named(&self, name: &str, value: &Value) -> ValidationResult {
        let mut ctx = ValidationContext {
            instance_path: JsonPointer::root(),
            schema_path: JsonPointer::root(),
            errors: Vec::new(),
            depth: 0,
        };

        if let Some(schema) = self.bundle.resolve(name) {
            self.validate_schema(schema, value, &mut ctx);
        } else {
            ctx.errors.push(ValidationError {
                instance_path: "/".to_string(),
                schema_path: "/".to_string(),
                code: ErrorCode::UnresolvedRef,
                message: format!("unknown schema: '{name}'"),
                hint: None,
            });
        }

        ValidationResult::from_errors(ctx.errors)
    }

    fn validate_schema(&self, schema: &Schema, value: &Value, ctx: &mut ValidationContext) {
        if ctx.depth > self.max_depth {
            ctx.errors.push(ValidationError {
                instance_path: ctx.instance_path.render(),
                schema_path: ctx.schema_path.render(),
                code: ErrorCode::MaxDepthExceeded,
                message: format!("max recursion depth ({}) exceeded", self.max_depth),
                hint: Some("check for deeply nested or recursive types".to_string()),
            });
            return;
        }

        match schema {
            Schema::Any => {}

            Schema::Scalar(kind) => self.validate_scalar(kind, value, ctx),

            Schema::Enum { values } => self.validate_enum(values, value, ctx),

            Schema::Array { elements } => self.validate_array(elements, value, ctx),

            Schema::Object {
                required,
                optional,
                additional,
            } => self.validate_object(required, optional, additional, value, ctx),

            Schema::Map { values: val_schema } => self.validate_map(val_schema, value, ctx),

            Schema::DiscriminatedUnion {
                discriminator,
                mapping,
            } => self.validate_discriminated(discriminator, mapping, value, ctx),

            Schema::OneOf { variants } => self.validate_oneof(variants, value, ctx),

            Schema::Ref { name } => {
                if let Some(resolved) = self.bundle.resolve(name) {
                    ctx.depth += 1;
                    self.validate_schema(resolved, value, ctx);
                    ctx.depth -= 1;
                } else {
                    ctx.errors.push(ValidationError {
                        instance_path: ctx.instance_path.render(),
                        schema_path: ctx.schema_path.render(),
                        code: ErrorCode::UnresolvedRef,
                        message: format!("unresolved type reference: '{name}'"),
                        hint: None,
                    });
                }
            }
        }
    }

    fn validate_scalar(&self, kind: &ScalarKind, value: &Value, ctx: &mut ValidationContext) {
        match kind {
            ScalarKind::String { pattern } => {
                if let Value::String(s) = value {
                    if let Some(pat) = pattern {
                        if let Ok(re) = regex::Regex::new(pat) {
                            if !re.is_match(s) {
                                ctx.errors.push(ValidationError {
                                    instance_path: ctx.instance_path.render(),
                                    schema_path: ctx.schema_path.with("pattern").render(),
                                    code: ErrorCode::PatternMismatch,
                                    message: format!(
                                        "string '{}' does not match pattern '{}'",
                                        s, pat
                                    ),
                                    hint: None,
                                });
                            }
                        }
                    }
                } else {
                    self.type_mismatch("string", value, ctx);
                }
            }
            ScalarKind::Boolean => {
                if !matches!(value, Value::Bool(_)) {
                    self.type_mismatch("boolean", value, ctx);
                }
            }
            ScalarKind::Int { min, max } => match value {
                Value::Number(Number::Integer(i)) => {
                    if let Some(lo) = min {
                        if *i < *lo {
                            self.range_error(*i, Some(*lo), *max, ctx);
                        }
                    }
                    if let Some(hi) = max {
                        if *i > *hi {
                            self.range_error(*i, *min, Some(*hi), ctx);
                        }
                    }
                }
                Value::Number(Number::Float(f)) => match self.mode {
                    ValidationMode::Strict => {
                        self.type_mismatch("integer", value, ctx);
                    }
                    ValidationMode::Open => {
                        if *f == f.floor() && f.is_finite() {
                            let i = *f as i64;
                            if let Some(lo) = min {
                                if i < *lo {
                                    self.range_error(i, Some(*lo), *max, ctx);
                                }
                            }
                            if let Some(hi) = max {
                                if i > *hi {
                                    self.range_error(i, *min, Some(*hi), ctx);
                                }
                            }
                        } else {
                            self.type_mismatch("integer", value, ctx);
                        }
                    }
                },
                _ => self.type_mismatch("integer", value, ctx),
            },
            ScalarKind::Float { min, max } => match value {
                Value::Number(Number::Float(f)) => {
                    if let Some(lo) = min {
                        if *f < *lo as f64 {
                            self.range_error_f(*f, Some(*lo as f64), max.map(|x| x as f64), ctx);
                        }
                    }
                    if let Some(hi) = max {
                        if *f > *hi as f64 {
                            self.range_error_f(*f, min.map(|x| x as f64), Some(*hi as f64), ctx);
                        }
                    }
                }
                Value::Number(Number::Integer(i)) => {
                    // Float accepts integers in both modes
                    let f = *i as f64;
                    if let Some(lo) = min {
                        if f < *lo as f64 {
                            self.range_error_f(f, Some(*lo as f64), max.map(|x| x as f64), ctx);
                        }
                    }
                    if let Some(hi) = max {
                        if f > *hi as f64 {
                            self.range_error_f(f, min.map(|x| x as f64), Some(*hi as f64), ctx);
                        }
                    }
                }
                _ => self.type_mismatch("float", value, ctx),
            },
            ScalarKind::Timestamp => {
                if let Value::String(s) = value {
                    // Simple ISO 8601 format check
                    let iso8601_pattern =
                        regex::Regex::new(r"^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}:\d{2})?").unwrap();
                    if !iso8601_pattern.is_match(s) {
                        ctx.errors.push(ValidationError {
                            instance_path: ctx.instance_path.render(),
                            schema_path: ctx.schema_path.render(),
                            code: ErrorCode::InvalidScalar,
                            message: format!("'{}' is not a valid ISO 8601 timestamp", s),
                            hint: None,
                        });
                    }
                } else {
                    self.type_mismatch("timestamp (ISO 8601 string)", value, ctx);
                }
            }
            ScalarKind::Custom {
                name: _,
                constraints,
            } => {
                if let Value::String(s) = value {
                    if let Some(pat) = &constraints.pattern {
                        if let Ok(re) = regex::Regex::new(pat) {
                            if !re.is_match(s) {
                                ctx.errors.push(ValidationError {
                                    instance_path: ctx.instance_path.render(),
                                    schema_path: ctx.schema_path.with("pattern").render(),
                                    code: ErrorCode::PatternMismatch,
                                    message: format!(
                                        "string '{}' does not match pattern '{}'",
                                        s, pat
                                    ),
                                    hint: None,
                                });
                            }
                        }
                    }
                    if let Some(min) = constraints.min_length {
                        if s.len() < min {
                            ctx.errors.push(ValidationError {
                                instance_path: ctx.instance_path.render(),
                                schema_path: ctx.schema_path.with("minLength").render(),
                                code: ErrorCode::ValueOutOfRange,
                                message: format!(
                                    "string length {} is below minimum {}",
                                    s.len(),
                                    min
                                ),
                                hint: None,
                            });
                        }
                    }
                    if let Some(max) = constraints.max_length {
                        if s.len() > max {
                            ctx.errors.push(ValidationError {
                                instance_path: ctx.instance_path.render(),
                                schema_path: ctx.schema_path.with("maxLength").render(),
                                code: ErrorCode::ValueOutOfRange,
                                message: format!(
                                    "string length {} exceeds maximum {}",
                                    s.len(),
                                    max
                                ),
                                hint: None,
                            });
                        }
                    }
                } else {
                    self.type_mismatch("string (custom scalar)", value, ctx);
                }
            }
        }
    }

    fn validate_enum(&self, values: &[String], val: &Value, ctx: &mut ValidationContext) {
        if let Value::String(s) = val {
            if !values.contains(s) {
                ctx.errors.push(ValidationError {
                    instance_path: ctx.instance_path.render(),
                    schema_path: ctx.schema_path.render(),
                    code: ErrorCode::InvalidEnumValue,
                    message: format!(
                        "'{}' is not a valid enum value (expected one of: {})",
                        s,
                        values.join(", ")
                    ),
                    hint: None,
                });
            }
        } else {
            self.type_mismatch("string (enum)", val, ctx);
        }
    }

    fn validate_array(&self, elements: &Schema, val: &Value, ctx: &mut ValidationContext) {
        if let Value::Array(arr) = val {
            for (i, item) in arr.iter().enumerate() {
                let prev_ip = ctx.instance_path.clone();
                let prev_sp = ctx.schema_path.clone();
                ctx.instance_path = prev_ip.with(i.to_string());
                ctx.schema_path = prev_sp.with("elements");
                self.validate_schema(elements, item, ctx);
                ctx.instance_path = prev_ip;
                ctx.schema_path = prev_sp;
            }
        } else {
            self.type_mismatch("array", val, ctx);
        }
    }

    fn validate_object(
        &self,
        required: &IndexMap<String, Box<Schema>>,
        optional: &IndexMap<String, Box<Schema>>,
        additional: &AdditionalPolicy,
        val: &Value,
        ctx: &mut ValidationContext,
    ) {
        if let Value::Object(map) = val {
            // Phase 1: Validate required properties
            for (key, schema) in required {
                let prev_ip = ctx.instance_path.clone();
                let prev_sp = ctx.schema_path.clone();
                ctx.instance_path = prev_ip.with(key);
                ctx.schema_path = prev_sp.with("properties").with(key);

                if let Some(v) = map.get(key) {
                    self.validate_schema(schema, v, ctx);
                } else {
                    ctx.errors.push(ValidationError {
                        instance_path: ctx.instance_path.render(),
                        schema_path: ctx.schema_path.render(),
                        code: ErrorCode::RequiredPropertyMissing,
                        message: format!("required property '{}' is missing", key),
                        hint: None,
                    });
                }

                ctx.instance_path = prev_ip;
                ctx.schema_path = prev_sp;
            }

            // Phase 2: Validate optional properties
            for (key, schema) in optional {
                if let Some(v) = map.get(key) {
                    let prev_ip = ctx.instance_path.clone();
                    let prev_sp = ctx.schema_path.clone();
                    ctx.instance_path = prev_ip.with(key);
                    ctx.schema_path = prev_sp.with("optionalProperties").with(key);
                    self.validate_schema(schema, v, ctx);
                    ctx.instance_path = prev_ip;
                    ctx.schema_path = prev_sp;
                }
            }

            // Phase 3: Check additional properties
            let known_keys: std::collections::HashSet<&String> =
                required.keys().chain(optional.keys()).collect();

            for (key, v) in map {
                if known_keys.contains(key) {
                    continue;
                }

                match additional {
                    AdditionalPolicy::Reject => {
                        if self.mode == ValidationMode::Strict {
                            ctx.errors.push(ValidationError {
                                instance_path: ctx.instance_path.with(key).render(),
                                schema_path: ctx.schema_path.with("additionalProperties").render(),
                                code: ErrorCode::UnknownProperty,
                                message: format!("unknown property '{}' (schema is closed)", key),
                                hint: None,
                            });
                        }
                    }
                    AdditionalPolicy::AllowAny => {}
                    AdditionalPolicy::AllowSchema(schema) => {
                        let prev_ip = ctx.instance_path.clone();
                        let prev_sp = ctx.schema_path.clone();
                        ctx.instance_path = prev_ip.with(key);
                        ctx.schema_path = prev_sp.with("additionalProperties");
                        self.validate_schema(schema, v, ctx);
                        ctx.instance_path = prev_ip;
                        ctx.schema_path = prev_sp;
                    }
                }
            }
        } else {
            self.type_mismatch("object", val, ctx);
        }
    }

    fn validate_map(&self, val_schema: &Schema, val: &Value, ctx: &mut ValidationContext) {
        if let Value::Object(map) = val {
            for (key, v) in map {
                let prev_ip = ctx.instance_path.clone();
                let prev_sp = ctx.schema_path.clone();
                ctx.instance_path = prev_ip.with(key);
                ctx.schema_path = prev_sp.with("values");
                self.validate_schema(val_schema, v, ctx);
                ctx.instance_path = prev_ip;
                ctx.schema_path = prev_sp;
            }
        } else {
            self.type_mismatch("object (map)", val, ctx);
        }
    }

    fn validate_discriminated(
        &self,
        discriminator: &str,
        mapping: &IndexMap<String, Box<Schema>>,
        val: &Value,
        ctx: &mut ValidationContext,
    ) {
        if let Value::Object(map) = val {
            if let Some(Value::String(tag)) = map.get(discriminator) {
                if let Some(variant_schema) = mapping.get(tag) {
                    let prev_sp = ctx.schema_path.clone();
                    ctx.schema_path = prev_sp.with("mapping").with(tag);
                    self.validate_schema(variant_schema, val, ctx);
                    ctx.schema_path = prev_sp;
                } else {
                    ctx.errors.push(ValidationError {
                        instance_path: ctx.instance_path.with(discriminator).render(),
                        schema_path: ctx.schema_path.with("mapping").render(),
                        code: ErrorCode::InvalidDiscriminatorValue,
                        message: format!(
                            "discriminator value '{}' not in mapping (expected one of: {})",
                            tag,
                            mapping.keys().cloned().collect::<Vec<_>>().join(", ")
                        ),
                        hint: None,
                    });
                }
            } else {
                ctx.errors.push(ValidationError {
                    instance_path: ctx.instance_path.render(),
                    schema_path: ctx.schema_path.with("discriminator").render(),
                    code: ErrorCode::RequiredPropertyMissing,
                    message: format!(
                        "missing discriminator field '{}' (must be a string)",
                        discriminator
                    ),
                    hint: None,
                });
            }
        } else {
            self.type_mismatch("object (discriminated union)", val, ctx);
        }
    }

    fn validate_oneof(
        &self,
        variants: &[validate_schema_with_graphql_ir::OneOfVariant],
        val: &Value,
        ctx: &mut ValidationContext,
    ) {
        let mut matching = Vec::new();

        for (i, variant) in variants.iter().enumerate() {
            let mut variant_ctx = ValidationContext {
                instance_path: ctx.instance_path.clone(),
                schema_path: ctx.schema_path.with("oneOf").with(i.to_string()),
                errors: Vec::new(),
                depth: ctx.depth,
            };

            // Short-circuit: check if required fields exist in value
            if let Schema::Object { required, .. } = &*variant.schema {
                if let Value::Object(map) = val {
                    let all_required_present = required.keys().all(|k| map.contains_key(k));
                    if !all_required_present {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            self.validate_schema(&variant.schema, val, &mut variant_ctx);

            if variant_ctx.errors.is_empty() {
                matching.push(variant.label.clone());
            }
        }

        match matching.len() {
            0 => {
                ctx.errors.push(ValidationError {
                    instance_path: ctx.instance_path.render(),
                    schema_path: ctx.schema_path.with("oneOf").render(),
                    code: ErrorCode::NoMatchingVariant,
                    message: "no variant matches the value".to_string(),
                    hint: Some(format!(
                        "expected one of: {}",
                        variants
                            .iter()
                            .map(|v| v.label.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                });
            }
            1 => {}
            _ => {
                ctx.errors.push(ValidationError {
                    instance_path: ctx.instance_path.render(),
                    schema_path: ctx.schema_path.with("oneOf").render(),
                    code: ErrorCode::AmbiguousVariant,
                    message: format!(
                        "ambiguous: multiple variants match ({})",
                        matching.join(", ")
                    ),
                    hint: Some(
                        "add more distinguishing fields to disambiguate variants".to_string(),
                    ),
                });
            }
        }
    }

    fn type_mismatch(&self, expected: &str, value: &Value, ctx: &mut ValidationContext) {
        let actual = value_type_name(value);
        ctx.errors.push(ValidationError {
            instance_path: ctx.instance_path.render(),
            schema_path: ctx.schema_path.render(),
            code: ErrorCode::TypeMismatch,
            message: format!("expected {}, got {}", expected, actual),
            hint: None,
        });
    }

    fn range_error(
        &self,
        val: i64,
        min: Option<i64>,
        max: Option<i64>,
        ctx: &mut ValidationContext,
    ) {
        ctx.errors.push(ValidationError {
            instance_path: ctx.instance_path.render(),
            schema_path: ctx.schema_path.render(),
            code: ErrorCode::ValueOutOfRange,
            message: format!(
                "value {} is out of range [{}, {}]",
                val,
                min.map(|m| m.to_string()).unwrap_or("−∞".to_string()),
                max.map(|m| m.to_string()).unwrap_or("∞".to_string())
            ),
            hint: None,
        });
    }

    fn range_error_f(
        &self,
        val: f64,
        min: Option<f64>,
        max: Option<f64>,
        ctx: &mut ValidationContext,
    ) {
        ctx.errors.push(ValidationError {
            instance_path: ctx.instance_path.render(),
            schema_path: ctx.schema_path.render(),
            code: ErrorCode::ValueOutOfRange,
            message: format!(
                "value {} is out of range [{}, {}]",
                val,
                min.map(|m| m.to_string()).unwrap_or("−∞".to_string()),
                max.map(|m| m.to_string()).unwrap_or("∞".to_string())
            ),
            hint: None,
        });
    }
}

fn value_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(Number::Integer(_)) => "integer",
        Value::Number(Number::Float(_)) => "float",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

struct ValidationContext {
    instance_path: JsonPointer,
    schema_path: JsonPointer,
    errors: Vec<ValidationError>,
    depth: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validate_schema_with_graphql_ir::{AdditionalPolicy, Schema, SchemaBundle};

    fn simple_object_schema() -> SchemaBundle {
        let mut bundle = SchemaBundle::new();
        bundle.insert(
            "Widget".into(),
            Schema::Object {
                required: {
                    let mut m = IndexMap::new();
                    m.insert(
                        "name".into(),
                        Box::new(Schema::Scalar(ScalarKind::String { pattern: None })),
                    );
                    m.insert(
                        "count".into(),
                        Box::new(Schema::Scalar(ScalarKind::Int {
                            min: Some(0),
                            max: None,
                        })),
                    );
                    m
                },
                optional: {
                    let mut m = IndexMap::new();
                    m.insert(
                        "tag".into(),
                        Box::new(Schema::Scalar(ScalarKind::String { pattern: None })),
                    );
                    m
                },
                additional: AdditionalPolicy::Reject,
            },
        );
        bundle.set_root("Widget");
        bundle
    }

    #[test]
    fn valid_object() {
        let bundle = simple_object_schema();
        let validator = Validator::new(&bundle);
        let value = Value::Object({
            let mut m = IndexMap::new();
            m.insert("name".into(), Value::String("test".into()));
            m.insert("count".into(), Value::Number(Number::Integer(5)));
            m
        });
        let result = validator.validate(&value);
        assert!(
            result.valid,
            "expected valid, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn missing_required_field() {
        let bundle = simple_object_schema();
        let validator = Validator::new(&bundle);
        let value = Value::Object({
            let mut m = IndexMap::new();
            m.insert("name".into(), Value::String("test".into()));
            m
        });
        let result = validator.validate(&value);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == ErrorCode::RequiredPropertyMissing));
    }

    #[test]
    fn unknown_field_in_strict_mode() {
        let bundle = simple_object_schema();
        let validator = Validator::new(&bundle).with_mode(ValidationMode::Strict);
        let value = Value::Object({
            let mut m = IndexMap::new();
            m.insert("name".into(), Value::String("test".into()));
            m.insert("count".into(), Value::Number(Number::Integer(1)));
            m.insert("bogus".into(), Value::String("nope".into()));
            m
        });
        let result = validator.validate(&value);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == ErrorCode::UnknownProperty));
    }

    #[test]
    fn unknown_field_ok_in_open_mode() {
        let bundle = simple_object_schema();
        let validator = Validator::new(&bundle).with_mode(ValidationMode::Open);
        let value = Value::Object({
            let mut m = IndexMap::new();
            m.insert("name".into(), Value::String("test".into()));
            m.insert("count".into(), Value::Number(Number::Integer(1)));
            m.insert("extra".into(), Value::String("fine".into()));
            m
        });
        let result = validator.validate(&value);
        assert!(
            result.valid,
            "open mode should allow extra fields, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn enum_validation() {
        let mut bundle = SchemaBundle::new();
        bundle.insert(
            "Color".into(),
            Schema::Enum {
                values: vec!["red".into(), "green".into(), "blue".into()],
            },
        );
        bundle.set_root("Color");

        let validator = Validator::new(&bundle);
        let result = validator.validate(&Value::String("red".into()));
        assert!(result.valid);

        let result = validator.validate(&Value::String("purple".into()));
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == ErrorCode::InvalidEnumValue));
    }

    #[test]
    fn parse_json_value() {
        let v = parse_json(r#"{"name": "test", "count": 5}"#).unwrap();
        assert!(matches!(v, Value::Object(_)));
    }
}
