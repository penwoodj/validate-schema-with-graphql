//! graphql-ish-schema-validator-compiler: SDL AST to IR lowering compiler

use indexmap::IndexMap;
use std::collections::HashSet;
use validate_schema_with_graphql_diagnostics::LoweringError;
use validate_schema_with_graphql_ir::{
    AdditionalPolicy, OneOfVariant, ScalarConstraints, ScalarKind, Schema, SchemaBundle,
};
use validate_schema_with_graphql_parser::{
    DirectiveInfo, EnumTypeInfo, InputFieldInfo, InputTypeInfo, ScalarTypeInfo, SdlAst,
    UnionTypeInfo,
};

/// Compile an SDL AST into a SchemaBundle (IR).
pub fn compile(ast: &SdlAst) -> Result<SchemaBundle, Vec<LoweringError>> {
    let mut ctx = LoweringContext {
        bundle: SchemaBundle::new(),
        errors: Vec::new(),
        type_names: ast
            .inputs
            .iter()
            .map(|i| i.name.clone())
            .chain(ast.enums.iter().map(|e| e.name.clone()))
            .chain(ast.unions.iter().map(|u| u.name.clone()))
            .chain(ast.scalars.iter().map(|s| s.name.clone()))
            .collect::<HashSet<_>>(),
    };

    // Lower scalars first (they're referenced by other types)
    for scalar in &ast.scalars {
        ctx.lower_scalar(scalar);
    }

    // Lower enums
    for en in &ast.enums {
        ctx.lower_enum(en);
    }

    // Lower input types
    for input in &ast.inputs {
        ctx.lower_input(input);
    }

    // Lower unions
    for union in &ast.unions {
        ctx.lower_union(union);
    }

    // Set root: explicit @root directive, otherwise last input type
    for input in &ast.inputs {
        if input.directives.iter().any(|d| d.name == "root") {
            ctx.bundle.set_root(&input.name);
            break;
        }
    }
    if ctx.bundle.root_name.is_none() {
        if let Some(last) = ast.inputs.last() {
            ctx.bundle.set_root(&last.name);
        }
    }

    if ctx.errors.is_empty() {
        Ok(ctx.bundle)
    } else {
        Err(ctx.errors)
    }
}

struct LoweringContext {
    bundle: SchemaBundle,
    errors: Vec<LoweringError>,
    type_names: HashSet<String>,
}

