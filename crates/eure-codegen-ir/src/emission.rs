use indexmap::IndexSet;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EmissionDefaultsIr {
    pub serde_serialize: bool,
    pub serde_deserialize: bool,
    pub derive_allow: IndexSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeEmissionConfigIr {
    pub serde_serialize: Option<bool>,
    pub serde_deserialize: Option<bool>,
    pub derive_allow: Option<IndexSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EffectiveEmissionIr {
    pub serde_serialize: bool,
    pub serde_deserialize: bool,
    pub derive_allow: IndexSet<String>,
}

pub fn effective_emission(
    defaults: &EmissionDefaultsIr,
    ty: &TypeEmissionConfigIr,
) -> EffectiveEmissionIr {
    EffectiveEmissionIr {
        serde_serialize: ty.serde_serialize.unwrap_or(defaults.serde_serialize),
        serde_deserialize: ty.serde_deserialize.unwrap_or(defaults.serde_deserialize),
        derive_allow: ty
            .derive_allow
            .clone()
            .unwrap_or_else(|| defaults.derive_allow.clone()),
    }
}

pub fn filter_desired_derives(
    desired: &[String],
    defaults: &EmissionDefaultsIr,
    ty: &TypeEmissionConfigIr,
) -> Vec<String> {
    let effective = effective_emission(defaults, ty);

    desired
        .iter()
        .filter(|derive_name| effective.derive_allow.contains(*derive_name))
        .filter(|derive_name| {
            (derive_name.as_str() != "Serialize" || effective.serde_serialize)
                && (derive_name.as_str() != "Deserialize" || effective.serde_deserialize)
        })
        .cloned()
        .collect()
}
