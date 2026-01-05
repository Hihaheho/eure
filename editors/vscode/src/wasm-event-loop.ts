import type { Message } from 'vscode-jsonrpc';
import { Uri, workspace, extensions, RelativePattern } from 'vscode';
import { debugLog } from './common';
import { WasmBridge, ActionKind } from './wasm-bridge';

function getUserAgent(): string {
  const ext = extensions.getExtension('hihaheho.eurels');
  const version = ext?.packageJSON?.version ?? 'unknown';
  return `eurels@${version}`;
}

// Default cache max age: 24 hours (in seconds)
const DEFAULT_MAX_AGE_SECS = 24 * 60 * 60;

const MAX_PENDING_ITERATIONS = 20;

export class WasmEventLoop {
  private bridge: WasmBridge;
  private messageCallback: ((msg: Message) => void) | null = null;
  private messageQueue: unknown[] = [];
  private processing = false;
  private needsPump = false;
  private disposed = false;
  private cacheDir: Uri | null = null;

  constructor() {
    this.bridge = new WasmBridge();
  }

  async start(extensionUri: Uri, globalStorageUri?: Uri): Promise<void> {
    await this.bridge.initialize(extensionUri);
    // Use globalStorageUri for cache if provided
    if (globalStorageUri) {
      this.cacheDir = Uri.joinPath(globalStorageUri, 'schema-cache');
    }
  }

  onMessage(callback: (msg: Message) => void): void {
    this.messageCallback = callback;
  }

  async sendMessage(msg: Message): Promise<void> {
    debugLog('[EventLoop] sendMessage: ' + JSON.stringify(msg).slice(0, 200));
    if (this.disposed) return;
    this.messageQueue.push(msg);

    if (this.processing) {
      this.needsPump = true;
      return;
    }
    await this.pump();
  }

  dispose(): void {
    this.disposed = true;
    this.messageCallback = null;
  }

  private async pump(): Promise<void> {
    if (this.processing) return;
    this.processing = true;

    try {
      while ((this.messageQueue.length > 0 || this.needsPump) && !this.disposed) {
        this.needsPump = false;

        while (this.messageQueue.length > 0 && !this.disposed) {
          const msg = this.messageQueue.shift()!;
          debugLog('[EventLoop] pump: calling handleMessage');
          this.bridge.handleMessage(msg);
          debugLog('[EventLoop] pump: handleMessage done, flushing outbox');
          this.flushOutbox();
          debugLog('[EventLoop] pump: resolving pending assets');
          await this.resolvePendingAssets();
          debugLog('[EventLoop] pump: cycle complete');
          // Yield to prevent UI blocking
          await new Promise((r) => setTimeout(r, 0));
        }
      }
    } finally {
      this.processing = false;
      // Reschedule if messages arrived during processing
      if ((this.needsPump || this.messageQueue.length > 0) && !this.disposed) {
        queueMicrotask(() => this.pump());
      }
    }
  }

  private flushOutbox(): void {
    const messages = this.bridge.drainOutbox();
    debugLog('[EventLoop] flushOutbox: draining ' + messages.length + ' messages');
    for (const outMsg of messages) {
      debugLog('[EventLoop] outbox message: ' + JSON.stringify(outMsg).slice(0, 200));
      this.messageCallback?.(outMsg as Message);
    }
  }

  private async resolvePendingAssets(): Promise<void> {
    const resolvedTextFilesInThisPump = new Set<string>();
    const resolvedGlobsInThisPump = new Set<string>();

    for (let i = 0; i < MAX_PENDING_ITERATIONS; i++) {
      // Resolve pending text files
      const pendingTextFiles = this.bridge.getPendingTextFiles();
      const newPendingTextFiles = pendingTextFiles.filter((uri) => !resolvedTextFilesInThisPump.has(uri));

      // Resolve pending globs
      const pendingGlobs = this.bridge.getPendingGlobs();
      const newPendingGlobs = pendingGlobs.filter((glob) => !resolvedGlobsInThisPump.has(glob.id));

      if (newPendingTextFiles.length === 0 && newPendingGlobs.length === 0) break;

      // Resolve text files
      await Promise.all(
        newPendingTextFiles.map(async (uri) => {
          resolvedTextFilesInThisPump.add(uri);
          try {
            const parsedUri = Uri.parse(uri);
            if (parsedUri.scheme === 'file') {
              // Local file: read from filesystem
              const content = await workspace.fs.readFile(parsedUri);
              this.bridge.resolveTextFile(uri, new TextDecoder().decode(content), null);
            } else if (parsedUri.scheme === 'https') {
              // Remote URL: fetch via HTTP with caching
              const result = await this.fetchWithCache(uri);
              if (result.content !== null) {
                this.bridge.resolveTextFile(uri, result.content, null);
              } else {
                this.bridge.resolveTextFile(uri, null, result.error ?? 'Unknown error');
              }
            } else {
              // Unknown scheme
              this.bridge.resolveTextFile(uri, null, null);
            }
          } catch (e) {
            const errorMsg = e instanceof Error ? e.message : String(e);
            this.bridge.resolveTextFile(uri, null, errorMsg);
          }
        })
      );

      // Resolve globs by finding matching files
      await Promise.all(
        newPendingGlobs.map(async (glob) => {
          resolvedGlobsInThisPump.add(glob.id);
          try {
            // Use VS Code's findFiles API with glob pattern
            const baseUri = Uri.file(glob.base_dir);
            const pattern = new RelativePattern(baseUri, glob.pattern);
            const files = await workspace.findFiles(pattern);
            const fileUris = files.map((f) => f.toString());
            this.bridge.resolveGlob(glob.id, fileUris);
          } catch (e) {
            // On error, resolve with empty array
            debugLog(`[EventLoop] Glob error for ${glob.pattern}: ${e}`);
            this.bridge.resolveGlob(glob.id, []);
          }
        })
      );

      this.bridge.tick();
      this.flushOutbox();
    }
  }

