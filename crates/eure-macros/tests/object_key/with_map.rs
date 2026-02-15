use eure::ObjectKey;
use eure::{FromEure, IntoEure};
use eure_document::map::Map;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, ObjectKey)]
#[eure(crate = ::eure::document)]
enum Color {
    Red,
    Green,
    Blue,
}

#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document)]
struct Palette {
    colors: Map<Color, i32>,
}

#[test]
fn test_parse_map_with_custom_key() {
    use eure::eure;
    let doc = eure!({
        colors {
            Red = 1
            Green = 2
            Blue = 3
        }
    });
    let palette = doc.parse::<Palette>(doc.get_root_id()).unwrap();

    assert_eq!(palette.colors.get(&Color::Red), Some(&1));
    assert_eq!(palette.colors.get(&Color::Green), Some(&2));
    assert_eq!(palette.colors.get(&Color::Blue), Some(&3));
}

#[test]
fn test_write_map_with_custom_key() {
    use eure_document::document::constructor::DocumentConstructor;

    let mut colors = Map::new();
    colors.insert(Color::Red, 1);
    colors.insert(Color::Green, 2);
    colors.insert(Color::Blue, 3);

    let palette = Palette { colors };
    let mut c = DocumentConstructor::new();
    c.write(palette).unwrap();
    let doc = c.finish();

    let parsed = doc.parse::<Palette>(doc.get_root_id()).unwrap();
    assert_eq!(parsed.colors.get(&Color::Red), Some(&1));
    assert_eq!(parsed.colors.get(&Color::Green), Some(&2));
    assert_eq!(parsed.colors.get(&Color::Blue), Some(&3));
}
