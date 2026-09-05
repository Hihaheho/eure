import { languages, Uri, workspace, type ExtensionContext } from 'vscode';
import type { LanguageClient, LanguageClientOptions } from 'vscode-languageclient';

const SCHEME = 'eure-schema';

/** Keep HTTPS identities on the wire, including relative-import base URLs. */
export const schemaUriConverters: NonNullable<LanguageClientOptions['uriConverters']> = {
  code2Protocol: (uri) => (uri.scheme === SCHEME ? uri.with({ scheme: 'https' }) : uri).toString(),
  protocol2Code: (value) => {
    const uri = Uri.parse(value);
    return uri.scheme === 'https' ? uri.with({ scheme: SCHEME }) : uri;
  },
};

/** Display the server's source, rather than independently downloading it. */
export function registerSchemaDocuments(context: ExtensionContext, client: Pick<LanguageClient, 'sendRequest' | 'error'>): void {
  context.subscriptions.push(workspace.registerTextDocumentContentProvider(SCHEME, {
    provideTextDocumentContent(uri, token) {
      return client.sendRequest<string>('eure/schemaContent', {
        uri: schemaUriConverters.code2Protocol(uri),
      }, token);
    },
  }));
  // Remote endpoints need not have a .eure suffix.
  context.subscriptions.push(workspace.onDidOpenTextDocument((document) => {
    if (document.uri.scheme === SCHEME && document.languageId !== 'eure') {
      void languages.setTextDocumentLanguage(document, 'eure').then(undefined, (error: unknown) => {
        client.error('Failed to set remote schema language', error);
      });
    }
  }));
}
