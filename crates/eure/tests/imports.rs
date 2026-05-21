//! End-to-end tests for cross-schema `$import` resolution through the query
//! system.
//!
//! Test schemas are registered directly as `TextFile` assets. The query layer
//! resolves import paths to `TextFile` keys without reading from the filesystem.

use std::path::PathBuf;

use eure::query::{
    DocumentToSchemaQuery, OpenDocuments, OpenDocumentsList, TextFile, TextFileContent,
    ValidateAgainstExplicitSchema, Workspace, WorkspaceId, build_runtime,
};
use eure::report::{AnnotationKind, Element};
use query_flow::DurabilityLevel;
use url::Url;

const COMMON_SCHEMA: &str = r#"
= `$types.user`

$schema = "../../../../../assets/schemas/eure-schema.schema.eure"

$export = ["username", "email"]

@ $types.username {
  $variant: text
  pattern = `^[a-z0-9_]+$`
}

@ $types.email = `text.email`

@ $types.user {
  name = `$types.username`
  email = `$types.email`
}
"#;

const PROFILE_SCHEMA: &str = r#"
= `$types.profile`

$schema = "../../../../../assets/schemas/eure-schema.schema.eure"

$import = {
  common => "common.schema.eure"
}

@ $types.profile {
  name = `$types.common.username`
  contact = `$types.common.email`
}
"#;

const PROFILE_DATA: &str = r#"
$schema = "profile.schema.eure"

name = `ada_lovelace`
contact = email`ada@example.com`
"#;

const CYCLE_A_SCHEMA: &str = r#"
= `text`

$schema = "../../../../../assets/schemas/eure-schema.schema.eure"

$import = {
  b => "cycle-b.schema.eure"
}
"#;

const CYCLE_B_SCHEMA: &str = r#"
= `text`

$schema = "../../../../../assets/schemas/eure-schema.schema.eure"

$import = {
  a => "cycle-a.schema.eure"
}
"#;

fn virtual_import_path(name: &str) -> PathBuf {
    PathBuf::from("$virtual/imports").join(name)
}

fn register_file_content(
    runtime: &query_flow::QueryRuntime,
    path: PathBuf,
    content: &str,
) -> TextFile {
    let file = TextFile::from_path(path);
    runtime.resolve_asset(
        file.clone(),
        TextFileContent(content.to_string()),
        DurabilityLevel::Static,
    );
    file
}

fn register_virtual(runtime: &query_flow::QueryRuntime, name: &str, content: &str) -> TextFile {
    register_file_content(runtime, virtual_import_path(name), content)
}

#[test]
fn import_resolves_from_virtual_text_file_assets_without_filesystem() {
    let runtime = build_runtime();
    let common = register_file_content(
        &runtime,
        PathBuf::from("$virtual/imports/common.schema.eure"),
        r#"
$types.User = `text`
"#,
    );
    let profile = register_file_content(
        &runtime,
        PathBuf::from("$virtual/imports/profile.schema.eure"),
        r#"
$import = { common => "./common.schema.eure" }
name = `$types.common.User`
"#,
    );
    runtime.resolve_asset(
        OpenDocuments,
        OpenDocumentsList(vec![common, profile.clone()]),
        DurabilityLevel::Volatile,
    );

    let validated = runtime
        .query(DocumentToSchemaQuery::new(profile))
        .expect("schema converts from virtual TextFile assets");
    let common_alias: eure_document::identifier::Identifier = "common".parse().unwrap();
    assert!(validated.schema.imports.contains_key(&common_alias));
}

