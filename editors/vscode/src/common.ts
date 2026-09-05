import type { ExtensionContext, LogOutputChannel } from 'vscode';
import type { LanguageClient, LanguageClientOptions, MessageTransports } from 'vscode-languageclient';
import { registerSchemaDocuments, schemaUriConverters } from './schema-documents';
import { WasmEventLoop } from './wasm-event-loop';
import { createWasmTransports } from './wasm-transport';

// Global channel for debug logging
let debugChannel: LogOutputChannel | null = null;
export function debugLog(msg: string): void {
  debugChannel?.appendLine(msg);
}

export type LanguageClientConstructor = new (
  id: string,
  name: string,
  serverOptions: () => Promise<MessageTransports>,
  clientOptions: LanguageClientOptions
) => LanguageClient;

export interface ActivationResult {
  client: LanguageClient;
  eventLoop: WasmEventLoop;
}

export async function activateCommon(
  context: ExtensionContext,
  channel: LogOutputChannel,
  ClientCtor: LanguageClientConstructor
): Promise<ActivationResult> {
  debugChannel = channel;
  channel.appendLine('[DEBUG] Creating WasmEventLoop...');
  const eventLoop = new WasmEventLoop();

  channel.appendLine('[DEBUG] Starting eventLoop...');
  await eventLoop.start(context.extensionUri, context.globalStorageUri);
  channel.appendLine('[DEBUG] eventLoop started.');

  channel.appendLine('[DEBUG] Creating transports...');
  const transports = createWasmTransports(eventLoop);
  channel.appendLine('[DEBUG] Transports created.');

  channel.appendLine('[DEBUG] Creating LanguageClient...');
  const client = new ClientCtor(
    'eure-ls',
    'Eure Language Server',
    async () => {
      channel.appendLine('[DEBUG] serverOptions called, returning transports...');
      return transports;
    },
    {
      documentSelector: [{ language: 'eure' }],
      uriConverters: schemaUriConverters,
      outputChannel: channel,
    }
  );
  channel.appendLine('[DEBUG] LanguageClient created.');

  channel.appendLine('[DEBUG] Starting client...');
  registerSchemaDocuments(context, client);
  await client.start();
  channel.appendLine('Eure LS started (WASM).');

  return { client, eventLoop };
}
