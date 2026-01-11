//! Eure Language Server - LSP implementation for the Eure data format.

pub mod io_pool;

use crate::io_pool::IoPool;
use anyhow::Result;
use crossbeam_channel::select;
use eure::query::{Glob, TextFile};
use eure_ls::{CoreRequestId, Effect, LspCore, LspOutput};
use lsp_server::{Connection, Message, Notification, Request, Response};
use tracing::{error, info};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("eure_ls=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    info!("Starting Eure Language Server");

    // Create LSP connection
    let (connection, io_threads) = Connection::stdio();

    // Initialize LSP - this happens before we create LspCore because lsp-server
    // handles the initialize handshake
    let server_capabilities = serde_json::to_value(eure_ls::server_capabilities())?;
    let init_params = connection.initialize(server_capabilities)?;
    let init_params: lsp_types::InitializeParams = serde_json::from_value(init_params)?;

    info!("Eure Language Server initialized");

    // Create components
    let io_pool = IoPool::new(4);
    let mut core = LspCore::new();

    // Register workspaces from initialization
    eure_ls::register_workspaces_from_init(core.runtime_mut(), &init_params);
    core.set_initialized();

    // Main event loop
    loop {
        select! {
            recv(connection.receiver) -> msg => {
                match msg {
                    Ok(Message::Request(req)) => {
                        if connection.handle_shutdown(&req)? {
                            info!("Shutdown requested");
                            break;
                        }
                        handle_request(req, &connection, &mut core, &io_pool);
                    }
                    Ok(Message::Notification(not)) => {
                        handle_notification(not, &connection, &mut core, &io_pool);
                    }
                    Ok(Message::Response(_)) => {
                        // We don't send requests, so we shouldn't receive responses
                    }
                    Err(e) => {
                        error!("Error receiving message: {}", e);
                        break;
                    }
                }
            }
            recv(io_pool.receiver()) -> response => {
                if let Ok(response) = response {
                    let result = match response.result {
                        Ok(content) => Ok(content.0),
                        Err(e) => Err(e.to_string()),
                    };
                    let (outputs, effects) = core.resolve_file(response.file, result);
                    send_outputs(&connection, outputs);
                    process_effects(&io_pool, &mut core, effects);
                }
            }
        }
    }

    io_threads.join()?;
    info!("Eure Language Server stopped");
    Ok(())
}

/// Handle an incoming LSP request.
fn handle_request(req: Request, connection: &Connection, core: &mut LspCore, io_pool: &IoPool) {
    let id = CoreRequestId::from(req.id.clone());
    let (outputs, effects) = core.handle_request(id, &req.method, req.params);

    // Send outputs that are ready
    send_outputs(connection, outputs);

    // Process effects (file fetches, glob expansions)
    process_effects(io_pool, core, effects);
}

/// Handle an incoming LSP notification.
fn handle_notification(
    not: Notification,
    connection: &Connection,
    core: &mut LspCore,
    io_pool: &IoPool,
) {
    let (outputs, effects) = core.handle_notification(&not.method, not.params);

    // Send outputs
    send_outputs(connection, outputs);

    // Process effects
    process_effects(io_pool, core, effects);
}

/// Send LspOutputs to the client.
fn send_outputs(connection: &Connection, outputs: Vec<LspOutput>) {
    for output in outputs {
        let msg = match output {
            LspOutput::Response { id, result } => {
                let response = match result {
                    Ok(value) => Response::new_ok(lsp_request_id(&id), value),
                    Err(err) => Response::new_err(lsp_request_id(&id), err.code, err.message),
                };
                Message::Response(response)
            }
            LspOutput::Notification { method, params } => {
                Message::Notification(Notification::new(method, params))
            }
        };

        if let Err(e) = connection.sender.send(msg) {
            error!("Failed to send message: {}", e);
        }
    }
}

/// Process effects by dispatching them to the appropriate handler.
fn process_effects(io_pool: &IoPool, core: &mut LspCore, effects: Vec<Effect>) {
    for effect in effects {
        match effect {
            Effect::FetchFile(file) => {
                io_pool.request_file(file);
            }
            Effect::ExpandGlob { id, glob } => {
                // Expand glob synchronously on native
                let files = expand_glob(&glob);
                // Resolve immediately - this may trigger more effects
                let (outputs, new_effects) = core.resolve_glob(&id, files);
                // We can't send outputs here since we don't have connection,
                // but glob expansion shouldn't produce immediate outputs anyway.
                // The effects will be processed when this function returns.
                drop(outputs);
                // Recursively process new effects
                if !new_effects.is_empty() {
                    process_effects(io_pool, core, new_effects);
                }
            }
        }
    }
}

/// Expand a glob pattern to matching files.
fn expand_glob(glob: &Glob) -> Vec<TextFile> {
    let pattern = glob.full_pattern();
    let pattern_str = pattern.to_string_lossy();
    glob::glob(&pattern_str)
        .into_iter()
        .flat_map(|paths| paths.flatten().map(TextFile::from_path))
        .collect()
}

/// Convert CoreRequestId to lsp_server RequestId.
fn lsp_request_id(id: &CoreRequestId) -> lsp_server::RequestId {
    // Try to parse as i32 first (most common case)
    if let Ok(n) = id.as_str().parse::<i32>() {
        lsp_server::RequestId::from(n)
    } else {
        lsp_server::RequestId::from(id.as_str().to_string())
    }
}
