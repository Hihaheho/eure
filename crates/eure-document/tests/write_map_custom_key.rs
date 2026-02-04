use eure_document::document::constructor::DocumentConstructor;
use eure_document::map::Map;
use eure_document::value::ObjectKey;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CustomKey(&'static str);

impl From<CustomKey> for ObjectKey {
    fn from(key: CustomKey) -> Self {
        ObjectKey::String(key.0.to_string())
    }
}

#[test]
fn write_map_with_custom_key() {
    let mut map: Map<CustomKey, i32> = Map::new();
    map.insert(CustomKey("beta"), 2);

    let mut c = DocumentConstructor::new();
    c.write(map).unwrap();
    let doc = c.finish();

    let root_map = doc.root().as_map().unwrap();
    assert_eq!(root_map.len(), 1);
}