#[test]
fn https_import_uses_text_file_locator_security_allowlist() {
    let runtime = build_runtime();
    let workspace_path = PathBuf::from("/workspace");
    let config_path = workspace_path.join("Eure.eure");
    runtime.resolve_asset(
        WorkspaceId("test".to_string()),
        Workspace {
            path: workspace_path.clone(),
            config_path: config_path.clone(),
        },
        DurabilityLevel::Static,
    );
    runtime.resolve_asset(
        TextFile::from_path(config_path),
        TextFileContent(
            r#"
security.allowed-hosts = ["schemas.example.com"]
"#
            .to_string(),
        ),
        DurabilityLevel::Static,
    );

    let common =
        TextFile::from_url(Url::parse("https://schemas.example.com/common.schema.eure").unwrap());
    runtime.resolve_asset(
        common.clone(),
        TextFileContent("$types.User = `text`\n".to_string()),
        DurabilityLevel::Static,
    );
    let profile = register_file_content(
        &runtime,
        workspace_path.join("profile.schema.eure"),
        r#"
$import = { common => "https://schemas.example.com/common.schema.eure" }
name = `$types.common.User`
"#,
    );

    runtime.resolve_asset(
        OpenDocuments,
        OpenDocumentsList(vec![common, profile.clone()]),
        DurabilityLevel::Volatile,
    );

    let validated = runtime
        .query(DocumentToSchemaQuery::new(profile))
        .expect("https schema import converts when host is allowed");
    let common_alias: eure_document::identifier::Identifier = "common".parse().unwrap();
    assert!(validated.schema.imports.contains_key(&common_alias));
}

#[test]
fn import_resolves_via_query_system() {
    let runtime = build_runtime();
    let common = register_virtual(&runtime, "common.schema.eure", COMMON_SCHEMA);
    let profile = register_virtual(&runtime, "profile.schema.eure", PROFILE_SCHEMA);
    runtime.resolve_asset(
        OpenDocuments,
        OpenDocumentsList(vec![common, profile.clone()]),
        DurabilityLevel::Volatile,
    );

    // Loading `profile.schema.eure` must trigger import resolution against
    // `common.schema.eure`.
    let validated = runtime
        .query(DocumentToSchemaQuery::new(profile.clone()))
        .expect("schema converts");
    let schema = &validated.schema;

    let common_alias: eure_document::identifier::Identifier = "common".parse().unwrap();
    let username: eure_document::identifier::Identifier = "username".parse().unwrap();
    let email: eure_document::identifier::Identifier = "email".parse().unwrap();
    let imported = schema.imports.get(&common_alias).expect("common import");
    assert!(imported.all_types.contains_key(&username));
    assert!(imported.all_types.contains_key(&email));
    let flat: eure_document::identifier::Identifier = "common__username".parse().unwrap();
    assert!(!schema.types.contains_key(&flat));
}

#[test]
fn import_file_is_loaded_through_pending_assets() {
    let runtime = build_runtime();
    let profile = register_virtual(&runtime, "profile.schema.eure", PROFILE_SCHEMA);
    runtime.resolve_asset(
        OpenDocuments,
        OpenDocumentsList(vec![profile.clone()]),
        DurabilityLevel::Volatile,
    );

    let first = runtime.query(DocumentToSchemaQuery::new(profile.clone()));
    assert!(matches!(first, Err(query_flow::QueryError::Suspend { .. })));
    let common = TextFile::from_path(virtual_import_path("common.schema.eure"));
    runtime.resolve_asset(
        common,
        TextFileContent(COMMON_SCHEMA.to_string()),
        DurabilityLevel::Static,
    );

    let validated = runtime
        .query(DocumentToSchemaQuery::new(profile))
        .expect("schema converts after pending import is loaded");
    let common_alias: eure_document::identifier::Identifier = "common".parse().unwrap();
    assert!(validated.schema.imports.contains_key(&common_alias));
}

#[test]
fn data_validates_through_imported_types() {
    let runtime = build_runtime();
    let common = register_virtual(&runtime, "common.schema.eure", COMMON_SCHEMA);
    let profile = register_virtual(&runtime, "profile.schema.eure", PROFILE_SCHEMA);
    let data = register_virtual(&runtime, "profile-data.eure", PROFILE_DATA);
    runtime.resolve_asset(
        OpenDocuments,
        OpenDocumentsList(vec![common, profile.clone(), data.clone()]),
        DurabilityLevel::Volatile,
    );

    let reports = runtime
        .query(ValidateAgainstExplicitSchema::new(data, profile))
        .expect("validation runs");
    let reports: &eure::report::ErrorReports = reports.as_ref();
    assert!(
        reports.is_empty(),
        "expected no validation errors, got {} reports: {:#?}",
        reports.len(),
        reports.iter().map(|r| &r.title).collect::<Vec<_>>()
    );
}

