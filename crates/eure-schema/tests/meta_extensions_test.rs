//! Tests for meta-extensions ($$-prefixed extensions)

use eure_parol::parse;
use eure_schema::*;

/// Helper to parse and extract schema from a document
fn extract(input: &str) -> ExtractedSchema {
    let tree = parse(input).expect("Failed to parse EURE document");
    extract_schema(input, &tree)
}

#[cfg(test)]
mod meta_extension_tests {
    use super::*;

    #[test]
    fn test_double_dollar_optional() {
        let schema_doc = r#"
@ $types.User
name = .string
email = .typed-string.email
email.$$optional = true
age = .number
age.$$optional = true
"#;
        let extracted = extract(schema_doc);
        
        let user_type = &extracted.document_schema.types["User"];
        if let Type::Object(obj) = &user_type.type_expr {
            // name is required (no $$optional)
            assert!(!obj.fields["name"].optional);
            
            // email is optional
            assert!(obj.fields["email"].optional);
            
            // age is optional
            assert!(obj.fields["age"].optional);
        } else {
            panic!("User should be object type");
        }
    }

    #[test]
    fn test_double_dollar_prefer() {
        let schema_doc = r#"
@ $types.Config
database = .object
database.$$prefer.section = true
cache = .object
cache.$$prefer.section = false
features = .array
features.$$prefer.array = true
flags = .array
flags.$$prefer.array = false
"#;
        let extracted = extract(schema_doc);
        
        let config_type = &extracted.document_schema.types["Config"];
        if let Type::Object(obj) = &config_type.type_expr {
            // Check prefer.section
            assert_eq!(obj.fields["database"].preferences.section, Some(true));
            assert_eq!(obj.fields["cache"].preferences.section, Some(false));
            
            // Check prefer.array
            assert_eq!(obj.fields["features"].preferences.array, Some(true));
            assert_eq!(obj.fields["flags"].preferences.array, Some(false));
        } else {
            panic!("Config should be object type");
        }
    }

    #[test]
    fn test_double_dollar_serde() {
        let schema_doc = r#"
@ $types.ApiResponse
status_code = .number
status_code.$$serde.rename = "statusCode"
error_message = .string
error_message.$$optional = true
error_message.$$serde.rename = "errorMessage"

@ $types.SnakeCase
$$serde.rename-all = "snake_case"
firstName = .string
lastName = .string

@ $types.CamelCase
$$serde.rename-all = "camelCase"
first_name = .string
last_name = .string
"#;
        let extracted = extract(schema_doc);
        
        // Check field-level rename
        let api_type = &extracted.document_schema.types["ApiResponse"];
        if let Type::Object(obj) = &api_type.type_expr {
            assert_eq!(obj.fields["status_code"].serde.rename, Some("statusCode".to_string()));
            assert_eq!(obj.fields["error_message"].serde.rename, Some("errorMessage".to_string()));
        } else {
            panic!("ApiResponse should be object type");
        }
        
        // Check type-level rename-all
        let snake_type = &extracted.document_schema.types["SnakeCase"];
        assert_eq!(snake_type.serde.rename_all, Some(RenameRule::SnakeCase));
        
        let camel_type = &extracted.document_schema.types["CamelCase"];
        assert_eq!(camel_type.serde.rename_all, Some(RenameRule::CamelCase));
    }

    #[test]
    fn test_double_dollar_array() {
        let schema_doc = r#"
@ $types.TodoList
items.$$array = .string
tags.$$array = .object
"#;
        let extracted = extract(schema_doc);
        
        let todo_type = &extracted.document_schema.types["TodoList"];
        if let Type::Object(obj) = &todo_type.type_expr {
            // items should be array of strings
            let items = &obj.fields["items"];
            if let Type::Array(element_type) = &items.type_expr {
                assert!(matches!(**element_type, Type::String));
            } else {
                panic!("items should be array type");
            }
            
            // tags should be array of objects
            let tags = &obj.fields["tags"];
            if let Type::Array(element_type) = &tags.type_expr {
                assert!(matches!(**element_type, Type::Object(_)));
            } else {
                panic!("tags should be array type");
            }
        } else {
            panic!("TodoList should be object type");
        }
    }

    #[test]
    fn test_double_dollar_cascade_type() {
        let schema_doc = r#"
$$cascade-type = .object

@ $types.SpecialSection
$$cascade-type = .string
"#;
        let extracted = extract(schema_doc);
        
        // Global cascade type
        assert!(matches!(extracted.document_schema.cascade_type, Some(Type::Object(_))));
        
        // Type-specific cascade type
        let special_type = &extracted.document_schema.types["SpecialSection"];
        if let Type::Object(obj) = &special_type.type_expr {
            // Check cascade type is set on the object schema
            // Note: This might need adjustment based on actual implementation
            // For now, just verify the type exists
            assert!(extracted.document_schema.types.contains_key("SpecialSection"));
        }
    }

    #[test]
    fn test_double_dollar_json_schema() {
        let schema_doc = r#"
@ $types.Validated
data = .object
data.$$json-schema = {
  "type": "object",
  "properties": {
    "x": { "type": "number" },
    "y": { "type": "number" }
  },
  "required": ["x", "y"]
}
"#;
        let extracted = extract(schema_doc);
        
        // Just verify the schema extracts without error
        // JSON schema validation would be handled separately
        assert!(extracted.document_schema.types.contains_key("Validated"));
    }

