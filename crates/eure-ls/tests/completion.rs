//! End-to-end check of `textDocument/completion` through `LspCore`, including
//! the suspend/resume path taken when the schema file has not been fetched yet.

use eure::query::TextFile;
use eure_ls::{CoreRequestId, Effect, LspCore, LspOutput};
use lsp_types::{
    CompletionParams, CompletionResponse, DidOpenTextDocumentParams, Position,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams,
    notification::{DidOpenTextDocument, Notification as _},
    request::{Completion, Request as _},
};
use serde_json::Value;

const DOC_URI: &str = "file:///ws/doc.eure";
const SCHEMA_FILE_NAME: &str = "doc.schema.eure";

const SCHEMA: &str = r#"
name = `text`
port = `integer`
enabled = `boolean`
"#;

fn open(core: &mut LspCore, text: &str) -> Vec<Effect> {
    let params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: DOC_URI.parse().unwrap(),
            language_id: "eure".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    core.handle_notification(
        DidOpenTextDocument::METHOD,
        serde_json::to_value(params).unwrap(),
    )
    .1
}

/// The schema file the core asked the platform to fetch.
fn requested_schema_file(effects: &[Effect]) -> Option<TextFile> {
    effects.iter().find_map(|effect| match effect {
        Effect::FetchFile(file) if file.ends_with(SCHEMA_FILE_NAME) => Some(file.clone()),
        _ => None,
    })
}

fn request_completion(
    core: &mut LspCore,
    id: i32,
    position: Position,
) -> (Vec<LspOutput>, Vec<Effect>) {
    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: DOC_URI.parse().unwrap(),
            },
            position,
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };
    core.handle_request(
        CoreRequestId::from(id),
        Completion::METHOD,
        serde_json::to_value(params).unwrap(),
    )
}

fn labels(outputs: &[LspOutput], id: i32) -> Vec<String> {
    let value = outputs
        .iter()
        .find_map(|output| match output {
            LspOutput::Response { id: got, result } if *got == CoreRequestId::from(id) => {
                Some(result.clone().expect("completion succeeds"))
            }
            _ => None,
        })
        .expect("completion response");
    match serde_json::from_value::<CompletionResponse>(value).unwrap() {
        CompletionResponse::Array(items) => items.into_iter().map(|i| i.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|i| i.label).collect(),
    }
}

#[test]
fn completion_waits_for_schema_then_answers() {
    let mut core = LspCore::new();
    let open_effects = open(
        &mut core,
        "$schema = \"./doc.schema.eure\"\nname = \"x\"\n@ ",
    );

    // The schema is not loaded yet: the request suspends until the file
    // (already requested by diagnostics on open, or now) is resolved.
    let (outputs, effects) = request_completion(&mut core, 1, Position::new(2, 2));
    assert!(
        outputs
            .iter()
            .all(|o| !matches!(o, LspOutput::Response { .. })),
        "no response before the schema is fetched"
    );
    let schema_file = requested_schema_file(&open_effects)
        .or_else(|| requested_schema_file(&effects))
        .expect("schema fetch requested");

    // Resolving the schema file resumes the pending request.
    let (outputs, _) = core.resolve_file(schema_file, Ok(SCHEMA.to_string()));
    assert_eq!(labels(&outputs, 1), vec!["port", "enabled"]);
}

#[test]
fn completion_offers_values_and_respects_partial_keys() {
    let mut core = LspCore::new();
    let open_effects = open(
        &mut core,
        "$schema = \"./doc.schema.eure\"\nenabled = \n@ na",
    );
    let (_, effects) = request_completion(&mut core, 1, Position::new(1, 10));
    let schema_file = requested_schema_file(&open_effects)
        .or_else(|| requested_schema_file(&effects))
        .expect("schema fetch requested");
    let (outputs, _) = core.resolve_file(schema_file, Ok(SCHEMA.to_string()));
    assert_eq!(labels(&outputs, 1), vec!["true", "false"]);

    let (outputs, _) = request_completion(&mut core, 2, Position::new(2, 4));
    assert_eq!(labels(&outputs, 2), vec!["name"]);
    let response = outputs
        .iter()
        .find_map(|o| match o {
            LspOutput::Response { result: Ok(v), .. } => Some(v.clone()),
            _ => None,
        })
        .unwrap();
    let edit = &response[0]["textEdit"]["range"];
    assert_eq!(
        edit["start"],
        serde_json::json!({ "line": 2, "character": 2 })
    );
    assert_eq!(
        edit["end"],
        serde_json::json!({ "line": 2, "character": 4 })
    );
    assert_eq!(response[0]["detail"], Value::String("text".to_string()));
}
