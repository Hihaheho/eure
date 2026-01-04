import type { Uri } from 'vscode';
import type { WasmCore, InitInput } from '../pkg/eure_ls';
import { debugLog } from './common';
import { loadWasmBytes } from './wasm-loader';

type InitFunction = (module_or_path?: InitInput) => Promise<void>;

// Dynamic import will be resolved after wasm-pack build
let initWasm: InitFunction | null = null;
let WasmCoreClass: (typeof WasmCore) | null = null;

export class WasmBridge {
  private core: WasmCore | null = null;

  async initialize(extensionUri: Uri): Promise<void> {
    debugLog('[WASM-Bridge] Importing wasm module...');
    const wasmModule = await import('../pkg/eure_ls');
    debugLog('[WASM-Bridge] Module imported: ' + Object.keys(wasmModule).join(', '));

    initWasm = wasmModule.default;
    WasmCoreClass = wasmModule.WasmCore;
    debugLog('[WASM-Bridge] initWasm and WasmCoreClass set');

    debugLog('[WASM-Bridge] Loading WASM bytes...');
    const bytes = await loadWasmBytes(extensionUri);
    debugLog('[WASM-Bridge] WASM bytes loaded, size: ' + bytes.length);

    debugLog('[WASM-Bridge] Initializing WASM...');
    await initWasm!(bytes);
    debugLog('[WASM-Bridge] WASM initialized');

    debugLog('[WASM-Bridge] Creating WasmCore...');
    this.core = new WasmCoreClass!();
    debugLog('[WASM-Bridge] WasmCore created');
  }

  handleMessage(msg: unknown): void {
    this.core!.handle_message(msg);
  }

  drainOutbox(): unknown[] {
    return this.core!.drain_outbox();
  }

  getPendingAssets(): string[] {
    return this.core!.get_pending_assets();
  }

  resolveAsset(uri: string, content: string | null, error: string | null): void {
    this.core!.resolve_asset(uri, content ?? undefined, error ?? undefined);
  }

  tick(): void {
    this.core!.tick();
  }
}