#[test]
fn imported_type_validation_points_to_import_source() {
    let runtime = build_runtime();
    let common = register_virtual(&runtime, "common.schema.eure", COMMON_SCHEMA);
    let profile = register_virtual(&runtime, "profile.schema.eure", PROFILE_SCHEMA);
    let data = register_file_content(
        &runtime,
        virtual_import_path("invalid-profile.eure"),
        r#"
$schema = "profile.schema.eure"
name = `Ada!`
contact = email`ada@example.com`
"#,
    );
    runtime.resolve_asset(
        OpenDocuments,
        OpenDocumentsList(vec![common.clone(), profile.clone(), data.clone()]),
        DurabilityLevel::Volatile,
    );

    let reports = runtime
        .query(ValidateAgainstExplicitSchema::new(data, profile))
        .expect("validation runs");
    let reports: &eure::report::ErrorReports = reports.as_ref();
    assert!(!reports.is_empty(), "expected validation error");
    let has_common_schema_annotation = reports.iter().any(|report| {
        report.elements.iter().any(|element| {
            matches!(
                element,
                Element::Annotation {
                    origin,
                    kind: AnnotationKind::Secondary,
                    ..
                } if origin.file == common
            )
        })
    });
    assert!(
        has_common_schema_annotation,
        "expected secondary annotation in imported common schema, got {reports:#?}"
    );
}

#[test]
fn import_cycle_is_reported() {
    let runtime = build_runtime();
    let a = register_virtual(&runtime, "cycle-a.schema.eure", CYCLE_A_SCHEMA);
    let b = register_virtual(&runtime, "cycle-b.schema.eure", CYCLE_B_SCHEMA);
    runtime.resolve_asset(
        OpenDocuments,
        OpenDocumentsList(vec![a.clone(), b]),
        DurabilityLevel::Volatile,
    );

    // Loading either side of the cycle should fail. We can't `expect_err`
    // because `ValidatedSchema` is not `Debug`, so we match directly.
    let result = runtime.query(DocumentToSchemaQuery::new(a));
    let err = match result {
        Ok(_) => panic!("expected ImportCycle error"),
        Err(e) => e,
    };
    let msg = format!("{err}");
    assert!(
        msg.contains("import cycle") || msg.contains("ImportCycle"),
        "expected ImportCycle in error, got {msg}"
    );
}

#[test]
fn remote_import_host_not_allowed_is_rejected() {
    let runtime = build_runtime();
    let schema = register_file_content(
        &runtime,
        virtual_import_path("remote-root.schema.eure"),
        r#"
= `text`
$import = { common => "https://example.com/common.schema.eure" }
"#,
    );

    let result = runtime.query(DocumentToSchemaQuery::new(schema));
    let err = match result {
        Ok(_) => panic!("expected remote import to be rejected"),
        Err(e) => e,
    };
    let msg = format!("{err}");
    assert!(msg.contains("https"), "expected https rejection, got {msg}");
}

#[test]
fn absolute_import_is_rejected() {
    let runtime = build_runtime();
    let schema = register_file_content(
        &runtime,
        virtual_import_path("absolute-root.schema.eure"),
        r#"
= `text`
$import = { common => "/outside/common.schema.eure" }
"#,
    );

    let result = runtime.query(DocumentToSchemaQuery::new(schema));
    let err = match result {
        Ok(_) => panic!("expected absolute import to be rejected"),
        Err(e) => e,
    };
    let msg = format!("{err}");
    assert!(
        msg.contains("absolute schema import"),
        "expected absolute import rejection, got {msg}"
    );
}

#[test]
fn import_cannot_escape_root_schema_directory_without_workspace() {
    let runtime = build_runtime();
    let schema = register_file_content(
        &runtime,
        PathBuf::from("/workspace/root/root.schema.eure"),
        r#"
= `text`
$import = { common => "../outside.schema.eure" }
"#,
    );

    let result = runtime.query(DocumentToSchemaQuery::new(schema));
    let err = match result {
        Ok(_) => panic!("expected escaping import to be rejected"),
        Err(e) => e,
    };
    let msg = format!("{err}");
    assert!(
        msg.contains("escapes workspace root"),
        "expected boundary rejection, got {msg}"
    );
}
