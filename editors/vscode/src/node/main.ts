import { ExtensionContext, window, workspace } from 'vscode';
import { LanguageClient, type ServerOptions } from 'vscode-languageclient/node';
import { activateCommon, type ActivationResult, type LanguageClientConstructor } from '../common';

let activation: ActivationResult | undefined;
let nativeClient: LanguageClient | undefined;

export async function activate(context: ExtensionContext) {
  const channel = window.createOutputChannel('Eure Language Server');
  context.subscriptions.push(channel);

  const config = workspace.getConfiguration('eure');
  const useWasm = config.get<boolean>('useWasm', true);

  if (useWasm) {
    try {
      activation = await activateCommon(
        context,
        channel,
        LanguageClient as unknown as LanguageClientConstructor
      );
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      channel.appendLine(`Failed to start Eure LS (WASM): ${message}`);
      void window.showErrorMessage(`Failed to start Eure LS: ${message}`);
      throw err;
    }
  } else {
    try {
      const configPath = config.get<string>('path', '');
      const command = configPath || 'eurels';

      channel.appendLine(`Starting native Eure LS: ${command}`);

      const serverOptions: ServerOptions = {
        command,
        args: [],
      };

      nativeClient = new LanguageClient(
        'eure-ls',
        'Eure Language Server',
        serverOptions,
        {
          documentSelector: [{ language: 'eure' }],
          outputChannel: channel,
        }
      );

      await nativeClient.start();
      channel.appendLine('Eure LS started (native).');
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      channel.appendLine(`Failed to start Eure LS (native): ${message}`);
      void window.showErrorMessage(`Failed to start Eure LS: ${message}`);
      throw err;
    }
  }
}

export async function deactivate() {
  activation?.eventLoop.dispose();
  await activation?.client.stop();
  activation = undefined;

  await nativeClient?.stop();
  nativeClient = undefined;
}
