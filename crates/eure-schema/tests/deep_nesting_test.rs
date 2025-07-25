//! Tests for deep nesting support in schema definitions

use eure_schema::*;
use eure_value::value::KeyCmpValue;

/// Helper to parse and extract schema from a document
fn extract(input: &str) -> ExtractedSchema {
    extract_schema_from_value(input).expect("Failed to extract schema")
}

/// Helper to parse and validate with schema
fn validate(input: &str, schema: DocumentSchema) -> Vec<ValidationError> {
    validate_with_schema_value(input, schema).expect("Failed to validate")
}

/// Helper to validate self-describing document
fn validate_self(input: &str) -> ValidationResult {
    validate_self_describing(input).expect("Failed to validate self-describing document")
}

#[cfg(test)]
mod deep_nesting_tests {
    use super::*;

    #[test]
    fn test_three_level_inline_schema() {
        let doc = r#"
company.department.manager.$type = .string
company.department.budget.$type = .number
"#;
        let extracted = extract(doc);

        // Should extract nested structure
        assert!(
            extracted
                .document_schema
                .root
                .fields
                .contains_key(&KeyCmpValue::String("company".to_string()))
        );

        let company = &extracted
            .document_schema
            .root
            .fields
            .get(&KeyCmpValue::String("company".to_string()))
            .unwrap();
        if let Type::Object(obj) = &company.type_expr {
            assert!(
                obj.fields
                    .contains_key(&KeyCmpValue::String("department".to_string()))
            );

            let department = &obj
                .fields
                .get(&KeyCmpValue::String("department".to_string()))
                .unwrap();
            if let Type::Object(dept_obj) = &department.type_expr {
                assert!(
                    dept_obj
                        .fields
                        .contains_key(&KeyCmpValue::String("manager".to_string()))
                );
                assert!(
                    dept_obj
                        .fields
                        .contains_key(&KeyCmpValue::String("budget".to_string()))
                );

                let manager = &dept_obj
                    .fields
                    .get(&KeyCmpValue::String("manager".to_string()))
                    .unwrap();
                assert!(matches!(manager.type_expr, Type::String));

                let budget = &dept_obj
                    .fields
                    .get(&KeyCmpValue::String("budget".to_string()))
                    .unwrap();
                assert!(matches!(budget.type_expr, Type::Number));
            } else {
                panic!("department should be an object");
            }
        } else {
            panic!("company should be an object");
        }
    }

    #[test]
    fn test_four_level_inline_schema() {
        let doc = r#"
org.division.team.lead.name.$type = .string
org.division.team.lead.email.$type = .string
org.division.team.size.$type = .number
"#;
        let extracted = extract(doc);

        // Navigate through the structure
        let org = &extracted
            .document_schema
            .root
            .fields
            .get(&KeyCmpValue::String("org".to_string()))
            .unwrap();
        if let Type::Object(org_obj) = &org.type_expr {
            let division = &org_obj
                .fields
                .get(&KeyCmpValue::String("division".to_string()))
                .unwrap();
            if let Type::Object(div_obj) = &division.type_expr {
                let team = &div_obj
                    .fields
                    .get(&KeyCmpValue::String("team".to_string()))
                    .unwrap();
                if let Type::Object(team_obj) = &team.type_expr {
                    assert!(
                        team_obj
                            .fields
                            .contains_key(&KeyCmpValue::String("lead".to_string()))
                    );
                    assert!(
                        team_obj
                            .fields
                            .contains_key(&KeyCmpValue::String("size".to_string()))
                    );

                    let lead = &team_obj
                        .fields
                        .get(&KeyCmpValue::String("lead".to_string()))
                        .unwrap();
                    if let Type::Object(lead_obj) = &lead.type_expr {
                        assert!(
                            lead_obj
                                .fields
                                .contains_key(&KeyCmpValue::String("name".to_string()))
                        );
                        assert!(
                            lead_obj
                                .fields
                                .contains_key(&KeyCmpValue::String("email".to_string()))
                        );

                        let email = &lead_obj
                            .fields
                            .get(&KeyCmpValue::String("email".to_string()))
                            .unwrap();
                        assert_eq!(email.type_expr, Type::String);
                    } else {
                        panic!("lead should be an object");
                    }
                } else {
                    panic!("team should be an object");
                }
            } else {
                panic!("division should be an object");
            }
        } else {
            panic!("org should be an object");
        }
    }

