use indexmap::IndexSet;

use eure_codegen_ir::{
    EmissionDefaultsIr, TypeEmissionConfigIr, effective_emission, filter_desired_derives,
};

#[test]
fn filters_derives_by_allow_list_and_serde_flags() {
    let defaults = EmissionDefaultsIr {
        serde_serialize: true,
        serde_deserialize: true,
        derive_allow: IndexSet::from([
            "Debug".to_string(),
            "Clone".to_string(),
            "Serialize".to_string(),
            "Deserialize".to_string(),
        ]),
    };
    let overrides = TypeEmissionConfigIr {
        serde_serialize: Some(false),
        serde_deserialize: Some(true),
        derive_allow: Some(IndexSet::from([
            "Debug".to_string(),
            "Serialize".to_string(),
            "Deserialize".to_string(),
        ])),
    };

    let desired = vec![
        "Debug".to_string(),
        "Clone".to_string(),
        "Serialize".to_string(),
        "Deserialize".to_string(),
    ];
    let derives = filter_desired_derives(&desired, &defaults, &overrides);

    assert_eq!(
        derives,
        vec!["Debug".to_string(), "Deserialize".to_string()]
    );
}

#[test]
fn effective_emission_applies_overrides_over_defaults() {
    let defaults = EmissionDefaultsIr {
        serde_serialize: false,
        serde_deserialize: false,
        derive_allow: IndexSet::from(["Debug".to_string()]),
    };
    let overrides = TypeEmissionConfigIr {
        serde_serialize: Some(true),
        serde_deserialize: Some(false),
        derive_allow: Some(IndexSet::from([
            "Debug".to_string(),
            "Serialize".to_string(),
        ])),
    };

    let effective = effective_emission(&defaults, &overrides);
    assert!(effective.serde_serialize);
    assert!(!effective.serde_deserialize);
    assert!(effective.derive_allow.contains("Serialize"));
}
