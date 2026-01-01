import type { ExtensionContext, OutputChannel } from 'vscode';
import type { LanguageClient, LanguageClientOptions, MessageTransports } from 'vscode-languageclient';
import { WasmEventLoop } from './wasm-event-loop';
import { createWasmTransports } from './wasm-transport';

// Global channel for debug logging
let debugChannel: OutputChannel | null = null;
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
  channel: OutputChannel,
  ClientCtor: LanguageClientConstructor
): Promise<ActivationResult> {
  debugChannel = channel;
  channel.appendLine('[DEBUG] Creating WasmEventLoop...');
  const eventLoop = new WasmEventLoop();

  channel.appendLine('[DEBUG] Starting eventLoop...');
  await eventLoop.start(context.extensionUri);
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
      outputChannel: channel,
    }
  );
  channel.appendLine('[DEBUG] LanguageClient created.');

  channel.appendLine('[DEBUG] Starting client...');
  await client.start();
  channel.appendLine('Eure LS started (WASM).');

  return { client, eventLoop };
}
