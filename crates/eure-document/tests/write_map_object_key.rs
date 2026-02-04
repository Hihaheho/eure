use eure_document::document::constructor::DocumentConstructor;
use eure_document::map::Map;
use eure_document::value::ObjectKey;

#[test]
fn write_map_with_object_key() {
    let mut map: Map<ObjectKey, i32> = Map::new();
    map.insert(ObjectKey::String("alpha".to_string()), 1);
    map.insert(2.into(), 2);

    let mut c = DocumentConstructor::new();
    c.write(map).unwrap();
    let doc = c.finish();

    let root_map = doc.root().as_map().unwrap();
    assert_eq!(root_map.len(), 2);
}
