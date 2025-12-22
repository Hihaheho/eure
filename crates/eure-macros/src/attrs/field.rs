use darling::FromField;

#[derive(Debug, Default, FromField)]
#[darling(default, attributes(eure))]
pub struct FieldAttrs {
    pub flatten: bool,
}
