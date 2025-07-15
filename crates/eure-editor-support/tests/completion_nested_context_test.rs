use eure_editor_support::completions::get_completions;
use eure_editor_support::schema_validation::SchemaManager;
use lsp_types::Position;

#[test]
fn test_completion_in_nested_section_should_suggest_nested_fields() {
    // Schema with nested structure
    let schema_text = r#"
@ script
$type = .string

@ name
$type = .string

@ database {
    @ host
    $type = .string
    
    @ port
    $type = .number
    
    @ credentials {
        @ username
        $type = .string
        
        @ password
        $type = .string
        
        @ api_key
        $type = .string
    }
}

@ logging {
    @ level
    $type = .string
    
    @ file
    $type = .string
}
"#;

    // User is typing inside the database.credentials section
    let input = r#"@ database.credentials
username = "admin"
"#;

    // Position is at beginning of empty line 3 where we want field name completions
    let position = Position {
        line: 2,
        character: 0,
    };

    let parse_result = eure_parol::parse_tolerant(input);
    let cst = parse_result.cst();

    // Create schema manager with schema
    let mut schema_manager = SchemaManager::new();
    let schema_parse_result = eure_parol::parse_tolerant(schema_text);
    let schema_cst = schema_parse_result.cst();

    schema_manager
        .load_schema("test://schema", schema_text, &schema_cst)
        .unwrap();
    schema_manager.set_document_schema("test.eure", "test://schema");

    let completions = get_completions(input, &cst, position, None, "test.eure", &schema_manager, None);

    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    println!("Completions in credentials section: {labels:?}");

    // SHOULD suggest fields from database.credentials context
    assert!(
        labels.contains(&"password".to_string()),
        "Should suggest 'password' from credentials context"
    );
    assert!(
        labels.contains(&"api_key".to_string()),
        "Should suggest 'api_key' from credentials context"
    );

    // SHOULD NOT suggest root-level fields
    assert!(
        !labels.contains(&"script".to_string()),
        "Should NOT suggest root-level 'script' field"
    );
    assert!(
        !labels.contains(&"name".to_string()),
        "Should NOT suggest root-level 'name' field"
    );
    assert!(
        !labels.contains(&"database".to_string()),
        "Should NOT suggest root-level 'database' field"
    );
    assert!(
        !labels.contains(&"logging".to_string()),
        "Should NOT suggest root-level 'logging' field"
    );

    // SHOULD NOT suggest fields from parent database context
    assert!(
        !labels.contains(&"host".to_string()),
        "Should NOT suggest 'host' from parent database context"
    );
    assert!(
        !labels.contains(&"port".to_string()),
        "Should NOT suggest 'port' from parent database context"
    );

    // SHOULD NOT suggest already used fields
    assert!(
        !labels.contains(&"username".to_string()),
        "Should NOT suggest already used 'username' field"
    );
}

#[test]
fn test_completion_in_deeply_nested_array_element() {
    // Schema with arrays containing nested objects
    let schema_text = r#"
@ users
$array {
    @ name
    $type = .string
    
    @ email
    $type = .string
    
    @ roles
    $array {
        @ role_name
        $type = .string
        
        @ permissions
        $array = .string
        
        @ metadata {
            @ created_at
            $type = .string
            
            @ expires_at
            $type = .string
        }
    }
}

@ settings {
    @ theme
    $type = .string
}
"#;

    // User is typing inside a role's metadata section within a user array
    let input = r#"
@ users[]
name = "Alice"

@ users[].roles[]
role_name = "admin"

@ users[].roles[].metadata
created_at = "2024-01-01"
"#;

    // Position is at the end where we want to suggest expires_at field
    let position = Position {
        line: 8,
        character: 0,
    };

    let parse_result = eure_parol::parse_tolerant(input);
    let cst = parse_result.cst();

    let mut schema_manager = SchemaManager::new();
    let schema_parse_result = eure_parol::parse_tolerant(schema_text);
    let schema_cst = schema_parse_result.cst();

    schema_manager
        .load_schema("test://schema", schema_text, &schema_cst)
        .unwrap();
    schema_manager.set_document_schema("test.eure", "test://schema");

    let completions = get_completions(input, &cst, position, None, "test.eure", &schema_manager, None);

    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    println!("Completions in role metadata: {labels:?}");

    // SHOULD suggest fields from roles[].metadata context
    assert!(
        labels.contains(&"expires_at".to_string()),
        "Should suggest 'expires_at' from metadata context"
    );

    // SHOULD NOT suggest fields from other contexts
    assert!(
        !labels.contains(&"role_name".to_string()),
        "Should NOT suggest 'role_name' from parent role context"
    );
    assert!(
        !labels.contains(&"permissions".to_string()),
        "Should NOT suggest 'permissions' from parent role context"
    );
    assert!(
        !labels.contains(&"name".to_string()),
        "Should NOT suggest 'name' from user context"
    );
    assert!(
        !labels.contains(&"email".to_string()),
        "Should NOT suggest 'email' from user context"
    );
    assert!(
        !labels.contains(&"settings".to_string()),
        "Should NOT suggest root-level 'settings'"
    );
    assert!(
        !labels.contains(&"theme".to_string()),
        "Should NOT suggest 'theme' from unrelated settings context"
    );
}

#[test]
fn test_completion_with_variants_in_nested_context() {
    // Schema with variants in nested structure
    let schema_text = r#"
@ $types {
    @ Status
    $variants {
        @ pending {
            @ reason
            $type = .string
        }
        @ approved {
            @ approved_by
            $type = .string
            
            @ approved_at
            $type = .string
        }
        @ rejected {
            @ rejected_by
            $type = .string
            
            @ rejection_reason
            $type = .string
        }
    }
}

@ requests
$array {
    @ id
    $type = .string
    
    @ status
    $type = .$types.Status
}
"#;

    // User is typing inside an approved variant
    let input = r#"
@ requests[]
id = 123

@ requests[].status
$variant: approved
approved_by = "manager"
"#;

    // Position is at the end where we want to suggest approved_at field
    let position = Position {
        line: 6,
        character: 0,
    };

    let parse_result = eure_parol::parse_tolerant(input);
    let cst = parse_result.cst();

    let mut schema_manager = SchemaManager::new();
    let schema_parse_result = eure_parol::parse_tolerant(schema_text);
    let schema_cst = schema_parse_result.cst();

    schema_manager
        .load_schema("test://schema", schema_text, &schema_cst)
        .unwrap();
    schema_manager.set_document_schema("test.eure", "test://schema");

    let completions = get_completions(input, &cst, position, None, "test.eure", &schema_manager, None);

    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    println!("Completions in approved variant: {labels:?}");

    // SHOULD suggest fields from the approved variant
    assert!(
        labels.contains(&"approved_at".to_string()),
        "Should suggest 'approved_at' from approved variant"
    );

    // SHOULD NOT suggest fields from other variants
    assert!(
        !labels.contains(&"reason".to_string()),
        "Should NOT suggest 'reason' from pending variant"
    );
    assert!(
        !labels.contains(&"rejected_by".to_string()),
        "Should NOT suggest 'rejected_by' from rejected variant"
    );
    assert!(
        !labels.contains(&"rejection_reason".to_string()),
        "Should NOT suggest 'rejection_reason' from rejected variant"
    );

    // SHOULD NOT suggest fields from parent contexts
    assert!(
        !labels.contains(&"id".to_string()),
        "Should NOT suggest 'id' from request context"
    );
    assert!(
        !labels.contains(&"status".to_string()),
        "Should NOT suggest 'status' from request context"
    );
}
