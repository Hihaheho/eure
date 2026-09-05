//! Definition requests across suspension, imports, and remote source display.
use eure::query::{TextFile, TextFileContent};
use eure_ls::{CoreRequestId, Effect, LspCore, LspOutput};
use query_flow::DurabilityLevel;
use serde_json::{Value, json};

fn open(core: &mut LspCore, uri: &str, text: &str) -> Vec<Effect> {
    core.handle_notification(
        "textDocument/didOpen",
        json!({"textDocument": {
            "uri": uri, "languageId": "eure", "version": 1, "text": text,
        }}),
    )
    .1
}

fn put(core: &mut LspCore, uri: &str, text: &str) {
    let file = if uri.starts_with("https:") {
        TextFile::parse(uri).unwrap()
    } else {
        TextFile::from_path(uri.strip_prefix("file://").unwrap().into())
    };
    core.runtime_mut().resolve_asset(
        file,
        TextFileContent(text.into()),
        DurabilityLevel::Volatile,
    );
}

fn request(
    core: &mut LspCore,
    uri: &str,
    line: u32,
    character: u32,
) -> (Vec<LspOutput>, Vec<Effect>) {
    core.handle_request(
        1.into(),
        "textDocument/definition",
        json!({
            "textDocument": {"uri": uri}, "position": {"line": line, "character": character},
        }),
    )
}

fn response(outputs: &[LspOutput]) -> Value {
    outputs
        .iter()
        .find_map(|output| match output {
            LspOutput::Response { id, result } if *id == CoreRequestId::Int(1) => {
                Some(result.clone().unwrap())
            }
            _ => None,
        })
        .expect("definition response")
}

#[test]
fn definition_waits_for_schema_and_selects_field_not_referenced_type() {
    let mut core = LspCore::new();
    let mut effects = open(
        &mut core,
        "file:///ws/doc.eure",
        "$schema = \"./doc.schema.eure\"\nname = \"x\"",
    );
    let (outputs, more) = request(&mut core, "file:///ws/doc.eure", 1, 2);
    assert!(outputs.is_empty());
    effects.extend(more);
    let file = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::FetchFile(file) if file.ends_with("doc.schema.eure") => Some(file.clone()),
            _ => None,
        })
        .unwrap();
    let (outputs, _) = core.resolve_file(
        file,
        Ok("name = `$types.name`\n@ $types.name = `text`".into()),
    );
    assert_eq!(
        response(&outputs),
        json!([{
            "uri": "file:///ws/doc.schema.eure",
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 4}},
        }])
    );
}

#[test]
fn imported_field_and_type_reference_use_original_source() {
    let mut core = LspCore::new();
    put(
        &mut core,
        "file:///ws/common.schema.eure",
        "@ $types.user {\n  name = `text`\n}",
    );
    let schema = "$import = { common => \"common.schema.eure\" }\nuser = `$types.common.user`";
    put(&mut core, "file:///ws/doc.schema.eure", schema);
    open(
        &mut core,
        "file:///ws/doc.eure",
        "$schema = \"./doc.schema.eure\"\nuser.name = \"x\"",
    );
    let result = response(&request(&mut core, "file:///ws/doc.eure", 1, 7).0);
    assert_eq!(result[0]["uri"], "file:///ws/common.schema.eure");
    assert_eq!(
        result[0]["range"]["start"],
        json!({"line": 1, "character": 2})
    );
    open(&mut core, "file:///ws/doc.schema.eure", schema);
    let result = response(&request(&mut core, "file:///ws/doc.schema.eure", 1, 18).0);
    assert_eq!(result[0]["uri"], "file:///ws/common.schema.eure");
    assert_eq!(result[0]["range"]["start"]["line"], 0);
}

#[test]
fn remote_schema_content_and_relative_imports_share_the_loaded_source() {
    let mut core = LspCore::new();
    let remote = "https://eure.dev/schemas/doc.schema.eure?version=1";
    let schema = "$import = { common => \"./common.schema.eure\" }\nuser = `$types.common.user`";
    put(&mut core, remote, schema);
    put(
        &mut core,
        "https://eure.dev/schemas/common.schema.eure",
        "@ $types.user { name = `text` }",
    );
    open(
        &mut core,
        "file:///ws/doc.eure",
        &format!("$schema = \"{remote}\"\nuser.name = \"x\""),
    );
    let result = response(&request(&mut core, "file:///ws/doc.eure", 1, 6).0);
    assert_eq!(
        result[0]["uri"],
        "https://eure.dev/schemas/common.schema.eure"
    );
    let (outputs, effects) =
        core.handle_request(1.into(), "eure/schemaContent", json!({"uri": remote}));
    assert!(effects.is_empty());
    assert_eq!(response(&outputs), schema);
    open(&mut core, remote, schema);
    let result = response(&request(&mut core, remote, 1, 18).0);
    assert_eq!(
        result[0]["uri"],
        "https://eure.dev/schemas/common.schema.eure"
    );
    core.close_document(remote);
    let (outputs, effects) =
        core.handle_request(1.into(), "eure/schemaContent", json!({"uri": remote}));
    assert!(effects.is_empty());
    assert_eq!(response(&outputs), schema);
}

