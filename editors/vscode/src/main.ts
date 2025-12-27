/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import {
  workspace,
  ExtensionContext,
  commands,
  window,
} from "vscode";

import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";
import path from "path";
import { homedir } from "os";

let client: LanguageClient | undefined;

export function activate(
  context: ExtensionContext
) {
  // Register the commands
  context.subscriptions.push(
    commands.registerCommand(
      "eure-ls.start",
      startLanguageServer
    ),
    commands.registerCommand(
      "eure-ls.stop",
      stopLanguageServer
    ),
    commands.registerCommand(
      "eure-ls.restart",
      restartLanguageServer
    ),
    commands.registerCommand(
      "eure.validateWithSchema",
      validateWithSchema
    ),
    commands.registerCommand(
      "eure.selectSchema",
      selectSchema
    )
  );

  // Start the language server by default
  startLanguageServer();

  async function startLanguageServer() {
    // TODO: Implement language server. For now textmate only.
    return;
    if (client) {
      window.showInformationMessage(
        "Eure language server is already running."
      );
      return;
    }

    // If the extension is launched in debug mode then the debug server options are used
    // Otherwise the run options are used
    const serverOptions: ServerOptions = {
      run: {
        command: path.join(
          homedir(),
          "/.cargo/bin/eure-ls"
        ),
        transport: TransportKind.stdio,
      },
      debug: {
        command: path.join(
          homedir(),
          "/.cargo/bin/eure-ls"
        ),
        transport: TransportKind.stdio,
      },
    };

    // Options to control the language client
    const clientOptions: LanguageClientOptions =
    {
      // Register the server for eure documents
      documentSelector: [
        { scheme: "file", language: "eure" },
      ],
      synchronize: {
        // Notify the server about file changes to '.clientrc files contained in the workspace
        fileEvents:
          workspace.createFileSystemWatcher(
            "**/*.eure"
          ),
      },
    };

    // Create the language client and start the client.
    client = new LanguageClient(
      "eure-ls",
      "Eure Language Server",
      serverOptions,
      clientOptions
    );

    // Start the client. This will also launch the server
    await client.start();
    window.showInformationMessage(
      "Eure language server started."
    );
  }

  async function stopLanguageServer() {
    if (!client) {
      window.showInformationMessage(
        "Eure language server is not running."
      );
      return;
    }

    await client.stop();
    client = undefined;
    window.showInformationMessage(
      "Eure language server stopped."
    );
  }

  async function restartLanguageServer() {
    window.showInformationMessage(
      "Restarting Eure language server..."
    );
    await stopLanguageServer();
    await startLanguageServer();
  }

  async function validateWithSchema() {
    const activeEditor = window.activeTextEditor;
    if (!activeEditor || activeEditor.document.languageId !== "eure") {
      window.showErrorMessage("No active Eure file to validate");
      return;
    }

    // Send a custom request to the language server to trigger validation
    if (!client) {
      window.showErrorMessage("Language server is not running");
      return;
    }

    // The language server already validates on change, so we just need to
    // inform the user that validation is active
    window.showInformationMessage(
      "Schema validation is active. Check the Problems panel for any issues."
    );
  }

  async function selectSchema() {
    const activeEditor = window.activeTextEditor;
    if (!activeEditor || activeEditor.document.languageId !== "eure") {
      window.showErrorMessage("No active Eure file");
      return;
    }

    // Show quick pick to select schema file
    const schemaFiles = await workspace.findFiles("**/*.schema.eure", "**/node_modules/**");

    if (schemaFiles.length === 0) {
      window.showWarningMessage("No schema files found in workspace");
      return;
    }

    const items = schemaFiles.map(uri => ({
      label: workspace.asRelativePath(uri),
      uri: uri
    }));

    const selected = await window.showQuickPick(items, {
      placeHolder: "Select a schema file for the current document"
    });

    if (selected) {
      // TODO: Send request to language server to associate this schema
      // For now, just inform the user
      window.showInformationMessage(
        `Schema association feature coming soon. Place your schema as ${activeEditor.document.fileName}.schema.eure for automatic detection.`
      );
    }
  }
}

export function deactivate():
  | Thenable<void>
  | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
