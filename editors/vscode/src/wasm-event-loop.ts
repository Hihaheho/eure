import type { Message } from 'vscode-jsonrpc';
import { Uri, workspace, extensions } from 'vscode';
import { debugLog } from './common';
import { WasmBridge } from './wasm-bridge';

function getUserAgent(): string {
  const ext = extensions.getExtension('hihaheho.vscode-eurels');
  const version = ext?.packageJSON?.version ?? 'unknown';
  return `vscode-eurels@${version}`;
}

const MAX_PENDING_ITERATIONS = 20;

export class WasmEventLoop {
  private bridge: WasmBridge;
  private messageCallback: ((msg: Message) => void) | null = null;
  private messageQueue: unknown[] = [];
  private processing = false;
  private needsPump = false;
  private disposed = false;

  constructor() {
    this.bridge = new WasmBridge();
  }

  async start(extensionUri: Uri): Promise<void> {
    await this.bridge.initialize(extensionUri);
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
              // Remote URL: fetch via HTTP
              const response = await fetch(uri, {
                headers: { 'User-Agent': getUserAgent() },
              });
              if (response.ok) {
                const text = await response.text();
                this.bridge.resolveTextFile(uri, text, null);
              } else {
                this.bridge.resolveTextFile(uri, null, `HTTP ${response.status}: ${response.statusText}`);
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
            const pattern = new workspace.RelativePattern(baseUri, glob.pattern);
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
}