#[test]
fn links_use_utf16_and_escaped_target_uri() {
    let mut core = LspCore::new();
    core.handle_request(
        0.into(),
        "initialize",
        json!({"capabilities": {
            "textDocument": {"definition": {"linkSupport": true}}
        }}),
    );
    put(
        &mut core,
        "file:///ws/my schema.eure",
        "\"😀\" = `text` name = `text`",
    );
    open(
        &mut core,
        "file:///ws/doc.eure",
        "$schema = \"./my schema.eure\"\n\"😀\" = \"x\" name = \"y\"",
    );
    let result = response(&request(&mut core, "file:///ws/doc.eure", 1, 14).0);
    assert_eq!(result[0]["targetUri"], "file:///ws/my%20schema.eure");
    assert_eq!(
        result[0]["targetSelectionRange"]["start"],
        json!({"line": 0, "character": 14})
    );
    assert_eq!(
        result[0]["originSelectionRange"]["start"],
        json!({"line": 1, "character": 11})
    );
}

#[test]
fn schema_reference_and_unknown_field() {
    let mut core = LspCore::new();
    put(&mut core, "file:///ws/s.eure", "name = `text`");
    open(
        &mut core,
        "file:///ws/doc.eure",
        "$schema = \"./s.eure\"\nunknown = 1",
    );
    assert_eq!(
        response(&request(&mut core, "file:///ws/doc.eure", 0, 15).0)[0]["uri"],
        "file:///ws/s.eure"
    );
    assert_eq!(
        response(&request(&mut core, "file:///ws/doc.eure", 1, 3).0),
        json!([])
    );
}

#[test]
fn union_candidates_and_array_elements_keep_field_definitions() {
    let mut core = LspCore::new();
    put(
        &mut core,
        "file:///ws/s.eure",
        "$types.action {\n  $variant: union\n  variants.say { line = `text` }\n  variants.shout { line = `text` }\n}\nactions = [`$types.action`]",
    );
    open(
        &mut core,
        "file:///ws/doc.eure",
        "$schema = \"s.eure\"\n@ actions[] {\n  line = \"hi\"\n}",
    );
    let result = response(&request(&mut core, "file:///ws/doc.eure", 2, 4).0);
    assert_eq!(result.as_array().unwrap().len(), 2);
    assert_eq!(result[0]["range"]["start"]["line"], 2);
    assert_eq!(result[1]["range"]["start"]["line"], 3);
    open(
        &mut core,
        "file:///ws/doc.eure",
        "$schema = \"s.eure\"\n@ actions[] {\n  $variant: say\n  line = \"hi\"\n}",
    );
    let result = response(&request(&mut core, "file:///ws/doc.eure", 3, 4).0);
    assert_eq!(result.as_array().unwrap().len(), 1);
    assert_eq!(result[0]["range"]["start"]["line"], 2);
    let result = response(&request(&mut core, "file:///ws/doc.eure", 2, 13).0);
    assert_eq!(result.as_array().unwrap().len(), 1);
    assert_eq!(result[0]["range"]["start"]["line"], 2);
}

#[test]
fn import_strings_and_unopened_remote_documents() {
    let mut core = LspCore::new();
    put(
        &mut core,
        "https://eure.dev/schemas/s.eure",
        "= `$types.common.name`\n$import = { common => \"./common.eure\" }",
    );
    put(
        &mut core,
        "https://eure.dev/schemas/common.eure",
        "@ $types.name = `text`",
    );
    // Definition converts positions against the loaded source without requiring didOpen.
    let result = response(&request(&mut core, "https://eure.dev/schemas/s.eure", 0, 30).0);
    assert_eq!(result[0]["uri"], "https://eure.dev/schemas/common.eure");
}

#[test]
fn partial_document_can_navigate_an_existing_key() {
    let mut core = LspCore::new();
    put(&mut core, "file:///ws/s.eure", "name = `text`");
    open(
        &mut core,
        "file:///ws/doc.eure",
        "$schema = \"s.eure\"\nname = \n@ ",
    );
    let result = response(&request(&mut core, "file:///ws/doc.eure", 1, 2).0);
    assert_eq!(result[0]["uri"], "file:///ws/s.eure");
}

#[test]
fn failed_download_answers_the_pending_request_with_an_error() {
    let mut core = LspCore::new();
    open(
        &mut core,
        "file:///ws/doc.eure",
        "$schema = \"https://eure.dev/missing.eure\"\nname = \"x\"",
    );
    assert!(request(&mut core, "file:///ws/doc.eure", 1, 2).0.is_empty());
    let (outputs, _) = core.resolve_file(
        TextFile::parse("https://eure.dev/missing.eure").unwrap(),
        Err("HTTP 404".to_string()),
    );
    assert!(outputs.iter().any(|output| matches!(
        output,
        LspOutput::Response {
            id: CoreRequestId::Int(1),
            result: Err(_)
        }
    )));
}
