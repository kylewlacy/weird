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

export const ServerMessage = z.union([
  z.object({
    id: z.string(),
    event: z.discriminatedUnion("$tag", [
      z.object({
        $tag: z.literal("WorldDidChange"),
        $value: WorldDidChangeEvent,
      }),
    ]),
  }),
]);
export type ServerMessage = z.infer<typeof ServerMessage>;
