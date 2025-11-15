use eure_schema::{extract_schema_from_value, validate_with_schema_value};
use eure_value::{identifier::Identifier, value::ObjectKey};
use std::str::FromStr;

#[test]
fn test_meta_extensions_treated_uniformly() {
    // Test that $$type, $$optional are handled the same as any custom meta-extension
    let schema_input = r#"
# Define schemas for extensions using meta-extensions
$$type = .string
$$optional = .boolean
$$custom = .number
$$another.$type = .string

# Regular schema definitions
@ config
name.$type = .string
"#;

    let extracted = extract_schema_from_value(schema_input).expect("Failed to extract schema");

    // Should be detected as pure schema
    assert!(extracted.is_pure_schema);

    // Should have the config field
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::String("config".to_string()))
    );
}

#[test]
fn test_meta_extension_schema_extraction() {
    // Test that $$foo.$type = .string means $foo extension has string type
    let schema_input = r#"
# Meta-extension defines schema for $rename extension
$$rename = .string
$$rename.$optional = true

# Another meta-extension with type
$$priority.$type = .string
$$priority.$length = (1, 10)

# Use the extensions in regular schema
@ field
name.$type = .string
name.$rename = "userName"  # This should be valid string
"#;

    let extracted = extract_schema_from_value(schema_input).expect("Failed to extract schema");

    // Verify it's a pure schema
    assert!(extracted.is_pure_schema);

    // The document schema should contain the field, not the meta-extensions
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::String("field".to_string()))
    );

    // Meta-extensions should appear in document schema as they define schemas for extensions
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::MetaExtension(
                Identifier::from_str("rename").unwrap()
            ))
    );
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::MetaExtension(
                Identifier::from_str("priority").unwrap()
            ))
    );
}

#[test]
fn test_meta_extensions_not_special_cased() {
    // Ensure $$type and $$optional are not treated specially compared to other meta-extensions
    let schema_input = r#"
# All these meta-extensions should be handled identically
$$type = .string
$$optional = .boolean
$$array = .path
$$mycustom = .number
$$anothercustom.$type = .string

# Regular schema using extensions
@ person
name.$type = .string
age.$type = .number
age.$optional = true
"#;

    let extracted = extract_schema_from_value(schema_input).expect("Failed to extract schema");

    // Should extract the person schema
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::String("person".to_string()))
    );

    // Meta-extensions should appear in the document schema
    // Check that we have the expected meta-extensions
    let meta_extension_count = extracted
        .document_schema
        .root
        .fields
        .keys()
        .filter(|key| matches!(key, ObjectKey::MetaExtension(_)))
        .count();
    assert!(
        meta_extension_count > 0,
        "Should have meta-extensions in document schema"
    );
}

#[test]
fn test_path_with_meta_extension() {
    // Test that documents can contain path values with meta-extensions
    // and that meta-extensions are properly handled in schema
    let schema_input = r#"
# Define schema for fields that will hold path values
@ paths
path1.$type = .path
path2.$type = .path
path3.$type = .path

# Meta-extensions define schemas for extensions
$$myext.$type = .string
$$another = .boolean

# Regular field
@ config
name.$type = .string
"#;

    let extracted = extract_schema_from_value(schema_input).expect("Failed to extract schema");

    // This is a pure schema
    assert!(extracted.is_pure_schema);

    // The schema fields should be present
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::String("paths".to_string()))
    );
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::String("config".to_string()))
    );

    // Meta-extensions should not appear in document schema
    assert!(
        !extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::MetaExtension(
                Identifier::from_str("myext").unwrap()
            ))
    );
    assert!(
        !extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::MetaExtension(
                Identifier::from_str("another").unwrap()
            ))
    );

    // Now test a document with path values containing meta-extensions
    let doc_input = r#"
@ paths
path1 = .$$type.something
path2 = .$$optional
path3 = .$$custom.nested.path

@ config
name = "test"
"#;

    // The document should validate - path values with meta-extensions are valid
    let errors = validate_with_schema_value(doc_input, extracted.document_schema)
        .expect("Failed to validate");

    assert_eq!(
        errors.len(),
        0,
        "Document with meta-extension paths should validate"
    );
}

#[test]
fn test_meta_extension_with_variants() {
    // Test meta-extensions with complex types like variants
    let schema_input = r#"
# Meta-extension with variant type
$$mode {
  @ $variants.dev {
    debug.$type = .boolean
  }
  @ $variants.prod {
    optimize.$type = .boolean
  }
}

# Regular schema
@ config
name.$type = .string
"#;

    let extracted = extract_schema_from_value(schema_input).expect("Failed to extract schema");

    // Should have config but not $$mode
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::String("config".to_string()))
    );
    assert!(
        !extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::MetaExtension(
                Identifier::from_str("mode").unwrap()
            ))
    );
}

#[test]
fn test_meta_extension_creates_extension_schema() {
    // Test that $$optional = .boolean creates Extension("optional") -> boolean schema
    let schema_input = r#"
# Meta-extensions define schemas for extensions
$$optional = .boolean
$$type = .string
$$custom.field = .number

# Regular field schema
@ person
name.$type = .string
"#;

    let extracted = extract_schema_from_value(schema_input).expect("Failed to extract schema");

    // Check that extension schemas were created
    let optional_schema = extracted
        .document_schema
        .root
        .fields
        .get(&ObjectKey::MetaExtension(
            eure_value::identifier::Identifier::from_str("optional").unwrap(),
        ))
        .expect("Should have schema for $optional extension");

    assert_eq!(optional_schema.type_expr, eure_schema::Type::Boolean);

    let type_schema = extracted
        .document_schema
        .root
        .fields
        .get(&ObjectKey::MetaExtension(
            eure_value::identifier::Identifier::from_str("type").unwrap(),
        ))
        .expect("Should have schema for $type extension");

    assert_eq!(type_schema.type_expr, eure_schema::Type::String);

    // Meta-extension with nested field should create object schema
    let custom_schema = extracted
        .document_schema
        .root
        .fields
        .get(&ObjectKey::MetaExtension(
            eure_value::identifier::Identifier::from_str("custom").unwrap(),
        ))
        .expect("Should have schema for $custom extension");

    match &custom_schema.type_expr {
        eure_schema::Type::Object(obj) => {
            let field_schema = obj
                .fields
                .get(&ObjectKey::String("field".to_string()))
                .expect("Should have field in custom extension");
            assert_eq!(field_schema.type_expr, eure_schema::Type::Number);
        }
        _ => panic!("$custom extension should have object type"),
    }

    // Regular field should still work
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::String("person".to_string()))
    );
}

#[test]
fn test_document_with_extension_values() {
    // Test a document that uses extensions defined by meta-extensions
    let schema_doc = r#"
# Define extension schemas
$$rename = .string
$$rename.$optional = true
$$rename-all.$union = [.string]
$$rename-all.$optional = true

# Document schema
@ user
name.$type = .string
name.$rename = "userName"
"#;

    let test_doc = r#"
$schema = "./test.schema.eure"

@ user
name = "John Doe"
"#;

    // Extract schema
    let schema = extract_schema_from_value(schema_doc).expect("Failed to extract schema");

    // Validate document - the $serde.rename extension should be allowed
    let errors =
        validate_with_schema_value(test_doc, schema.document_schema).expect("Failed to validate");

    // Debug: print errors if any
    if !errors.is_empty() {
        println!("Validation errors:");
        for error in &errors {
            println!("  - {error:?}");
        }
    }

    // Should validate successfully
    assert_eq!(errors.len(), 0, "Document should validate against schema");
}
