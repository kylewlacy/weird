import * as z from "zod";

export const NodeId = z.string().brand("NodeId");
export type NodeId = z.infer<typeof NodeId>;

export const FlatElement = z.object({
  $tag: z.string(),
  $value: z.record(z.string(), z.string()),
});
export type FlatElement = z.infer<typeof FlatElement>;

export const FlatNode = z.union([z.string(), FlatElement]);
export type FlatNode = z.infer<typeof FlatNode>;

export const SyncChange = z.discriminatedUnion("$tag", [
  z.object({
    $tag: z.literal("DidInsert"),
    $value: z.object({
      id: NodeId,
      parent: NodeId,
      node: FlatNode,
    }),
  }),
]);
export type SyncChange = z.infer<typeof SyncChange>;

export const SyncWorldResponse = z.object({
  requestId: z.string(),
  changes: SyncChange.array(),
});
export type SyncWorldResponse = z.infer<typeof SyncWorldResponse>;

export const ServerMessage = z.union([
  z.object({ syncWorld: SyncWorldResponse }),
]);
export type ServerMessage = z.infer<typeof ServerMessage>;
