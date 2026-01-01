import { ExtensionContext, window } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/browser';
import { activateCommon, type ActivationResult, type LanguageClientConstructor } from '../common';

let activation: ActivationResult | undefined;

export async function activate(context: ExtensionContext) {
  const channel = window.createOutputChannel('Eure Language Server');
  context.subscriptions.push(channel);

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
}

export async function deactivate() {
  activation?.eventLoop.dispose();
  await activation?.client.stop();
  activation = undefined;
}
