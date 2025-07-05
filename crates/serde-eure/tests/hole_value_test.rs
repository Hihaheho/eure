use serde_eure::from_str;

#[test]
fn test_deserialize_hole_value_error() {
    // Test that holes cause deserialization errors
    let input = r#"
        name = "John"
        age = !
    "#;

    #[derive(Debug, serde::Deserialize)]
    struct Person {
        name: String,
        age: u32,
    }

    let result = from_str::<Person>(input);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Hole") || error.to_string().contains("hole"), "Error was: {error}");
}

#[test]
fn test_deserialize_hole_in_array() {
    let input = r#"
        items = ["first", !, "third"]
    "#;

    #[derive(Debug, serde::Deserialize)]
    struct Container {
        items: Vec<String>,
    }

    let result = from_str::<Container>(input);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Hole") || error.to_string().contains("hole"), "Error was: {error}");
}

#[test]
fn test_deserialize_nested_hole() {
    let input = r#"
        person = {
            name = "John"
            address = {
                street = !
                city = "New York"
            }
        }
    "#;

    #[derive(Debug, serde::Deserialize)]
    struct Address {
        street: String,
        city: String,
    }

    #[derive(Debug, serde::Deserialize)]
    struct Person {
        name: String,
        address: Address,
    }

    #[derive(Debug, serde::Deserialize)]
    struct Container {
        person: Person,
    }

    let result = from_str::<Container>(input);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Hole") || error.to_string().contains("hole"), "Error was: {error}");
}

#[test]
fn test_parse_file_with_holes() {
    // Verify that files with holes can be parsed, but fail at deserialization
    let input = r#"
        # Configuration file with incomplete values
        database = {
            host = "localhost"
            port = !  # TODO: Set the port
            username = "admin"
            password = !  # TODO: Set password before deployment
        }
        
        features = {
            logging = true
            monitoring = !  # TODO: Decide on monitoring
        }
    "#;

    // The parse should succeed
    let tree = eure_parol::parse(input).expect("Failed to parse");
    
    // Value extraction should succeed (holes become Value::Hole)
    let mut values = eure_tree::value_visitor::Values::default();
    let mut visitor = eure_tree::value_visitor::ValueVisitor::new(input, &mut values);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    
    // But deserialization to a concrete type should fail
    #[derive(Debug, serde::Deserialize)]
    struct Database {
        host: String,
        port: u16,
        username: String,
        password: String,
    }
    
    #[derive(Debug, serde::Deserialize)]
    struct Features {
        logging: bool,
        monitoring: bool,
    }
    
    #[derive(Debug, serde::Deserialize)]
    struct Config {
        database: Database,
        features: Features,
    }
    
    let result = from_str::<Config>(input);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Hole") || error.to_string().contains("hole"), "Error was: {error}");
}