import type { Uri } from 'vscode';
// @ts-expect-error - ESM module imported as type
import type { WasmCore, InitInput, InitOutput, CacheKeyInfo, CacheAction, CacheActionKind } from '../pkg/eure_ls';
import { debugLog } from './common';
import { loadWasmBytes } from './wasm-loader';

type InitFunction = (module_or_path?: InitInput) => Promise<InitOutput>;

// Dynamic import will be resolved after wasm-pack build
let initWasm: InitFunction | null = null;
let WasmCoreClass: (typeof WasmCore) | null = null;
// CacheActionKind enum - set during initialization
// eslint-disable-next-line import/no-mutable-exports
export let ActionKind: typeof CacheActionKind;

// Re-export generated types
export type { CacheKeyInfo, CacheAction, CacheActionKind };

export class WasmBridge {
  private core: WasmCore | null = null;

  async initialize(extensionUri: Uri): Promise<void> {
    debugLog('[WASM-Bridge] Importing wasm module...');
    const wasmModule = await import('../pkg/eure_ls.js');
    debugLog('[WASM-Bridge] Module imported: ' + Object.keys(wasmModule).join(', '));

    initWasm = wasmModule.default;
    WasmCoreClass = wasmModule.WasmCore;
    ActionKind = wasmModule.CacheActionKind;
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

  getPendingTextFiles(): string[] {
    return this.core!.get_pending_text_files();
  }

  resolveTextFile(uri: string, content: string | null, error: string | null): void {
    this.core!.resolve_text_file(uri, content ?? undefined, error ?? undefined);
  }

  getPendingGlobs(): Array<{ id: string; base_dir: string; pattern: string }> {
    return this.core!.get_pending_globs();
  }

  resolveGlob(id: string, files: string[]): void {
    this.core!.resolve_glob(id, files);
  }

  tick(): void {
    this.core!.tick();
  }

  // Cache helper methods

  computeCacheKey(url: string): CacheKeyInfo | undefined {
    return this.core!.compute_cache_key(url);
  }

  checkCacheStatus(metaJson: string | undefined, maxAgeSecs: number): CacheAction {
    return this.core!.check_cache_status(metaJson, maxAgeSecs);
  }

  buildCacheMeta(
    url: string,
    etag: string | undefined,
    lastModified: string | undefined,
    contentHash: string,
    sizeBytes: number
  ): string {
    return this.core!.build_cache_meta(url, etag, lastModified, contentHash, sizeBytes);
  }

  computeContentHash(content: string): string {
    return this.core!.compute_content_hash(content);
  }
}