    #[test]
    fn test_deep_nesting_with_constraints() {
        let doc = r#"
api.v1.endpoints.users.rateLimit.$type = .number
api.v1.endpoints.users.rateLimit.$range = (0, 1000)
api.v1.endpoints.users.path.$type = .string
api.v1.endpoints.users.path.$pattern = "^/api/v1/users.*$"
"#;
        let extracted = extract(doc);

        // Extract the deeply nested field
        let api = &extracted
            .document_schema
            .root
            .fields
            .get(&KeyCmpValue::String("api".to_string()))
            .unwrap();
        if let Type::Object(api_obj) = &api.type_expr {
            let v1 = &api_obj
                .fields
                .get(&KeyCmpValue::String("v1".to_string()))
                .unwrap();
            if let Type::Object(v1_obj) = &v1.type_expr {
                let endpoints = &v1_obj
                    .fields
                    .get(&KeyCmpValue::String("endpoints".to_string()))
                    .unwrap();
                if let Type::Object(ep_obj) = &endpoints.type_expr {
                    let users = &ep_obj
                        .fields
                        .get(&KeyCmpValue::String("users".to_string()))
                        .unwrap();
                    if let Type::Object(users_obj) = &users.type_expr {
                        // Check rateLimit constraints
                        let rate_limit = &users_obj
                            .fields
                            .get(&KeyCmpValue::String("rateLimit".to_string()))
                            .unwrap();
                        assert!(matches!(rate_limit.type_expr, Type::Number));
                        assert_eq!(
                            rate_limit.constraints.range,
                            Some((Some(0.0), Some(1000.0)))
                        );

                        // Check path constraints
                        let path = &users_obj
                            .fields
                            .get(&KeyCmpValue::String("path".to_string()))
                            .unwrap();
                        assert!(matches!(path.type_expr, Type::String));
                        assert!(path.constraints.pattern.is_some());
                    } else {
                        panic!("users should be an object");
                    }
                } else {
                    panic!("endpoints should be an object");
                }
            } else {
                panic!("v1 should be an object");
            }
        } else {
            panic!("api should be an object");
        }
    }

    #[test]
    fn test_deep_nesting_with_meta_extensions() {
        let doc = r#"
server.config.database.connection.timeout.$type = .number
server.config.database.connection.timeout.$optional = true
server.config.database.connection.host.$type = .string
server.config.database.connection.host.$prefer.section = false
"#;
        let extracted = extract(doc);

        // Navigate to connection object
        let server = &extracted
            .document_schema
            .root
            .fields
            .get(&KeyCmpValue::String("server".to_string()))
            .unwrap();
        if let Type::Object(server_obj) = &server.type_expr {
            let config = &server_obj
                .fields
                .get(&KeyCmpValue::String("config".to_string()))
                .unwrap();
            if let Type::Object(config_obj) = &config.type_expr {
                let database = &config_obj
                    .fields
                    .get(&KeyCmpValue::String("database".to_string()))
                    .unwrap();
                if let Type::Object(db_obj) = &database.type_expr {
                    let connection = &db_obj
                        .fields
                        .get(&KeyCmpValue::String("connection".to_string()))
                        .unwrap();
                    if let Type::Object(conn_obj) = &connection.type_expr {
                        // Check timeout is optional
                        let timeout = &conn_obj
                            .fields
                            .get(&KeyCmpValue::String("timeout".to_string()))
                            .unwrap();
                        assert!(timeout.optional);

                        // Check host preference
                        let host = &conn_obj
                            .fields
                            .get(&KeyCmpValue::String("host".to_string()))
                            .unwrap();
                        assert_eq!(host.preferences.section, Some(false));
                    } else {
                        panic!("connection should be an object");
                    }
                } else {
                    panic!("database should be an object");
                }
            } else {
                panic!("config should be an object");
            }
        } else {
            panic!("server should be an object");
        }
    }