  /**
   * Fetch a remote URL with caching support.
   * Uses WASM helpers to determine cache strategy and build metadata.
   */
  private async fetchWithCache(url: string): Promise<{ content: string | null; error: string | null }> {
    // If no cache directory, just fetch directly
    if (!this.cacheDir) {
      return this.fetchDirect(url);
    }

    try {
      // Get cache key info from WASM
      const keyInfo = this.bridge.computeCacheKey(url);
      if (!keyInfo) {
        return this.fetchDirect(url);
      }

      // Build cache file paths
      const cacheFilePath = Uri.joinPath(this.cacheDir, keyInfo.cache_path);
      const metaFilePath = Uri.joinPath(this.cacheDir, keyInfo.cache_path + '.meta');

      // Try to read existing meta
      let metaJson: string | undefined;
      try {
        const metaBytes = await workspace.fs.readFile(metaFilePath);
        metaJson = new TextDecoder().decode(metaBytes);
      } catch {
        // No cached meta
      }

      // Ask WASM what to do
      const action = this.bridge.checkCacheStatus(metaJson, DEFAULT_MAX_AGE_SECS);

      if (action.action === ActionKind.UseCached) {
        // Cache is fresh, just read it
        debugLog(`[Cache] Using cached: ${url} (${cacheFilePath.fsPath})`);
        try {
          const content = await workspace.fs.readFile(cacheFilePath);
          return { content: new TextDecoder().decode(content), error: null };
        } catch {
          // Cache file missing, fall through to fetch
          debugLog(`[Cache] Cache file missing, fetching: ${url}`);
        }
      }

      // Build request headers
      const headers: Record<string, string> = {
        'User-Agent': getUserAgent(),
      };

      if (action.action === ActionKind.Revalidate && action.headers) {
        debugLog(`[Cache] Revalidating: ${url}`);
        if (action.headers.if_none_match) {
          headers['If-None-Match'] = action.headers.if_none_match;
        }
        if (action.headers.if_modified_since) {
          headers['If-Modified-Since'] = action.headers.if_modified_since;
        }
      } else {
        debugLog(`[Cache] Fetching fresh: ${url}`);
      }

      // Make the request
      const response = await fetch(url, { headers });

      if (response.status === 304) {
        // Not modified - use cached content
        debugLog(`[Cache] 304 Not Modified: ${url}`);
        try {
          const content = await workspace.fs.readFile(cacheFilePath);
          // Update last_used_at in meta (touch)
          await this.updateCacheLastUsed(metaFilePath, metaJson!);
          return { content: new TextDecoder().decode(content), error: null };
        } catch {
          // Cache file missing after 304 - this shouldn't happen, but fetch again
          return this.fetchDirect(url);
        }
      }

      if (!response.ok) {
        return { content: null, error: `HTTP ${response.status}: ${response.statusText}` };
      }

      // Got fresh content
      const content = await response.text();
      debugLog(`[Cache] Fetched ${content.length} bytes: ${url}`);

      // Cache it
      try {
        await this.writeCacheEntry(url, content, response, cacheFilePath, metaFilePath);
      } catch (e) {
        debugLog(`[Cache] Failed to write cache: ${e}`);
        // Continue anyway, we have the content
      }

      return { content, error: null };
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      debugLog(`[Cache] Error: ${errorMsg}`);
      return { content: null, error: errorMsg };
    }
  }

  /**
   * Fetch directly without caching.
   */
  private async fetchDirect(url: string): Promise<{ content: string | null; error: string | null }> {
    try {
      const response = await fetch(url, {
        headers: { 'User-Agent': getUserAgent() },
      });
      if (response.ok) {
        const content = await response.text();
        return { content, error: null };
      } else {
        return { content: null, error: `HTTP ${response.status}: ${response.statusText}` };
      }
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      return { content: null, error: errorMsg };
    }
  }

  /**
   * Write content and metadata to cache.
   */
  private async writeCacheEntry(
    url: string,
    content: string,
    response: Response,
    contentPath: Uri,
    metaPath: Uri
  ): Promise<void> {
    // Compute content hash
    const contentHash = this.bridge.computeContentHash(content);

    // Extract response headers
    const etag = response.headers.get('etag') ?? undefined;
    const lastModified = response.headers.get('last-modified') ?? undefined;

    // Build metadata JSON
    const metaJson = this.bridge.buildCacheMeta(
      url,
      etag,
      lastModified,
      contentHash,
      content.length
    );

    // Ensure directory exists and write files
    const contentBytes = new TextEncoder().encode(content);
    const metaBytes = new TextEncoder().encode(metaJson);

    await workspace.fs.writeFile(contentPath, contentBytes);
    await workspace.fs.writeFile(metaPath, metaBytes);
  }

  /**
   * Update last_used_at in cached metadata.
   */
  private async updateCacheLastUsed(metaPath: Uri, existingMetaJson: string): Promise<void> {
    try {
      const meta = JSON.parse(existingMetaJson);
      meta.last_used_at = new Date().toISOString();
      const updatedJson = JSON.stringify(meta, null, 2);
      await workspace.fs.writeFile(metaPath, new TextEncoder().encode(updatedJson));
    } catch {
      // Ignore errors updating metadata
    }
  }
}
