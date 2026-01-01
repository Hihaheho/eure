import {
  AbstractMessageReader,
  AbstractMessageWriter,
  DataCallback,
  Disposable,
  Message,
  MessageReader,
  MessageWriter,
} from 'vscode-jsonrpc';
import type { MessageTransports } from 'vscode-languageclient';
import { WasmEventLoop } from './wasm-event-loop';

class WasmMessageReader extends AbstractMessageReader implements MessageReader {
  private callback: DataCallback | null = null;

  listen(callback: DataCallback): Disposable {
    this.callback = callback;
    return Disposable.create(() => {
      this.callback = null;
    });
  }

  accept(msg: Message): void {
    try {
      this.callback?.(msg);
    } catch (e) {
      this.fireError(e as Error);
    }
  }

  close(): void {
    this.fireClose();
  }
}

class WasmMessageWriter extends AbstractMessageWriter implements MessageWriter {
  constructor(private eventLoop: WasmEventLoop) {
    super();
  }

  async write(msg: Message): Promise<void> {
    try {
      await this.eventLoop.sendMessage(msg);
    } catch (e) {
      this.fireError(e as Error, msg);
      throw e;
    }
  }

  end(): void {
    // no-op
  }
}

export function createWasmTransports(eventLoop: WasmEventLoop): MessageTransports {
  const reader = new WasmMessageReader();
  const writer = new WasmMessageWriter(eventLoop);

  eventLoop.onMessage((msg) => {
    reader.accept(msg);
  });

  return { reader, writer };
}
