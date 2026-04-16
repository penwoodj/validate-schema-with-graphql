//! graphql-ish-schema-validator-parser: GraphQL SDL parser using apollo-parser with CST walking

use apollo_parser::{cst, cst::CstNode, Parser as ApolloParser};
use graphql_ish_schema_validator_diagnostics::SdlError;
use std::collections::HashSet;

/// Result of parsing SDL source.
#[derive(Debug)]
pub struct ParsedSdl {
    pub type_names: Vec<String>,
    pub raw_errors: Vec<SdlError>,
}

/// Parse SDL source and report syntax errors.
/// Returns parsed CST + extracted type names. Does NOT lower to IR.
pub fn parse_sdl(input: &str) -> Result<ParsedSdl, Vec<SdlError>> {
    let parser = ApolloParser::new(input);
    let tree = parser.parse();

    let mut errors: Vec<SdlError> = Vec::new();

    for err in tree.errors() {
        errors.push(SdlError::ParseError {
            line: 0,
            col: 0,
            message: err.message().to_string(),
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let doc = tree.document();
    let mut type_names = Vec::new();

    for def in doc.definitions() {
        match def {
            cst::Definition::InputObjectTypeDefinition(d) => {
                if let Some(name) = d.name() {
                    type_names.push(name.text().to_string());
                }
            }
            cst::Definition::ScalarTypeDefinition(d) => {
                if let Some(name) = d.name() {
                    type_names.push(name.text().to_string());
                }
            }
            cst::Definition::EnumTypeDefinition(d) => {
                if let Some(name) = d.name() {
                    type_names.push(name.text().to_string());
                }
            }
            cst::Definition::UnionTypeDefinition(d) => {
                if let Some(name) = d.name() {
                    type_names.push(name.text().to_string());
                }
            }
            _ => {}
        }
    }

    Ok(ParsedSdl {
        type_names,
        raw_errors: Vec::new(),
    })
}

/// Extract all directive names used in the SDL source.
pub fn extract_directive_names(input: &str) -> Vec<String> {
    let parser = ApolloParser::new(input);
    let tree = parser.parse();
    let doc = tree.document();
    let mut names = HashSet::new();

    for def in doc.definitions() {
        collect_directives_from_def(&def, &mut names);
    }

    let mut result: Vec<String> = names.into_iter().collect();
    result.sort();
    result
}

fn collect_directives_from_def(def: &cst::Definition, names: &mut HashSet<String>) {
    let directives_node = match def {
        cst::Definition::InputObjectTypeDefinition(d) => d.directives(),
        cst::Definition::ScalarTypeDefinition(d) => d.directives(),
        cst::Definition::EnumTypeDefinition(d) => d.directives(),
        cst::Definition::UnionTypeDefinition(d) => d.directives(),
        cst::Definition::ObjectTypeDefinition(d) => d.directives(),
        cst::Definition::InterfaceTypeDefinition(d) => d.directives(),
        _ => None,
    };

    if let Some(directives) = directives_node {
        for dir in directives.directives() {
            if let Some(name) = dir.name() {
                names.insert(name.text().to_string());
            }
        }
    }
}

/// Description of a parsed input field.
#[derive(Debug, Clone)]
pub struct InputFieldInfo {
    pub name: String,
    pub type_ref: String,
    pub required: bool,
    pub description: Option<String>,
    pub default_value: Option<String>,
    pub directives: Vec<DirectiveInfo>,
}

/// Description of a parsed directive.
#[derive(Debug, Clone)]
pub struct DirectiveInfo {
    pub name: String,
    pub arguments: Vec<(String, String)>,
}

/// Description of a parsed input type.
#[derive(Debug, Clone)]
pub struct InputTypeInfo {
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<InputFieldInfo>,
    pub directives: Vec<DirectiveInfo>,
}

/// Description of a parsed enum type.
#[derive(Debug, Clone)]
pub struct EnumTypeInfo {
    pub name: String,
    pub description: Option<String>,
    pub values: Vec<String>,
    pub directives: Vec<DirectiveInfo>,
}

/// Description of a parsed union type.
#[derive(Debug, Clone)]
pub struct UnionTypeInfo {
    pub name: String,
    pub description: Option<String>,
    pub members: Vec<String>,
    pub directives: Vec<DirectiveInfo>,
}

/// Description of a parsed scalar type.
#[derive(Debug, Clone)]
pub struct ScalarTypeInfo {
    pub name: String,
    pub description: Option<String>,
    pub directives: Vec<DirectiveInfo>,
}

/// Full extracted SDL AST (our own representation, not Rowan-based).
#[derive(Debug, Default)]
pub struct SdlAst {
    pub inputs: Vec<InputTypeInfo>,
    pub enums: Vec<EnumTypeInfo>,
    pub unions: Vec<UnionTypeInfo>,
    pub scalars: Vec<ScalarTypeInfo>,
}

/// Extract the full AST from SDL source.
pub fn extract_ast(input: &str) -> Result<SdlAst, Vec<SdlError>> {
    let parser = ApolloParser::new(input);
    let tree = parser.parse();

    let mut errors: Vec<SdlError> = Vec::new();
    for err in tree.errors() {
        errors.push(SdlError::ParseError {
            line: 0,
            col: 0,
            message: err.message().to_string(),
        });
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let doc = tree.document();
    let mut ast = SdlAst::default();

    for def in doc.definitions() {
        match def {
            cst::Definition::InputObjectTypeDefinition(d) => {
                ast.inputs.push(extract_input_type(d));
            }
            cst::Definition::EnumTypeDefinition(d) => {
                ast.enums.push(extract_enum_type(d));
            }
            cst::Definition::UnionTypeDefinition(d) => {
                ast.unions.push(extract_union_type(d));
            }
            cst::Definition::ScalarTypeDefinition(d) => {
                ast.scalars.push(extract_scalar_type(d));
            }
            _ => {}
        }
    }

    Ok(ast)
}

fn extract_input_type(def: cst::InputObjectTypeDefinition) -> InputTypeInfo {
    let name = def.name().map(|n| n.text().to_string()).unwrap_or_default();
    let description = def.description().and_then(|d| {
        d.string_value()
            .map(|sv| sv.source_string().trim_matches('"').to_string())
    });
    let directives = extract_directives(def.directives());
    let fields = def
        .input_fields_definition()
        .map(|f| {
            f.input_value_definitions()
                .map(extract_input_field)
                .collect()
        })
        .unwrap_or_default();

    InputTypeInfo {
        name,
        description,
        fields,
        directives,
    }
}

fn extract_input_field(field: cst::InputValueDefinition) -> InputFieldInfo {
    let name = field
        .name()
        .map(|n| n.text().to_string())
        .unwrap_or_default();
    let type_ref = field.ty().map(extract_type_ref).unwrap_or_default();
    let required = field.ty().map(is_required_type).unwrap_or(false);
    let description = field.description().and_then(|d| {
        d.string_value()
            .map(|sv| sv.source_string().trim_matches('"').to_string())
    });
    let default_value = field.default_value().map(|dv| dv.source_string());
    let directives = extract_directives(field.directives());

    InputFieldInfo {
        name,
        type_ref,
        required,
        description,
        default_value,
        directives,
    }
}

fn extract_type_ref(ty: cst::Type) -> String {
    match ty {
        cst::Type::NamedType(n) => n.name().map(|nm| nm.text().to_string()).unwrap_or_default(),
        cst::Type::ListType(l) => {
            let inner = l.ty().map(extract_type_ref).unwrap_or_default();
            format!("[{inner}]")
        }
        cst::Type::NonNullType(nn) => {
            if let Some(named) = nn.named_type() {
                let n = named
                    .name()
                    .map(|nm| nm.text().to_string())
                    .unwrap_or_default();
                format!("{n}!")
            } else if let Some(list) = nn.list_type() {
                let inner = list.ty().map(extract_type_ref).unwrap_or_default();
                format!("[{inner}]!")
            } else {
                String::new()
            }
        }
    }
}

fn is_required_type(ty: cst::Type) -> bool {
    matches!(ty, cst::Type::NonNullType(_))
}

fn extract_enum_type(def: cst::EnumTypeDefinition) -> EnumTypeInfo {
    let name = def.name().map(|n| n.text().to_string()).unwrap_or_default();
    let description = def.description().and_then(|d| {
        d.string_value()
            .map(|sv| sv.source_string().trim_matches('"').to_string())
    });
    let directives = extract_directives(def.directives());
    let values = def
        .enum_values_definition()
        .map(|v| {
            v.enum_value_definitions()
                .filter_map(|ev| ev.enum_value().and_then(|e| e.name()))
                .map(|n| n.text().to_string())
                .collect()
        })
        .unwrap_or_default();

    EnumTypeInfo {
        name,
        description,
        values,
        directives,
    }
}

fn extract_union_type(def: cst::UnionTypeDefinition) -> UnionTypeInfo {
    let name = def.name().map(|n| n.text().to_string()).unwrap_or_default();
    let description = def.description().and_then(|d| {
        d.string_value()
            .map(|sv| sv.source_string().trim_matches('"').to_string())
    });
    let directives = extract_directives(def.directives());
    let members = def
        .union_member_types()
        .map(|m| {
            m.named_types()
                .filter_map(|nt| nt.name().map(|n| n.text().to_string()))
                .collect()
        })
        .unwrap_or_default();

    UnionTypeInfo {
        name,
        description,
        members,
        directives,
    }
}

fn extract_scalar_type(def: cst::ScalarTypeDefinition) -> ScalarTypeInfo {
    let name = def.name().map(|n| n.text().to_string()).unwrap_or_default();
    let description = def.description().and_then(|d| {
        d.string_value()
            .map(|sv| sv.source_string().trim_matches('"').to_string())
    });
    let directives = extract_directives(def.directives());

    ScalarTypeInfo {
        name,
        description,
        directives,
    }
}

fn extract_directives(directives: Option<cst::Directives>) -> Vec<DirectiveInfo> {
    directives
        .map(|ds| {
            ds.directives()
                .map(|d| DirectiveInfo {
                    name: d.name().map(|n| n.text().to_string()).unwrap_or_default(),
                    arguments: d
                        .arguments()
                        .map(|args| {
                            args.arguments()
                                .filter_map(|a| {
                                    let name = a.name()?.text().to_string();
                                    let value =
                                        a.value().map(|v| v.source_string()).unwrap_or_default();
                                    Some((name, value))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_input_type() {
        let sdl = r#"
            input Widget {
                name: String!
                count: Int
                tag: String
            }
        "#;

        let ast = extract_ast(sdl).unwrap();
        assert_eq!(ast.inputs.len(), 1);
        assert_eq!(ast.inputs[0].name, "Widget");
        assert_eq!(ast.inputs[0].fields.len(), 3);
        assert_eq!(ast.inputs[0].fields[0].name, "name");
        assert!(ast.inputs[0].fields[0].required);
        assert!(!ast.inputs[0].fields[1].required);
    }

    #[test]
    fn parse_enum_type() {
        let sdl = r#"
            enum Color {
                red
                green
                blue
            }
        "#;

        let ast = extract_ast(sdl).unwrap();
        assert_eq!(ast.enums.len(), 1);
        assert_eq!(ast.enums[0].name, "Color");
        assert_eq!(ast.enums[0].values, vec!["red", "green", "blue"]);
    }

    #[test]
    fn parse_union_type() {
        let sdl = r#"
            union Step @oneOf = AgentStep | ToolStep
        "#;

        let ast = extract_ast(sdl).unwrap();
        assert_eq!(ast.unions.len(), 1);
        assert_eq!(ast.unions[0].name, "Step");
        assert_eq!(ast.unions[0].members, vec!["AgentStep", "ToolStep"]);
        assert!(ast.unions[0].directives.iter().any(|d| d.name == "oneOf"));
    }

    #[test]
    fn parse_directives() {
        let sdl = r#"
            input Doc @closed {
                name: String! @pattern(regex: "^[a-z]+$")
            }
        "#;

        let ast = extract_ast(sdl).unwrap();
        assert!(ast.inputs[0].directives.iter().any(|d| d.name == "closed"));
        assert!(ast.inputs[0].fields[0]
            .directives
            .iter()
            .any(|d| d.name == "pattern"));

        let pattern_dir = ast.inputs[0].fields[0]
            .directives
            .iter()
            .find(|d| d.name == "pattern")
            .unwrap();
        assert!(pattern_dir
            .arguments
            .iter()
            .any(|(k, v)| k == "regex" && v.contains("[a-z]+")));
    }

    #[test]
    fn report_parse_errors() {
        let sdl = "input { broken";
        let result = extract_ast(sdl);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_with_descriptions() {
        let sdl = r#"
            """A configurable widget"""
            input Widget {
                """Display name"""
                name: String!
            }
        "#;

        let ast = extract_ast(sdl).unwrap();
        assert!(ast.inputs[0].description.is_some());
        assert!(ast.inputs[0].fields[0].description.is_some());
    }
}
