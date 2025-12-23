use eure::ParseDocument;

fn default_timeout() -> i32 {
    30
}

fn default_name() -> String {
    "anonymous".to_string()
}

// Basic struct with Default trait
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct ConfigWithDefault {
    name: String,
    #[eure(default)]
    enabled: bool,
    #[eure(default)]
    count: i32,
}

// Struct with custom default functions
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct ConfigWithCustomDefault {
    #[eure(default = "default_name")]
    name: String,
    #[eure(default = "default_timeout")]
    timeout: i32,
}

// Struct with ext field and default
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct ConfigWithExtDefault {
    name: String,
    #[eure(ext, default)]
    deprecated: bool,
}

// Struct with ext field and custom default
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct ConfigWithExtCustomDefault {
    name: String,
    #[eure(ext, default = "default_timeout")]
    timeout: i32,
}

// parse_ext context with default
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, parse_ext)]
struct ExtFieldsWithDefault {
    required: bool,
    #[eure(default)]
    optional: bool,
}

// parse_ext context with custom default
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, parse_ext)]
struct ExtFieldsWithCustomDefault {
    #[eure(default = "default_timeout")]
    timeout: i32,
}

#[test]
fn test_default_trait_all_present() {
    use eure::eure;
    let doc = eure!({ name = "Alice", enabled = true, count = 5 });
    assert_eq!(
        doc.parse::<ConfigWithDefault>(doc.get_root_id()).unwrap(),
        ConfigWithDefault {
            name: "Alice".to_string(),
            enabled: true,
            count: 5,
        }
    );
}

#[test]
fn test_default_trait_missing() {
    use eure::eure;
    let doc = eure!({ name = "Alice" });
    assert_eq!(
        doc.parse::<ConfigWithDefault>(doc.get_root_id()).unwrap(),
        ConfigWithDefault {
            name: "Alice".to_string(),
            enabled: false, // Default::default() for bool
            count: 0,       // Default::default() for i32
        }
    );
}

#[test]
fn test_default_trait_partial() {
    use eure::eure;
    let doc = eure!({ name = "Alice", enabled = true });
    assert_eq!(
        doc.parse::<ConfigWithDefault>(doc.get_root_id()).unwrap(),
        ConfigWithDefault {
            name: "Alice".to_string(),
            enabled: true,
            count: 0, // Default::default() for i32
        }
    );
}

#[test]
fn test_custom_default_all_present() {
    use eure::eure;
    let doc = eure!({ name = "Bob", timeout = 60 });
    assert_eq!(
        doc.parse::<ConfigWithCustomDefault>(doc.get_root_id()).unwrap(),
        ConfigWithCustomDefault {
            name: "Bob".to_string(),
            timeout: 60,
        }
    );
}

#[test]
fn test_custom_default_missing() {
    use eure::eure;
    let doc = eure!({});
    assert_eq!(
        doc.parse::<ConfigWithCustomDefault>(doc.get_root_id()).unwrap(),
        ConfigWithCustomDefault {
            name: "anonymous".to_string(), // from default_name()
            timeout: 30,                   // from default_timeout()
        }
    );
}

#[test]
fn test_custom_default_partial() {
    use eure::eure;
    let doc = eure!({ name = "Charlie" });
    assert_eq!(
        doc.parse::<ConfigWithCustomDefault>(doc.get_root_id()).unwrap(),
        ConfigWithCustomDefault {
            name: "Charlie".to_string(),
            timeout: 30, // from default_timeout()
        }
    );
}

#[test]
fn test_ext_field_with_default_present() {
    use eure::eure;
    let doc = eure!({ name = "Alice", %deprecated = true });
    assert_eq!(
        doc.parse::<ConfigWithExtDefault>(doc.get_root_id()).unwrap(),
        ConfigWithExtDefault {
            name: "Alice".to_string(),
            deprecated: true,
        }
    );
}

#[test]
fn test_ext_field_with_default_missing() {
    use eure::eure;
    let doc = eure!({ name = "Alice" });
    assert_eq!(
        doc.parse::<ConfigWithExtDefault>(doc.get_root_id()).unwrap(),
        ConfigWithExtDefault {
            name: "Alice".to_string(),
            deprecated: false, // Default::default() for bool
        }
    );
}

#[test]
fn test_ext_field_with_custom_default_present() {
    use eure::eure;
    let doc = eure!({ name = "Alice", %timeout = 120 });
    assert_eq!(
        doc.parse::<ConfigWithExtCustomDefault>(doc.get_root_id()).unwrap(),
        ConfigWithExtCustomDefault {
            name: "Alice".to_string(),
            timeout: 120,
        }
    );
}

#[test]
fn test_ext_field_with_custom_default_missing() {
    use eure::eure;
    let doc = eure!({ name = "Alice" });
    assert_eq!(
        doc.parse::<ConfigWithExtCustomDefault>(doc.get_root_id()).unwrap(),
        ConfigWithExtCustomDefault {
            name: "Alice".to_string(),
            timeout: 30, // from default_timeout()
        }
    );
}

#[test]
fn test_parse_ext_context_default_present() {
    use eure::document::node::NodeValue;
    use eure::document::EureDocument;
    use eure::value::PrimitiveValue;

    let mut doc = EureDocument::new();
    let root_id = doc.get_root_id();

    // Add required extension
    let req_id = doc
        .add_extension("required".parse().unwrap(), root_id)
        .unwrap()
        .node_id;
    doc.node_mut(req_id).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

    // Add optional extension
    let opt_id = doc
        .add_extension("optional".parse().unwrap(), root_id)
        .unwrap()
        .node_id;
    doc.node_mut(opt_id).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

    let ctx = doc.parse_extension_context(root_id);
    let result: ExtFieldsWithDefault = ctx.parse().unwrap();
    assert_eq!(
        result,
        ExtFieldsWithDefault {
            required: true,
            optional: true
        }
    );
}

#[test]
fn test_parse_ext_context_default_missing() {
    use eure::document::node::NodeValue;
    use eure::document::EureDocument;
    use eure::value::PrimitiveValue;

    let mut doc = EureDocument::new();
    let root_id = doc.get_root_id();

    // Only add required extension
    let req_id = doc
        .add_extension("required".parse().unwrap(), root_id)
        .unwrap()
        .node_id;
    doc.node_mut(req_id).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

    let ctx = doc.parse_extension_context(root_id);
    let result: ExtFieldsWithDefault = ctx.parse().unwrap();
    assert_eq!(
        result,
        ExtFieldsWithDefault {
            required: true,
            optional: false // Default::default() for bool
        }
    );
}

#[test]
fn test_parse_ext_context_custom_default_missing() {
    use eure::document::EureDocument;

    let doc = EureDocument::new();
    let root_id = doc.get_root_id();

    let ctx = doc.parse_extension_context(root_id);
    let result: ExtFieldsWithCustomDefault = ctx.parse().unwrap();
    assert_eq!(
        result,
        ExtFieldsWithCustomDefault {
            timeout: 30 // from default_timeout()
        }
    );
}
