use darling::FromDeriveInput;
use syn::Path;

#[derive(Debug, Default, FromDeriveInput)]
#[darling(attributes(eure), default)]
pub struct ContainerAttrs {
    #[darling(rename = "crate")]
    pub crate_path: Option<Path>,
}
