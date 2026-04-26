use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use eure::document::{EureDocument, parse_to_document};
use eure_document::plan::LayoutPlan;
use serde_json::{Value as JsonValue, json};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "eure-cli-schema-aware-json-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create temp dir");
        Self { path }
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, contents).expect("write temp file");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_eure(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_eure"))
        .args(args)
        .output()
        .expect("run eure")
}

fn path_arg(path: &Path) -> String {
    path.to_str().expect("utf-8 temp path").to_string()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is utf-8")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "expected failure\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn format_eure_document(doc: &EureDocument) -> String {
    let plan = LayoutPlan::auto(doc.clone()).expect("layout plan");
    eure_fmt::format_source_document(&plan.emit())
}

fn assert_schema_roundtrip(
    schema_source: &str,
    expected_json: JsonValue,
    expected_doc: EureDocument,
) {
    let dir = TempDir::new();
    let schema = dir.write("schema.eure", schema_source);
    let input_eure = dir.write("input.eure", &format_eure_document(&expected_doc));
    let input_json = dir.write(
        "input.json",
        &serde_json::to_string_pretty(&expected_json).expect("serialize expected json"),
    );

    let schema = path_arg(&schema);
    let input_eure = path_arg(&input_eure);
    let output = run_eure(&["to-json", &input_eure, "--schema", &schema, "--pretty"]);

    assert_success(&output);
    let actual_json: JsonValue = serde_json::from_str(&stdout(&output)).expect("json stdout");
    assert_eq!(actual_json, expected_json);

    let input_json = path_arg(&input_json);
    let output = run_eure(&["from-json", &input_json, "--schema", &schema]);

    assert_success(&output);
    let actual_doc =
        parse_to_document(&stdout(&output), "<from-json stdout>").expect("parse from-json stdout");
    assert_eq!(actual_doc, expected_doc);
}

fn internal_union_schema() -> &'static str {
    r#"
$variant = "union"
$interop."variant-repr".tag = "type"
variants.success {
  message = `text`
}
variants.failure {
  code = `integer`
}
"#
}

fn external_union_schema() -> &'static str {
    r#"
$variant = "union"
$interop."variant-repr" = "external"
variants.success {
  message = `text`
}
variants.failure {
  code = `integer`
}
"#
}

fn adjacent_union_schema() -> &'static str {
    r#"
$variant = "union"
$interop."variant-repr".tag = "t"
$interop."variant-repr".content = "c"
variants.ok {
  value = `text`
}
variants.err = `null`
"#
}

fn untagged_union_schema() -> &'static str {
    r#"
$variant = "union"
$interop."variant-repr" = "untagged"
variants.text = `text`
variants.count = `integer`
"#
}

#[test]
fn internal_variant_encodes_and_decodes_with_tag_field() {
    assert_schema_roundtrip(
        internal_union_schema(),
        json!({
            "type": "success",
            "message": "ok",
        }),
        eure::eure!({ message = "ok", %variant = "success" }),
    );
}

#[test]
fn external_variant_encodes_and_decodes_with_wrapper_object() {
    assert_schema_roundtrip(
        external_union_schema(),
        json!({
            "success": {
                "message": "ok",
            },
        }),
        eure::eure!({ message = "ok", %variant = "success" }),
    );
}

#[test]
fn untagged_variant_encodes_and_decodes_without_tag_data() {
    assert_schema_roundtrip(
        untagged_union_schema(),
        json!("hello"),
        eure::eure!({ = "hello", %variant = "text" }),
    );
}

#[test]
fn adjacent_unit_variant_encodes_and_decodes_without_content_field() {
    assert_schema_roundtrip(
        adjacent_union_schema(),
        json!({ "t": "err" }),
        eure::eure!({ = null, %variant = "err" }),
    );
}

#[test]
fn schema_free_variant_option_still_controls_to_json() {
    let dir = TempDir::new();
    let input = dir.write(
        "input.eure",
        r#"
value = 42
$variant = "ok"
"#,
    );

    let input = path_arg(&input);
    let output = run_eure(&[
        "to-json",
        &input,
        "--variant",
        "internal",
        "--tag",
        "kind",
        "--pretty",
    ]);

    assert_success(&output);
    let actual: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json stdout");
    assert_eq!(
        actual,
        json!({
            "kind": "ok",
            "value": 42,
        })
    );
}

#[test]
fn invalid_schema_path_exits_non_zero() {
    let dir = TempDir::new();
    let input = dir.write("input.eure", r#"message = "ok""#);
    let missing_schema = dir.path.join("missing.eure");

    let input = path_arg(&input);
    let missing_schema = path_arg(&missing_schema);
    let output = run_eure(&["to-json", &input, "--schema", &missing_schema]);

    assert_failure(&output);
    assert!(
        stderr(&output).contains("missing.eure"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn schema_aware_deserialization_error_exits_non_zero() {
    let dir = TempDir::new();
    let schema = dir.write("schema.eure", internal_union_schema());
    let input = dir.write(
        "input.json",
        r#"
{
  "type": "success",
  "message": 123
}
"#,
    );

    let schema = path_arg(&schema);
    let input = path_arg(&input);
    let output = run_eure(&["from-json", &input, "--schema", &schema]);

    assert_failure(&output);
    assert!(
        stderr(&output).contains("Schema-aware JSON deserialization failed"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn schema_aware_from_json_rejects_trailing_json() {
    let dir = TempDir::new();
    let schema = dir.write("schema.eure", r#"= `text`"#);
    let input = dir.write("input.json", r#""hello" "extra""#);

    let schema = path_arg(&schema);
    let input = path_arg(&input);
    let output = run_eure(&["from-json", &input, "--schema", &schema]);

    assert_failure(&output);
    assert!(
        stderr(&output).contains("Schema-aware JSON deserialization failed"),
        "{}",
        stderr(&output)
    );
}