    #[test]
    fn test_validation_with_deep_nesting() {
        // Create schema with deep nesting
        let schema_doc = r#"
app.services.auth.providers.oauth.clientId.$type = .string
app.services.auth.providers.oauth.clientSecret.$type = .string
app.services.auth.providers.oauth.redirectUrl.$type = .code.url
app.services.auth.providers.oauth.enabled.$type = .boolean
"#;
        let schema = extract(schema_doc).document_schema;

        // Debug: print schema structure
        eprintln!(
            "Schema root fields: {:?}",
            schema.root.fields.keys().collect::<Vec<_>>()
        );
        if let Some(app_field) = schema
            .root
            .fields
            .get(&KeyCmpValue::String("app".to_string()))
        {
            eprintln!("app field optional: {}", app_field.optional);
        }

        // Valid document
        let valid_doc = r#"
@ app.services.auth.providers.oauth
clientId = "my-client-id"
clientSecret = "my-secret"
redirectUrl = url`https://example.com/callback`
enabled = true
"#;
        let errors = validate(valid_doc, schema.clone());
        if !errors.is_empty() {
            eprintln!("Validation errors for valid_doc:");
            for error in &errors {
                eprintln!("  - {error:?}");
            }
        }
        assert!(errors.is_empty());

        // Invalid document - wrong type
        let invalid_doc = r#"
@ app.services.auth.providers.oauth
clientId = "my-client-id"
clientSecret = "my-secret"
redirectUrl = url`https://example.com/callback`
enabled = "yes"  # Should be boolean
"#;
        let errors = validate(invalid_doc, schema);
        if errors.len() != 1 {
            panic!("Expected exactly 1 error, got {}: {errors:?}", errors.len());
        }
        assert!(matches!(
            &errors[0].kind,
            ValidationErrorKind::TypeMismatch { expected, actual }
                if expected == "boolean" && actual == "string"
        ));
    }

    #[test]
    fn test_mixed_depth_inline_schemas() {
        // Test mixing different depths of inline schemas
        let doc = r#"
# Root level
name.$type = .string

# Two levels
person.age.$type = .number

# Three levels
company.info.founded.$type = .number
company.info.founded.$range = (1800, 2100)

# Four levels
system.modules.core.version.major.$type = .number
system.modules.core.version.minor.$type = .number
system.modules.core.version.patch.$type = .number

# Back to two levels
config.debug.$type = .boolean
"#;
        let result = validate_self(doc);

        // Check all fields were extracted correctly
        assert!(
            result
                .schema
                .document_schema
                .root
                .fields
                .contains_key(&KeyCmpValue::String("name".to_string()))
        );
        assert!(
            result
                .schema
                .document_schema
                .root
                .fields
                .contains_key(&KeyCmpValue::String("person".to_string()))
        );
        assert!(
            result
                .schema
                .document_schema
                .root
                .fields
                .contains_key(&KeyCmpValue::String("company".to_string()))
        );
        assert!(
            result
                .schema
                .document_schema
                .root
                .fields
                .contains_key(&KeyCmpValue::String("system".to_string()))
        );
        assert!(
            result
                .schema
                .document_schema
                .root
                .fields
                .contains_key(&KeyCmpValue::String("config".to_string()))
        );

        // Verify the system.modules.core.version structure
        let system = &result
            .schema
            .document_schema
            .root
            .fields
            .get(&KeyCmpValue::String("system".to_string()))
            .unwrap();
        if let Type::Object(sys_obj) = &system.type_expr {
            let modules = &sys_obj
                .fields
                .get(&KeyCmpValue::String("modules".to_string()))
                .unwrap();
            if let Type::Object(mod_obj) = &modules.type_expr {
                let core = &mod_obj
                    .fields
                    .get(&KeyCmpValue::String("core".to_string()))
                    .unwrap();
                if let Type::Object(core_obj) = &core.type_expr {
                    let version = &core_obj
                        .fields
                        .get(&KeyCmpValue::String("version".to_string()))
                        .unwrap();
                    if let Type::Object(ver_obj) = &version.type_expr {
                        assert!(
                            ver_obj
                                .fields
                                .contains_key(&KeyCmpValue::String("major".to_string()))
                        );
                        assert!(
                            ver_obj
                                .fields
                                .contains_key(&KeyCmpValue::String("minor".to_string()))
                        );
                        assert!(
                            ver_obj
                                .fields
                                .contains_key(&KeyCmpValue::String("patch".to_string()))
                        );
                    } else {
                        panic!("version should be an object");
                    }
                } else {
                    panic!("core should be an object");
                }
            } else {
                panic!("modules should be an object");
            }
        } else {
            panic!("system should be an object");
        }
    }

