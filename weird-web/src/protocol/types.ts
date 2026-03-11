import * as z from "zod";

export const JsonRpcRequest = z.object({
  jsonrpc: z.literal("2.0"),
  method: z.string(),
  params: z.unknown().optional(),
  id: z.string().or(z.number()).nullable(),
});
export type JsonRpcRequest = z.infer<typeof JsonRpcRequest>;

export const JsonRpcSuccessResponse = z.object({
  jsonrpc: z.literal("2.0"),
  result: z.unknown().optional(),
  id: z.string().or(z.number()).nullable(),
});
export type JsonRpcSuccessResponse = z.infer<typeof JsonRpcSuccessResponse>;

export const JsonRpcError = z.object({
  code: z.int(),
  message: z.string(),
  data: z.unknown().optional(),
});
export type JsonRpcError = z.infer<typeof JsonRpcError>;

export const JsonRpcErrorResponse = z.object({
  jsonrpc: z.literal("2.0"),
  error: JsonRpcError,
  id: z.string().or(z.number()).nullable(),
});
export type JsonRpcErrorResponse = z.infer<typeof JsonRpcErrorResponse>;

export const JsonRpcResponse = JsonRpcErrorResponse.or(JsonRpcSuccessResponse);
export type JsonRpcResponse = z.infer<typeof JsonRpcResponse>;

export const NodeId = z.string().brand("NodeId");
export type NodeId = z.infer<typeof NodeId>;

export const FlatElement = z.object({
  tag: z.string(),
  attributes: z.record(z.string(), z.unknown()),
});
export type FlatElement = z.infer<typeof FlatElement>;

export const FlatNode = z.union([z.string(), FlatElement]);
export type FlatNode = z.infer<typeof FlatNode>;

export const InsertedNode = z.object({
  id: NodeId,
  parent: NodeId,
  node: FlatNode,
});
export type InsertedNode = z.infer<typeof InsertedNode>;

export const WorldDidChangeEvent = z.object({
  inserted: InsertedNode.array(),
  removed: NodeId.array(),
});
export type WorldDidChangeEvent = z.infer<typeof WorldDidChangeEvent>;

export const EventType = z.union([z.literal("syncWorld")]);
export type EventType = z.infer<typeof EventType>;

export const Events = {
  syncWorld: {
    request: z.object({}),
    response: WorldDidChangeEvent,
  },
} as const satisfies {
  [K in EventType]: { request: z.ZodObject; response: z.ZodObject };
};

export type EventParams<E extends EventType> = z.output<
  (typeof Events)[E]["request"]
>;

export type EventResponse<E extends EventType> = z.output<
  (typeof Events)[E]["response"]
>;
