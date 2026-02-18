// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Thin Unix socket client for communicating with the Cortex runtime.
 *
 * Implements the newline-delimited JSON protocol used by all Cortex clients:
 *   Request:  {"id": "...", "method": "...", "params": {...}}\n
 *   Response: {"id": "...", "result": {...}} | {"id": "...", "error": {...}}\n
 */

import * as net from "node:net";

/** Default path for the Cortex Unix domain socket. */
export const DEFAULT_SOCKET_PATH = "/tmp/cortex.sock";

/** Default timeout for socket operations in milliseconds. */
export const DEFAULT_TIMEOUT = 60_000;

/** Maximum number of automatic reconnection attempts on broken pipe. */
const MAX_RECONNECT_ATTEMPTS = 2;

/** Response from the Cortex runtime. */
export interface CortexResponse {
  id: string;
  result?: Record<string, unknown>;
  error?: {
    code?: string;
    message?: string;
  };
}

/** Error thrown when communication with Cortex fails. */
export class CortexClientError extends Error {
  readonly code: string;

  constructor(message: string, code = "E_UNKNOWN") {
    super(message);
    this.name = "CortexClientError";
    this.code = code;
  }
}

/**
 * Unix socket client for the Cortex runtime.
 *
 * Connects to the Cortex daemon over a Unix domain socket and sends
 * newline-delimited JSON messages. Auto-reconnects on broken pipe.
 *
 * Usage:
 * ```ts
 * const client = new CortexClient();
 * await client.connect();
 * const result = await client.send("map", { domain: "example.com" });
 * client.disconnect();
 * ```
 */
export class CortexClient {
  private socketPath: string;
  private timeout: number;
  private socket: net.Socket | null = null;
  private buffer = "";
  private requestCounter = 0;

  constructor(
    socketPath: string = DEFAULT_SOCKET_PATH,
    timeout: number = DEFAULT_TIMEOUT,
  ) {
    this.socketPath = socketPath;
    this.timeout = timeout;
  }

  /**
   * Connect to the Cortex runtime socket.
   *
   * @throws {CortexClientError} If the socket cannot be reached.
   */
  async connect(): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      const sock = net.createConnection({ path: this.socketPath }, () => {
        resolve();
      });
      sock.setTimeout(this.timeout);

      sock.on("error", (err: NodeJS.ErrnoException) => {
        this.socket = null;
        const code = err.code ?? "";
        if (code === "ENOENT") {
          reject(
            new CortexClientError(
              `Cannot connect to Cortex at ${this.socketPath}. ` +
                "The process may not be running. Start it with: cortex start",
              "E_SOCKET_NOT_FOUND",
            ),
          );
        } else if (code === "ECONNREFUSED") {
          reject(
            new CortexClientError(
              `Cortex refused connection at ${this.socketPath}. ` +
                "The process may have crashed. Try: cortex stop && cortex start",
              "E_CONNECTION_REFUSED",
            ),
          );
        } else if (code === "EACCES") {
          reject(
            new CortexClientError(
              `Permission denied on ${this.socketPath}.`,
              "E_PERMISSION_DENIED",
            ),
          );
        } else {
          reject(
            new CortexClientError(
              `Cannot connect to Cortex: ${err.message}`,
              "E_CONNECTION",
            ),
          );
        }
      });