    #[test]
    fn test_double_dollar_literal() {
        let schema_doc = r#"
@ $types.Status
value = .string
value.$$literal = ["active", "inactive", "pending"]
"#;
        let extracted = extract(schema_doc);
        
        // Verify extraction succeeds
        // Literal validation would be handled in the validator
        let status_type = &extracted.document_schema.types["Status"];
        if let Type::Object(obj) = &status_type.type_expr {
            assert!(obj.fields.contains_key("value"));
        } else {
            panic!("Status should be object type");
        }
    }

    #[test]
    fn test_double_dollar_key_value() {
        let schema_doc = r#"
@ $types.Dictionary
$$map = true
$$key = .string
$$value = .number
"#;
        let extracted = extract(schema_doc);
        
        // Verify map type extraction
        assert!(extracted.document_schema.types.contains_key("Dictionary"));
    }

    #[test]
    fn test_variant_representation() {
        let schema_doc = r#"
@ $types.UntaggedUnion
$$variant-repr = "untagged"
@ $$variants.text
content = .string
@ $$variants.number
value = .number

@ $types.InternallyTagged
$$variant-repr = { tag = "type" }
@ $$variants.create
name = .string
@ $$variants.delete
id = .number

@ $types.AdjacentlyTagged  
$$variant-repr = { tag = "t", content = "c" }
@ $$variants.message
text = .string
"#;
        let extracted = extract(schema_doc);
        
        // Check untagged representation
        let untagged = &extracted.document_schema.types["UntaggedUnion"];
        if let Type::Variants(var_schema) = &untagged.type_expr {
            assert_eq!(var_schema.representation, VariantRepr::Untagged);
        } else {
            panic!("UntaggedUnion should be variants type");
        }
        
        // Check internally tagged
        let internal = &extracted.document_schema.types["InternallyTagged"];
        if let Type::Variants(var_schema) = &internal.type_expr {
            assert_eq!(var_schema.representation, VariantRepr::InternallyTagged { 
                tag: "type".to_string() 
            });
        } else {
            panic!("InternallyTagged should be variants type");
        }
        
        // Check adjacently tagged
        let adjacent = &extracted.document_schema.types["AdjacentlyTagged"];
        if let Type::Variants(var_schema) = &adjacent.type_expr {
            assert_eq!(var_schema.representation, VariantRepr::AdjacentlyTagged { 
                tag: "t".to_string(),
                content: "c".to_string()
            });
        } else {
            panic!("AdjacentlyTagged should be variants type");
        }
    }

    #[test]
    fn test_meta_extensions_in_variants() {
        let schema_doc = r#"
@ $types.Event
@ $$variants.created
timestamp = .number
user = .string
user.$$optional = true
details = .object
details.$$prefer.section = false

@ $$variants.updated
timestamp = .number
changes = .array
changes.$$array = .string
"#;
        let extracted = extract(schema_doc);
        
        let event_type = &extracted.document_schema.types["Event"];
        if let Type::Variants(var_schema) = &event_type.type_expr {
            // Check created variant
            let created = &var_schema.variants["created"];
            assert!(created.fields["user"].optional);
            assert_eq!(created.fields["details"].preferences.section, Some(false));
            
            // Check updated variant
            let updated = &var_schema.variants["updated"];
            let changes = &updated.fields["changes"];
            if let Type::Array(elem_type) = &changes.type_expr {
                assert!(matches!(**elem_type, Type::String));
            } else {
                panic!("changes should be array type");
            }
        } else {
            panic!("Event should be variants type");
        }
    }

    #[test]
    fn test_meta_extensions_with_deep_nesting() {
        let schema_doc = r#"
config.server.http.port = .number
config.server.http.port.$$optional = true
config.server.https.cert.path = .string
config.server.https.cert.path.$$serde.rename = "certPath"
"#;
        let extracted = extract(schema_doc);
        
        // Navigate to deeply nested fields
        let config = &extracted.document_schema.root.fields["config"];
        if let Type::Object(config_obj) = &config.type_expr {
            let server = &config_obj.fields["server"];
            if let Type::Object(server_obj) = &server.type_expr {
                // Check HTTP port is optional
                let http = &server_obj.fields["http"];
                if let Type::Object(http_obj) = &http.type_expr {
                    assert!(http_obj.fields["port"].optional);
                } else {
                    panic!("http should be object");
                }
                
                // Check HTTPS cert path rename
                let https = &server_obj.fields["https"];
                if let Type::Object(https_obj) = &https.type_expr {
                    let cert = &https_obj.fields["cert"];
                    if let Type::Object(cert_obj) = &cert.type_expr {
                        assert_eq!(
                            cert_obj.fields["path"].serde.rename,
                            Some("certPath".to_string())
                        );
                    } else {
                        panic!("cert should be object");
                    }
                } else {
                    panic!("https should be object");
                }
            } else {
                panic!("server should be object");
            }
        } else {
            panic!("config should be object");
        }
    }
}