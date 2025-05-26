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
    )
  );

  // Start the language server by default
  startLanguageServer();

  async function startLanguageServer() {
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
}

export function deactivate():
  | Thenable<void>
  | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