      this.socket = sock;
    });
  }

  /**
   * Disconnect from the Cortex runtime.
   */
  disconnect(): void {
    if (this.socket) {
      this.socket.destroy();
      this.socket = null;
    }
    this.buffer = "";
  }

  /** Whether the client is currently connected. */
  get isConnected(): boolean {
    return this.socket !== null && !this.socket.destroyed;
  }

  /**
   * Send a protocol method call and return the result.
   *
   * Auto-connects if not connected. Retries on broken pipe up to
   * {@link MAX_RECONNECT_ATTEMPTS} times.
   *
   * @param method - Cortex protocol method (e.g. "map", "query", "pathfind").
   * @param params - Method parameters.
   * @returns The result payload from the Cortex response.
   * @throws {CortexClientError} On connection, timeout, or protocol errors.
   */
  async send(
    method: string,
    params: Record<string, unknown> = {},
  ): Promise<Record<string, unknown>> {
    let lastError: Error | undefined;

    for (let attempt = 0; attempt <= MAX_RECONNECT_ATTEMPTS; attempt++) {
      try {
        if (!this.isConnected) {
          await this.connect();
        }
        const response = await this.sendRaw(method, params);
        if (response.error) {
          throw new CortexClientError(
            response.error.message ?? `Cortex error on ${method}`,
            response.error.code ?? "E_CORTEX",
          );
        }
        return response.result ?? {};
      } catch (err) {
        lastError = err as Error;
        // Only retry on connection/pipe errors, not on protocol errors
        if (
          err instanceof CortexClientError &&
          (err.code === "E_BROKEN_PIPE" || err.code === "E_CONNECTION_CLOSED")
        ) {
          this.disconnect();
          continue;
        }
        throw err;
      }
    }

    throw (
      lastError ??
      new CortexClientError(
        "Failed to send request after reconnection attempts",
        "E_RECONNECT_FAILED",
      )
    );
  }

  /**
   * Send a raw request and return the full response object.
   */
  private async sendRaw(
    method: string,
    params: Record<string, unknown>,
  ): Promise<CortexResponse> {
    const sock = this.socket;
    if (!sock || sock.destroyed) {
      throw new CortexClientError(
        "Not connected to Cortex runtime.",
        "E_NOT_CONNECTED",
      );
    }

    const id = `mcp-${++this.requestCounter}`;
    const request = JSON.stringify({ id, method, params }) + "\n";

    return new Promise<CortexResponse>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        cleanup();
        reject(
          new CortexClientError(
            `Timeout on ${method} request after ${this.timeout}ms. ` +
              "The Cortex daemon may be overloaded.",
            "E_TIMEOUT",
          ),
        );
      }, this.timeout);

      const onData = (chunk: Buffer): void => {
        this.buffer += chunk.toString("utf-8");
        const newlineIdx = this.buffer.indexOf("\n");
        if (newlineIdx !== -1) {
          const line = this.buffer.substring(0, newlineIdx);
          this.buffer = this.buffer.substring(newlineIdx + 1);
          cleanup();
          try {
            resolve(JSON.parse(line) as CortexResponse);
          } catch {
            reject(
              new CortexClientError(
                "Invalid JSON response from Cortex daemon.",
                "E_INVALID_JSON",
              ),
            );
          }
        }
      };

      const onError = (err: NodeJS.ErrnoException): void => {
        cleanup();
        if (err.code === "EPIPE") {
          reject(
            new CortexClientError(
              "Broken pipe — Cortex daemon may have restarted.",
              "E_BROKEN_PIPE",
            ),
          );
        } else {
          reject(
            new CortexClientError(
              `Connection error: ${err.message}`,
              "E_CONNECTION",
            ),
          );
        }
      };

      const onClose = (): void => {
        cleanup();
        reject(
          new CortexClientError(
            "Connection closed by Cortex daemon.",
            "E_CONNECTION_CLOSED",
          ),
        );
      };

      const cleanup = (): void => {
        clearTimeout(timeoutId);
        sock.removeListener("data", onData);
        sock.removeListener("error", onError);
        sock.removeListener("close", onClose);
      };

      sock.on("data", onData);
      sock.on("error", onError);
      sock.on("close", onClose);

      sock.write(request, (err) => {
        if (err) {
          cleanup();
          reject(
            new CortexClientError(
              `Failed to write to socket: ${err.message}`,
              "E_WRITE_FAILED",
            ),
          );
        }
      });
    });
  }
}
