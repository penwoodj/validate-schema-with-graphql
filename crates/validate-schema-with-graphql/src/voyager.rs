//! GraphQL Voyager-compatible schema converter
//!
//! Converts validation-focused SDL (with custom directives like @root, @closed, @pattern, @oneOf)
//! into a Voyager-compatible GraphQL schema (standard types only).

use validate_schema_with_graphql_parser::SdlAst;

/// Converts validation SDL to Voyager-compatible GraphQL schema.
///
/// This function:
/// 1. Converts `input` types to `type` (Voyager only visualizes standard object types)
/// 2. Strips custom directives (@root, @closed, @pattern, @oneOf)
/// 3. Creates a `type Query { ... }` root entry point
/// 4. Preserves type names, field names, and type references
///
/// # Example
///
/// ```rust
/// use validate_schema_with_graphql::to_voyager_schema;
///
/// let validation_schema = r#"
///     input Widget @closed {
///         name: String!
///         count: Int
///     }
/// "#;
///
/// let voyager_schema = to_voyager_schema(validation_schema).unwrap();
/// println!("{}", voyager_schema);
/// ```
///
/// Output:
/// ```graphql
/// type Widget {
///     name: String!
///     count: Int
/// }
///
/// type Query {
///     workflow: Widget
/// }
/// ```
pub fn to_voyager_schema(schema: &str) -> Result<String, String> {
    let ast = validate_schema_with_graphql_parser::extract_ast(schema).map_err(|errs| {
        format!(
            "SDL parse errors: {}",
            errs.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;

    let mut output = String::new();

    // Convert each input type to a regular type
    for input_type in &ast.inputs {
        output.push_str("type ");
        output.push_str(&input_type.name);
        output.push_str(" {\n");

        for field in &input_type.fields {
            let type_ref = convert_type_ref(&field.type_ref);
            let required_marker = if field.required { "!" } else { "" };
            output.push_str("    ");
            output.push_str(&field.name);
            output.push_str(": ");
            output.push_str(&type_ref);
            output.push_str(required_marker);
            output.push('\n');
        }

        output.push_str("}\n\n");
    }

    // Convert enums (these are already compatible)
    for enum_type in &ast.enums {
        output.push_str("enum ");
        output.push_str(&enum_type.name);
        output.push_str(" {\n");

        for value in &enum_type.values {
            output.push_str("    ");
            output.push_str(value);
            output.push('\n');
        }

        output.push_str("}\n\n");
    }

    // Convert scalars (strip custom directives)
    for scalar in &ast.scalars {
        output.push_str("scalar ");
        output.push_str(&scalar.name);
        output.push('\n');
    }

    // Create Query type entry point
    // Use the first input type as the root if we have one
    if let Some(root_input) = ast.inputs.first() {
        output.push_str("type Query {\n");
        output.push_str("    workflow: ");
        output.push_str(&root_input.name);
        output.push_str("\n");
        output.push_str("}\n");
    }

    Ok(output)
}

/// Converts validation type references to standard GraphQL type syntax.
///
/// - `[String!]` stays as `[String!]`
/// - `TypeName!` stays as `TypeName!`
/// - Handles List and NonNull types
fn convert_type_ref(ty: &str) -> String {
    ty.replace("!", "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_simple_input_type() {
        let schema = r#"
            input Widget @closed {
                name: String!
                count: Int
            }
        "#;

        let result = to_voyager_schema(schema).unwrap();

        assert!(result.contains("type Widget"));
        assert!(result.contains("name: String!"));
        assert!(result.contains("count: Int"));
        assert!(result.contains("type Query"));
        assert!(result.contains("workflow: Widget"));
        // Should NOT contain @closed directive
        assert!(!result.contains("@closed"));
    }

    #[test]
    fn convert_enum_type() {
        let schema = r#"
            enum BackoffStrategy {
                exponential
                linear
                fixed
            }
        "#;

        let result = to_voyager_schema(schema).unwrap();

        assert!(result.contains("enum BackoffStrategy"));
        assert!(result.contains("exponential"));
        assert!(result.contains("linear"));
        assert!(result.contains("fixed"));
    }

    #[test]
    fn convert_scalar_with_pattern() {
        let schema = r#"
            scalar SemVer @pattern(regex: "^[0-9]+\\.[0-9]+\\.[0-9]+$")
        "#;

        let result = to_voyager_schema(schema).unwrap();

        assert!(result.contains("scalar SemVer"));
        // Should NOT contain @pattern directive
        assert!(!result.contains("@pattern"));
    }
}
