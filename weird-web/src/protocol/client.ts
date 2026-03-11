import {
  type EventType,
  type EventParams,
  type EventResponse,
  JsonRpcResponse,
  JsonRpcError,
  JsonRpcRequest,
  Events,
} from "./types";

export class WeirdClient {
  #nextId: number = 1;
  #listeners: Map<string | number, ClientListener> = new Map();
  #socket: WebSocket;

  constructor(socket: WebSocket) {
    this.#socket = socket;

    socket.addEventListener("message", (event) => {
      if (typeof event.data !== "string") {
        console.warn("Ignoring non-string WebSocket messsage", { event });
        return;
      }

      let data;
      try {
        data = JSON.parse(event.data);
      } catch (error) {
        console.warn("Ignoring non-JSON WebSocket message", {
          data: event.data,
          error,
        });
        return;
      }

      const rpcResponseParsed = JsonRpcResponse.safeParse(data);
      if (!rpcResponseParsed.success) {
        console.warn("Failed to parse JSON RPC response message", {
          data,
          error: rpcResponseParsed.error,
        });
        return;
      }

      const rpcResponse = rpcResponseParsed.data;
      if (rpcResponse.id === null) {
        console.debug("Received JSON RPC message with nullish key, ignoring");
        return;
      }

      const listener = this.#listeners.get(rpcResponse.id);
      if (listener == null) {
        console.info("Received JSON RPC message with no listener, ignoring", {
          rpcResponse,
        });
        return;
      }

      if ("error" in rpcResponse) {
        if (listener.onError) {
          listener.onError(rpcResponse.error);
        } else {
          console.warn("Unhandled JSON RPC error", { rpcResponse });
        }
      } else {
        listener.on?.(rpcResponse.result);
      }

      if (listener.once) {
        this.#listeners.delete(rpcResponse.id);
      }
    });
    socket.addEventListener("close", () => {
      this.#listeners.forEach((listener) => {
        listener.onClose?.({ cause: "socketClosed" });
      });
      this.#listeners.clear();
    });
    socket.addEventListener("error", () => {
      this.#listeners.forEach((listener) => {
        listener.onClose?.({ cause: "socketError" });
      });
      this.#listeners.clear();
    });
  }

  #sendRequest(
    method: string,
    params: unknown,
    listener: ClientListener,
  ): Listener {
    const id = this.#nextId++;
    this.#listeners.set(id, listener);
    this.#socket.send(
      JSON.stringify({
        jsonrpc: "2.0",
        method,
        params,
        id,
      } satisfies JsonRpcRequest),
    );

    return {
      unsubscribe: () => {
        this.#listeners.delete(id);
      },
    };
  }

  subscribe<E extends EventType>(options: SubscribeOptions<E>): Listener {
    const on =
      options.on != null
        ? (params: unknown) => {
            const eventParamsType = Events[options.event].response;
            const eventParams = eventParamsType.safeParse(params);
            if (eventParams.success) {
              options.on?.(eventParams.data as EventResponse<E>);
            } else {
              console.error("Failed to parse event response for message", {
                event: options.event,
                params,
                error: eventParams.error,
              });
            }
          }
        : undefined;
    return this.#sendRequest(options.event, options.params, {
      on,
      onClose: options.onClose,
      onError: options.onError,
      once: false,
    });
  }
}

interface SubscribeOptions<E extends EventType> {
  event: E;
  params: EventParams<E>;
  on?: (params: EventResponse<E>) => void;
  onError?: (error: JsonRpcError) => void;
  onClose?: (cause: CloseCause) => void;
}

interface ClientListener {
  on?: (params: unknown) => void;
  onError?: (error: JsonRpcError) => void;
  onClose?: (cause: CloseCause) => void;
  once: boolean;
}

type CloseCause =
  | { cause: "socketClosed" }
  | { cause: "socketError" }
  | { cause: "unsubscribed" };

export interface Listener {
  unsubscribe(): void;
}
