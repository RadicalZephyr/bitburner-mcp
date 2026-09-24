import type { NS } from '@ns';

/**
 * Where the host-side proxy listens. Override with the first script
 * argument: `run mcp.js ws://localhost:12526`.
 */
const DEFAULT_URL = 'ws://localhost:12526';

/** Pause between reconnect attempts. */
const RECONNECT_MS = 5_000;

type JsonRpcId = number | string | null;

interface JsonRpcRequest {
    jsonrpc: '2.0';
    id?: JsonRpcId;
    method: string;
    params?: unknown;
}

interface JsonRpcResponse {
    jsonrpc: '2.0';
    id: JsonRpcId;
    result?: unknown;
    error?: { code: number; message: string };
}

export async function main(ns: NS): Promise<void> {
    ns.disableLog('asleep');
    const url = String(ns.args[0] ?? DEFAULT_URL);

    let socket: WebSocket | undefined;
    ns.atExit(() => socket?.close());

    // Reconnect forever. Killing the script is how this stops: atExit
    // closes the socket, and the next Netscript call throws ScriptDeath.
    for (;;) {
        ns.print(`INFO: connecting to ${url}`);
        socket = new WebSocket(url);
        await serve(ns, socket);
        socket = undefined;
        ns.print(`WARN: disconnected, retrying in ${RECONNECT_MS / 1000}s`);
        await ns.asleep(RECONNECT_MS);
    }
}

/**
 * Serve one connection until it closes.
 *
 * The handlers run while `main` is parked on a plain promise, so Netscript
 * calls made from them pass the concurrency check. That stays true only as
 * long as `main` never waits on a Netscript function other than `asleep`,
 * and handlers never overlap two async Netscript calls.
 *
 * @param ns - Netscript API
 * @param socket - Freshly constructed socket to the proxy
 * @returns Resolves when the socket closes
 */
function serve(ns: NS, socket: WebSocket): Promise<void> {
    return new Promise((resolve) => {
        socket.onopen = () => ns.print('SUCCESS: connected');
        socket.onerror = () => ns.print('ERROR: websocket error');
        socket.onclose = () => resolve();
        socket.onmessage = (event: MessageEvent<string>) => {
            const reply = handle(event.data);
            if (reply !== undefined) {
                socket.send(JSON.stringify(reply));
            }
        };
    });
}

/**
 * Answer one JSON-RPC message. Stub: only `ping` is implemented, which is
 * enough to test the pipe end to end. Everything else is "method not found".
 *
 * @param raw - One JSON-RPC message as sent by the proxy
 * @returns The response, or nothing for a notification
 */
function handle(raw: string): JsonRpcResponse | undefined {
    let request: JsonRpcRequest;
    try {
        request = JSON.parse(raw) as JsonRpcRequest;
    } catch {
        return error(null, -32700, 'Parse error');
    }
    if (request.method === 'ping') {
        return { jsonrpc: '2.0', id: request.id ?? null, result: {} };
    }
    if (request.id === undefined) {
        return undefined; // Notification: nothing to say back.
    }
    return error(request.id, -32601, `Method not found: ${request.method}`);
}

function error(id: JsonRpcId, code: number, message: string): JsonRpcResponse {
    return { jsonrpc: '2.0', id, error: { code, message } };
}
