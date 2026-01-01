import { Uri, workspace } from 'vscode';

export async function loadWasmBytes(extensionUri: Uri): Promise<Uint8Array> {
  const wasmUri = Uri.joinPath(extensionUri, 'pkg', 'eure_ls_bg.wasm');
  return workspace.fs.readFile(wasmUri);
}
