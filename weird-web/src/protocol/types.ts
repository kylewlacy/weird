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

export const ConnectionId = z.string().brand("ConnectionId");
export type ConnectionId = z.infer<typeof ConnectionId>;

export const WeirdProtocolVersion = z.enum(["0.1.0"]);
export type WeirdProtocolVersion = z.infer<typeof WeirdProtocolVersion>;

export const InitRequest = z.object({
  weirdProtocolVersion: WeirdProtocolVersion,
  client: z.string().nullish(),
  app: z.string().nullish(),
});
export type InitRequest = z.infer<typeof InitRequest>;

export const InitResponse = z.object({
  weirdProtocolVersion: WeirdProtocolVersion,
  connectionId: ConnectionId,
});
export type InitResponse = z.infer<typeof InitResponse>;

export const FlatElement = z.object({
  tag: z.string(),
  attributes: z.record(z.string(), z.unknown()),
});
export type FlatElement = z.infer<typeof FlatElement>;

export const FlatNode = z.union([z.string(), FlatElement]);
export type FlatNode = z.infer<typeof FlatNode>;

export const InsertedNode = z.object({
  id: NodeId,
  parentId: NodeId,
  parentIndex: z.number(),
  node: FlatNode.nullish(),
});
export type InsertedNode = z.infer<typeof InsertedNode>;

export const CreatedNodeChange = z.object({
  type: z.literal("created"),
  id: NodeId,
  parentId: NodeId,
  beforeSiblingId: NodeId.nullish(),
  node: FlatNode,
});
export type CreatedNodeChange = z.infer<typeof CreatedNodeChange>;

export const UpdatedNodeChange = z.object({
  type: z.literal("updated"),
  id: NodeId,
  text: z.string().nullish(),
  setAttributes: z.record(z.string(), z.unknown()).nullish(),
  clearAttributes: z.string().array().nullish(),
});
export type UpdatedNodeChange = z.infer<typeof UpdatedNodeChange>;

export const MovedNodeChange = z.object({
  type: z.literal("moved"),
  id: NodeId,
  parentId: NodeId,
  beforeSiblingId: NodeId.nullish(),
});
export type MovedNodeChange = z.infer<typeof MovedNodeChange>;

export const DeletedNodeChange = z.object({
  type: z.literal("deleted"),
  id: NodeId,
});
export type DeletedNodeChange = z.infer<typeof DeletedNodeChange>;

export const WorldChange = z.discriminatedUnion("type", [
  CreatedNodeChange,
  UpdatedNodeChange,
  MovedNodeChange,
  DeletedNodeChange,
]);
type WorldChange = z.infer<typeof WorldChange>;

export const WorldDidChangeResponse = z.object({
  changes: WorldChange.array(),
});
export type WorldDidChangeResponse = z.infer<typeof WorldDidChangeResponse>;

export const ConnectionSource = z
  .discriminatedUnion("type", [
    z.object({ type: z.literal("websocket") }),
    z.object({ type: z.literal("unixSocket") }),
    z.object({ type: z.literal("other") }),
  ])
  .or(z.object({ type: z.string() }));

export const ConnectionDetails = z.object({
  connectionId: ConnectionId,
  source: ConnectionSource,
  connected: z.boolean(),
  weirdProtocolVersion: WeirdProtocolVersion,
  client: z.string().nullish(),
  app: z.string().nullish(),
});
export type ConnectionDetails = z.infer<typeof ConnectionDetails>;

export const ConnectedEvent = ConnectionDetails.extend({
  type: z.literal("connected"),
});
export type ConnectedEvent = z.infer<typeof ConnectedEvent>;

export const DisconnectedEvent = z.object({
  type: z.literal("disconnected"),
  connectionId: ConnectionId,
});
export type DisconnectedEvent = z.infer<typeof DisconnectedEvent>;

export const ConnectionEvent = z.discriminatedUnion("type", [
  ConnectedEvent,
  DisconnectedEvent,
]);
export type ConnectionEvent = z.infer<typeof ConnectionEvent>;

export const SyncConnectionsResponse = z
  .object({ connections: ConnectionDetails.array() })
  .or(ConnectionEvent);
export type SyncConnectionsResponse = z.infer<typeof SyncConnectionsResponse>;

export const EventType = z.union([
  z.literal("syncWorld"),
  z.literal("syncConnections"),
]);
export type EventType = z.infer<typeof EventType>;

export const Events = {
  syncWorld: {
    request: z.object({}),
    response: WorldDidChangeResponse,
  },
  syncConnections: {
    request: z.object({}),
    response: SyncConnectionsResponse,
  },
} as const satisfies {
  [K in EventType]: { request: z.ZodObject; response: z.ZodType };
};

export type EventParams<E extends EventType> = z.output<
  (typeof Events)[E]["request"]
>;

export type EventResponse<E extends EventType> = z.output<
  (typeof Events)[E]["response"]
>;