    #[test]
    fn test_type_reference_with_deep_nesting() {
        let schema_doc = r#"
@ $types.Version
major.$type = .number
minor.$type = .number
patch.$type = .number

# Deep reference to custom type - use nested section syntax
@ product.info.software.version
$type = .$types.Version
"#;
        let extracted = extract(schema_doc);

        // Check type was defined
        assert!(extracted.document_schema.types.contains_key(
            &eure_value::value::KeyCmpValue::String("Version".to_string())
        ));

        // Check deep reference was created
        let product = &extracted
            .document_schema
            .root
            .fields
            .get(&KeyCmpValue::String("product".to_string()))
            .unwrap();
        if let Type::Object(prod_obj) = &product.type_expr {
            let info = &prod_obj
                .fields
                .get(&KeyCmpValue::String("info".to_string()))
                .unwrap();
            if let Type::Object(info_obj) = &info.type_expr {
                let software = &info_obj
                    .fields
                    .get(&KeyCmpValue::String("software".to_string()))
                    .unwrap();
                if let Type::Object(soft_obj) = &software.type_expr {
                    let version = &soft_obj
                        .fields
                        .get(&KeyCmpValue::String("version".to_string()))
                        .unwrap();
                    if let Type::TypeRef(type_ref) = &version.type_expr {
                        assert_eq!(
                            type_ref.to_string(),
                            "Version"
                        );
                    } else {
                        panic!("version should be a type reference");
                    }
                } else {
                    panic!("software should be an object");
                }
            } else {
                panic!("info should be an object");
            }
        } else {
            panic!("product should be an object");
        }
    }

    #[test]
    fn test_inline_schema_in_section_with_deep_path() {
        let doc = r#"
@ company.departments.engineering
team.frontend.lead.$type = .string
team.frontend.members.$type = .array
team.backend.lead.$type = .string
team.backend.members.$type = .array
"#;
        let result = validate_self(doc);

        // The inline schemas inside the section should be applied to the section path
        let company = &result
            .schema
            .document_schema
            .root
            .fields
            .get(&KeyCmpValue::String("company".to_string()))
            .unwrap();
        if let Type::Object(company_obj) = &company.type_expr {
            let departments = &company_obj
                .fields
                .get(&KeyCmpValue::String("departments".to_string()))
                .unwrap();
            if let Type::Object(dept_obj) = &departments.type_expr {
                let engineering = &dept_obj
                    .fields
                    .get(&KeyCmpValue::String("engineering".to_string()))
                    .unwrap();
                if let Type::Object(eng_obj) = &engineering.type_expr {
                    let team = &eng_obj
                        .fields
                        .get(&KeyCmpValue::String("team".to_string()))
                        .unwrap();
                    if let Type::Object(team_obj) = &team.type_expr {
                        // Check frontend and backend teams
                        assert!(
                            team_obj
                                .fields
                                .contains_key(&KeyCmpValue::String("frontend".to_string()))
                        );
                        assert!(
                            team_obj
                                .fields
                                .contains_key(&KeyCmpValue::String("backend".to_string()))
                        );

                        let frontend = &team_obj
                            .fields
                            .get(&KeyCmpValue::String("frontend".to_string()))
                            .unwrap();
                        if let Type::Object(fe_obj) = &frontend.type_expr {
                            assert!(
                                fe_obj
                                    .fields
                                    .contains_key(&KeyCmpValue::String("lead".to_string()))
                            );
                            assert!(
                                fe_obj
                                    .fields
                                    .contains_key(&KeyCmpValue::String("members".to_string()))
                            );
                        } else {
                            panic!("frontend should be an object");
                        }
                    } else {
                        panic!("team should be an object");
                    }
                } else {
                    panic!("engineering should be an object");
                }
            } else {
                panic!("departments should be an object");
            }
        } else {
            panic!("company should be an object");
        }
    }
}
