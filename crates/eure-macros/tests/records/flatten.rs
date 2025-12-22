use eure::ParseDocument;

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct Address {
    city: String,
    country: String,
}

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct Person {
    name: String,
    #[eure(flatten)]
    address: Address,
}

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct ContactInfo {
    email: String,
    phone: String,
}

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct FullProfile {
    id: i32,
    #[eure(flatten)]
    address: Address,
    #[eure(flatten)]
    contact: ContactInfo,
}

#[test]
fn test_flatten_basic() {
    use eure::eure;
    let doc = eure!({ name = "Alice", city = "Tokyo", country = "Japan" });
    assert_eq!(
        doc.parse::<Person>(doc.get_root_id()).unwrap(),
        Person {
            name: "Alice".to_string(),
            address: Address {
                city: "Tokyo".to_string(),
                country: "Japan".to_string(),
            }
        }
    );
}

#[test]
fn test_flatten_multiple() {
    use eure::eure;
    let doc = eure!({
        id = 42,
        city = "New York",
        country = "USA",
        email = "test@example.com",
        phone = "123-456-7890"
    });
    assert_eq!(
        doc.parse::<FullProfile>(doc.get_root_id()).unwrap(),
        FullProfile {
            id: 42,
            address: Address {
                city: "New York".to_string(),
                country: "USA".to_string(),
            },
            contact: ContactInfo {
                email: "test@example.com".to_string(),
                phone: "123-456-7890".to_string(),
            }
        }
    );
}

#[test]
fn test_flatten_unknown_field_error() {
    use eure::eure;
    // Unknown field should be detected by root parser
    let doc = eure!({ name = "Alice", city = "Tokyo", country = "Japan", extra = "field" });
    let result = doc.parse::<Person>(doc.get_root_id());
    assert!(result.is_err());
}

#[test]
fn test_flatten_missing_field_error() {
    use eure::eure;
    // Missing required field in flattened type
    let doc = eure!({ name = "Alice", city = "Tokyo" }); // missing country
    let result = doc.parse::<Person>(doc.get_root_id());
    assert!(result.is_err());
}
