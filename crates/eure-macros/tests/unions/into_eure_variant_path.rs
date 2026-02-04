use eure::{FromEure, IntoEure};

#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document)]
enum Direction {
    Left(f32),
    Right(f32),
}

#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document)]
enum Role {
    Normal { name: String },
    Admin { name: String },
}

#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document)]
enum Outer {
    Move(Direction),
    User(Role),
}

#[test]
fn test_into_eure_sets_variant_for_newtype_union() {
    use eure::document::constructor::DocumentConstructor;
    use eure::document::write::IntoEure;
    use eure::eure;

    let mut c = DocumentConstructor::new();
    Direction::write(Direction::Right(1.5_f32), &mut c).unwrap();
    let doc = c.finish();

    let expected = eure!({ %variant = "Right", = 1.5_f32 });
    assert_eq!(doc, expected);

    let parsed: Direction = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(parsed, Direction::Right(1.5_f32));
}

#[test]
fn test_into_eure_sets_variant_for_struct_union() {
    use eure::document::constructor::DocumentConstructor;
    use eure::document::write::IntoEure;
    use eure::eure;

    let mut c = DocumentConstructor::new();
    Role::write(
        Role::Admin {
            name: "root".to_string(),
        },
        &mut c,
    )
    .unwrap();
    let doc = c.finish();

    let expected = eure!({ %variant = "Admin", name = "root" });
    assert_eq!(doc, expected);

    let parsed: Role = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        parsed,
        Role::Admin {
            name: "root".to_string(),
        }
    );
}

#[test]
fn test_into_eure_sets_nested_variant_path_newtype() {
    use eure::document::constructor::DocumentConstructor;
    use eure::document::write::IntoEure;
    use eure::eure;

    let mut c = DocumentConstructor::new();
    Outer::write(Outer::Move(Direction::Right(1.5_f32)), &mut c).unwrap();
    let doc = c.finish();

    let expected = eure!({ %variant = "Move.Right", = 1.5_f32 });
    assert_eq!(doc, expected);

    let parsed: Outer = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(parsed, Outer::Move(Direction::Right(1.5_f32)));
}

#[test]
fn test_into_eure_sets_nested_variant_path_struct() {
    use eure::document::constructor::DocumentConstructor;
    use eure::document::write::IntoEure;
    use eure::eure;

    let mut c = DocumentConstructor::new();
    Outer::write(
        Outer::User(Role::Admin {
            name: "root".to_string(),
        }),
        &mut c,
    )
    .unwrap();
    let doc = c.finish();

    let expected = eure!({ %variant = "User.Admin", name = "root" });
    assert_eq!(doc, expected);

    let parsed: Outer = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        parsed,
        Outer::User(Role::Admin {
            name: "root".to_string(),
        })
    );
}
