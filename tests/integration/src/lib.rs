#[cfg(test)]
use graphql_ish_schema_validator_compiler::compile;
#[cfg(test)]
use graphql_ish_schema_validator_diagnostics::ValidationMode;
#[cfg(test)]
use graphql_ish_schema_validator_ir::SchemaBundle;
#[cfg(test)]
use graphql_ish_schema_validator_parser::extract_ast;
#[cfg(test)]
use graphql_ish_schema_validator_validator::{parse_json, parse_yaml_with_mode, Validator};

#[cfg(test)]
fn workspace_root() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

#[cfg(test)]
fn fixture(path: &str) -> String {
    format!("{}/../{}", workspace_root(), path)
}

#[cfg(test)]
fn load_and_compile_schema(path: &str) -> SchemaBundle {
    let sdl = std::fs::read_to_string(fixture(path)).unwrap();
    let ast = extract_ast(&sdl).unwrap();
    compile(&ast).unwrap()
}

#[test]
fn end_to_end_valid_yaml() {
    let bundle = load_and_compile_schema("fixtures/schemas/simple.graphql");
    let yaml = std::fs::read_to_string(fixture("fixtures/documents/valid/widget.yaml")).unwrap();
    let value = parse_yaml_with_mode(&yaml, ValidationMode::Strict).unwrap();

    let validator = Validator::new(&bundle).with_mode(ValidationMode::Strict);
    let result = validator.validate(&value);

    assert!(
        result.valid,
        "expected valid, got errors: {:#?}",
        result.errors
    );
}

#[test]
fn end_to_end_invalid_yaml() {
    let bundle = load_and_compile_schema("fixtures/schemas/simple.graphql");
    let yaml =
        std::fs::read_to_string(fixture("fixtures/documents/invalid/widget_bad.yaml")).unwrap();
    let value = parse_yaml_with_mode(&yaml, ValidationMode::Strict).unwrap();

    let validator = Validator::new(&bundle).with_mode(ValidationMode::Strict);
    let result = validator.validate(&value);

    assert!(!result.valid, "expected invalid");

    let codes: Vec<_> = result.errors.iter().map(|e| e.code).collect();
    assert!(
        codes.contains(&graphql_ish_schema_validator_diagnostics::ErrorCode::UnknownProperty),
        "expected UnknownProperty error, got: {:?}",
        codes
    );
}

#[test]
fn end_to_end_valid_json() {
    let bundle = load_and_compile_schema("fixtures/schemas/simple.graphql");
    let json = r#"{
        "name": "json-widget",
        "version": "2.0.0",
        "retries": {
            "max_attempts": 5,
            "backoff": "linear"
        },
        "tags": ["json"]
    }"#;
    let value = parse_json(json).unwrap();

    let validator = Validator::new(&bundle).with_mode(ValidationMode::Strict);
    let result = validator.validate(&value);

    assert!(
        result.valid,
        "expected valid, got errors: {:#?}",
        result.errors
    );
}

#[test]
fn end_to_end_missing_required_field() {
    let bundle = load_and_compile_schema("fixtures/schemas/simple.graphql");
    let json = r#"{"name": "incomplete"}"#;
    let value = parse_json(json).unwrap();

    let validator = Validator::new(&bundle);
    let result = validator.validate(&value);

    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code
            == graphql_ish_schema_validator_diagnostics::ErrorCode::RequiredPropertyMissing));
}

#[test]
fn end_to_end_open_mode_allows_extra_fields() {
    let bundle = load_and_compile_schema("fixtures/schemas/simple.graphql");
    let json = r#"{
        "name": "widget",
        "version": "1.0.0",
        "extra": "allowed"
    }"#;
    let value = parse_json(json).unwrap();

    let validator = Validator::new(&bundle).with_mode(ValidationMode::Open);
    let result = validator.validate(&value);

    assert!(
        result.valid,
        "open mode should allow extra fields, got: {:?}",
        result.errors
    );
}

#[test]
fn end_to_end_nested_enum_validation() {
    let bundle = load_and_compile_schema("fixtures/schemas/simple.graphql");
    let json = r#"{
        "name": "widget",
        "version": "1.0.0",
        "retries": {
            "max_attempts": 3,
            "backoff": "INVALID"
        }
    }"#;
    let value = parse_json(json).unwrap();

    let validator = Validator::new(&bundle);
    let result = validator.validate(&value);

    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code == graphql_ish_schema_validator_diagnostics::ErrorCode::InvalidEnumValue));
}

#[test]
fn end_to_end_workflow_schema() {
    let bundle = load_and_compile_schema("fixtures/schemas/workflow.graphql");

    // Test valid workflow
    let yaml = std::fs::read_to_string(fixture("fixtures/documents/valid/workflow.yaml")).unwrap();
    let value = parse_yaml_with_mode(&yaml, ValidationMode::Strict).unwrap();
    let validator = Validator::new(&bundle).with_mode(ValidationMode::Strict);
    let result = validator.validate(&value);
    assert!(
        result.valid,
        "expected valid workflow, got errors: {:#?}",
        result.errors
    );

    // Test invalid workflow
    let yaml =
        std::fs::read_to_string(fixture("fixtures/documents/invalid/workflow_bad.yaml")).unwrap();
    let value = parse_yaml_with_mode(&yaml, ValidationMode::Strict).unwrap();
    let validator = Validator::new(&bundle).with_mode(ValidationMode::Strict);
    let result = validator.validate(&value);
    assert!(!result.valid, "expected invalid workflow");

    // Should have multiple errors
    let codes: Vec<_> = result.errors.iter().map(|e| e.code).collect();
    assert!(
        codes.len() >= 2,
        "expected multiple errors, got: {:?}",
        codes
    );
}