impl LoweringContext {
    fn lower_scalar(&mut self, scalar: &ScalarTypeInfo) {
        let mut constraints = ScalarConstraints::default();

        for dir in &scalar.directives {
            match dir.name.as_str() {
                "pattern" => {
                    if let Some(val) = dir_arg_unquote(dir, "regex") {
                        match regex::Regex::new(&val) {
                            Ok(_) => constraints.pattern = Some(val),
                            Err(e) => {
                                self.errors.push(LoweringError::InvalidDirective {
                                    directive: "pattern".into(),
                                    target: scalar.name.clone(),
                                    reason: format!("invalid regex: {e}"),
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let schema = Schema::Scalar(ScalarKind::Custom {
            name: scalar.name.clone(),
            constraints,
        });
        self.bundle.insert(scalar.name.clone(), schema);
    }

    fn lower_enum(&mut self, en: &EnumTypeInfo) {
        let schema = Schema::Enum {
            values: en.values.clone(),
        };
        self.bundle.insert(en.name.clone(), schema);
    }

    fn lower_input(&mut self, input: &InputTypeInfo) {
        let additional = self.compute_additional_policy(&input.directives, &input.name);
        let mut required = IndexMap::new();
        let mut optional = IndexMap::new();

        for field in &input.fields {
            let field_schema = self.lower_field_type(field);
            if field.required {
                required.insert(field.name.clone(), Box::new(field_schema));
            } else {
                optional.insert(field.name.clone(), Box::new(field_schema));
            }
        }

        let schema = Schema::Object {
            required,
            optional,
            additional,
        };
        self.bundle.insert(input.name.clone(), schema);
    }

    fn lower_field_type(&mut self, field: &InputFieldInfo) -> Schema {
        let base = self.resolve_type_ref(&field.type_ref);

        // Apply field-level directives
        let mut schema = base;
        for dir in &field.directives {
            match dir.name.as_str() {
                "pattern" => {
                    if let Some(pat) = dir_arg_unquote(dir, "regex") {
                        schema =
                            apply_pattern_to_schema(schema, &pat, &field.name, &mut self.errors);
                    }
                }
                "min" | "max" => {
                    if let Some(val) = dir_arg_unquote(dir, "value") {
                        schema = apply_range_to_schema(
                            schema,
                            &dir.name,
                            &val,
                            &field.name,
                            &mut self.errors,
                        );
                    }
                }
                _ => {}
            }
        }

        schema
    }

    fn resolve_type_ref(&self, type_ref: &str) -> Schema {
        let trimmed = type_ref.trim_end_matches('!');
        let is_list = trimmed.starts_with('[') && trimmed.ends_with(']');

        if is_list {
            let inner = trimmed.trim_start_matches('[').trim_end_matches(']');
            let inner_trimmed = inner.trim_end_matches('!');
            let element = self.resolve_named_type(inner_trimmed);
            Schema::Array {
                elements: Box::new(element),
            }
        } else {
            self.resolve_named_type(trimmed)
        }
    }

    fn resolve_named_type(&self, name: &str) -> Schema {
        match name {
            "String" => Schema::Scalar(ScalarKind::String { pattern: None }),
            "Int" => Schema::Scalar(ScalarKind::Int {
                min: None,
                max: None,
            }),
            "Float" => Schema::Scalar(ScalarKind::Float {
                min: None,
                max: None,
            }),
            "Boolean" => Schema::Scalar(ScalarKind::Boolean),
            "ID" => Schema::Scalar(ScalarKind::String { pattern: None }),
            other => {
                if self.type_names.contains(other) {
                    Schema::Ref {
                        name: other.to_string(),
                    }
                } else {
                    Schema::Ref {
                        name: other.to_string(),
                    }
                }
            }
        }
    }

    fn compute_additional_policy(
        &mut self,
        directives: &[DirectiveInfo],
        target: &str,
    ) -> AdditionalPolicy {
        let has_closed = directives.iter().any(|d| d.name == "closed");
        let has_open = directives.iter().any(|d| d.name == "open");
        let map_rest = directives.iter().find(|d| d.name == "mapRest");

        if has_closed && has_open {
            self.errors.push(LoweringError::ConflictingDirectives {
                target: target.to_string(),
                detail: "@closed and @open are mutually exclusive".to_string(),
            });
        }

        if let Some(mr) = map_rest {
            if let Some(ref_type) = dir_arg(mr, "value") {
                if self.type_names.contains(ref_type) {
                    AdditionalPolicy::AllowSchema(Box::new(Schema::Ref {
                        name: ref_type.to_string(),
                    }))
                } else {
                    self.errors.push(LoweringError::UnresolvedRef {
                        name: ref_type.to_string(),
                    });
                    AdditionalPolicy::AllowAny
                }
            } else {
                AdditionalPolicy::AllowAny
            }
        } else if has_closed {
            AdditionalPolicy::Reject
        } else if has_open {
            AdditionalPolicy::AllowAny
        } else {
            // Default: closed (reject additional properties)
            AdditionalPolicy::Reject
        }
    }

    fn lower_union(&mut self, union: &UnionTypeInfo) {
        let has_one_of = union.directives.iter().any(|d| d.name == "oneOf");
        let discriminator = union.directives.iter().find(|d| d.name == "discriminator");

        if let Some(disc) = discriminator {
            if let Some(field) = dir_arg(disc, "field") {
                let mut mapping = IndexMap::new();
                for member in &union.members {
                    mapping.insert(
                        member.clone(),
                        Box::new(Schema::Ref {
                            name: member.clone(),
                        }),
                    );
                }
                let schema = Schema::DiscriminatedUnion {
                    discriminator: field.to_string(),
                    mapping,
                };
                self.bundle.insert(union.name.clone(), schema);
                return;
            }
        }

        if has_one_of {
            let variants: Vec<OneOfVariant> = union
                .members
                .iter()
                .map(|m| OneOfVariant {
                    label: m.clone(),
                    schema: Box::new(Schema::Ref { name: m.clone() }),
                })
                .collect();
            let schema = Schema::OneOf { variants };
            self.bundle.insert(union.name.clone(), schema);
            return;
        }

        // Plain union (no discriminator, no oneOf) — treat as OneOf
        let variants: Vec<OneOfVariant> = union
            .members
            .iter()
            .map(|m| OneOfVariant {
                label: m.clone(),
                schema: Box::new(Schema::Ref { name: m.clone() }),
            })
            .collect();
        let schema = Schema::OneOf { variants };
        self.bundle.insert(union.name.clone(), schema);
    }
}

fn dir_arg<'a>(dir: &'a DirectiveInfo, name: &str) -> Option<&'a str> {
    dir.arguments
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

fn dir_arg_unquote(dir: &DirectiveInfo, name: &str) -> Option<String> {
    dir_arg(dir, name).map(|v| {
        let s = v.trim();
        if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
            let unquoted = &s[1..s.len() - 1];
            unescape_graphql_string(unquoted)
        } else {
            s.to_string()
        }
    })
}

fn unescape_graphql_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('b') => result.push('\x08'),
                Some('f') => result.push('\x0c'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('\'') => result.push('\''),
                Some('u') => {
                    let mut hex = String::new();
                    for _ in 0..4 {
                        if let Some(h) = chars.next() {
                            hex.push(h);
                        }
                    }
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(unicode_char) = char::from_u32(code) {
                            result.push(unicode_char);
                        }
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => {
                    result.push('\\');
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn apply_pattern_to_schema(
    schema: Schema,
    pattern: &str,
    field_name: &str,
    errors: &mut Vec<LoweringError>,
) -> Schema {
    match regex::Regex::new(pattern) {
        Ok(_) => match schema {
            Schema::Scalar(ScalarKind::String { .. }) => Schema::Scalar(ScalarKind::String {
                pattern: Some(pattern.to_string()),
            }),
            Schema::Scalar(ScalarKind::Custom {
                name,
                mut constraints,
            }) => {
                constraints.pattern = Some(pattern.to_string());
                Schema::Scalar(ScalarKind::Custom { name, constraints })
            }
            other => other,
        },
        Err(e) => {
            errors.push(LoweringError::InvalidDirective {
                directive: "pattern".into(),
                target: field_name.to_string(),
                reason: format!("invalid regex: {e}"),
            });
            schema
        }
    }
}

fn apply_range_to_schema(
    schema: Schema,
    dir_name: &str,
    value_str: &str,
    field_name: &str,
    errors: &mut Vec<LoweringError>,
) -> Schema {
    let value: i64 = match value_str.parse() {
        Ok(v) => v,
        Err(e) => {
            errors.push(LoweringError::InvalidDirective {
                directive: dir_name.to_string(),
                target: field_name.to_string(),
                reason: format!("invalid integer: {e}"),
            });
            return schema;
        }
    };

    match schema {
        Schema::Scalar(ScalarKind::Int { min, max }) => Schema::Scalar(ScalarKind::Int {
            min: if dir_name == "min" { Some(value) } else { min },
            max: if dir_name == "max" { Some(value) } else { max },
        }),
        Schema::Scalar(ScalarKind::Float { min, max }) => Schema::Scalar(ScalarKind::Float {
            min: if dir_name == "min" { Some(value) } else { min },
            max: if dir_name == "max" { Some(value) } else { max },
        }),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validate_schema_with_graphql_parser::extract_ast;

    fn compile_sdl(sdl: &str) -> Result<SchemaBundle, Vec<LoweringError>> {
        let ast = extract_ast(sdl).map_err(|_| {
            vec![LoweringError::UnsupportedConstruct {
                detail: "parse error".to_string(),
            }]
        })?;
        compile(&ast)
    }

    #[test]
    fn compile_simple_input() {
        let sdl = r#"
            input Widget {
                name: String!
                count: Int
            }
        "#;
        let bundle = compile_sdl(sdl).unwrap();
        assert!(bundle.resolve("Widget").is_some());
        assert!(bundle.root_name.is_some());
    }

    #[test]
    fn compile_with_enum() {
        let sdl = r#"
            enum Color { red green blue }
            input Thing {
                color: Color!
            }
        "#;
        let bundle = compile_sdl(sdl).unwrap();
        let color_schema = bundle.resolve("Color").unwrap();
        assert!(matches!(color_schema, Schema::Enum { .. }));

        let thing_schema = bundle.resolve("Thing").unwrap();
        if let Schema::Object { required, .. } = thing_schema {
            assert!(required.contains_key("color"));
        } else {
            panic!("expected Object schema");
        }
    }

    #[test]
    fn compile_closed_directive() {
        let sdl = r#"
            input Doc @closed {
                name: String!
            }
        "#;
        let bundle = compile_sdl(sdl).unwrap();
        if let Schema::Object { additional, .. } = bundle.resolve("Doc").unwrap() {
            assert_eq!(*additional, AdditionalPolicy::Reject);
        }
    }

    #[test]
    fn compile_open_directive() {
        let sdl = r#"
            input Doc @open {
                name: String!
            }
        "#;
        let bundle = compile_sdl(sdl).unwrap();
        if let Schema::Object { additional, .. } = bundle.resolve("Doc").unwrap() {
            assert_eq!(*additional, AdditionalPolicy::AllowAny);
        }
    }

    #[test]
    fn compile_union_oneof() {
        let sdl = r#"
            input A { x: String! }
            input B { y: Int! }
            union Step @oneOf = A | B
        "#;
        let bundle = compile_sdl(sdl).unwrap();
        if let Schema::OneOf { variants } = bundle.resolve("Step").unwrap() {
            assert_eq!(variants.len(), 2);
            assert_eq!(variants[0].label, "A");
        } else {
            panic!("expected OneOf");
        }
    }

    #[test]
    fn compile_pattern_directive() {
        let sdl = r#"
            input Doc {
                name: String! @pattern(regex: "^[a-z]+$")
            }
        "#;
        let bundle = compile_sdl(sdl).unwrap();
        if let Schema::Object { required, .. } = bundle.resolve("Doc").unwrap() {
            if let Schema::Scalar(ScalarKind::String { pattern }) = &**required.get("name").unwrap()
            {
                assert_eq!(pattern.as_deref(), Some("^[a-z]+$"));
            } else {
                panic!("expected String scalar");
            }
        }
    }
}
