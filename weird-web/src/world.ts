import unreachable from "ts-unreachable";
import type { NodeId, SyncChange } from "./message";

export class World {
  #nodes: Record<NodeId, Node> = {};

  applyChanges(changes: SyncChange[]) {
    for (const change of changes) {
      switch (change.$tag) {
        case "DidInsert": {
          const id = change.$value.id;
          const parent = change.$value.parent ?? undefined;
          const newNode: Node =
            typeof change.$value.node === "string"
              ? {
                  type: "text",
                  parent,
                  text: change.$value.node,
                }
              : {
                  type: "element",
                  parent,
                  class: change.$value.node.$tag,
                  attributes: change.$value.node.$value,
                  children: [],
                };
          if (this.#nodes[id] != null) {
            throw new Error(
              `tried to insert node ${id} but a node with that ID already exists`,
            );
          }

          if (parent) {
            const parentNode = this.#nodes[parent];
            if (parentNode == null) {
              throw new Error(
                `tried to insert node ${id} but parent ${parent} not found`,
              );
            }
            switch (parentNode?.type) {
              case "element":
                parentNode.children.push(id);
                break;
              case "text":
                throw new Error(
                  `cannot add node ${id} as a child of ${parent}`,
                );
              default:
                return unreachable(parentNode);
            }
          }

          this.#nodes[id] = newNode;
          break;
        }
        default:
          return unreachable(change.$tag);
      }
    }
  }

  printNodes() {
    console.info(this.#nodes);
  }
}

export type Node = Element | Text;

export interface Element {
  type: "element";
  parent: NodeId | undefined;
  class: string | undefined;
  children: NodeId[];
  attributes: {
    [attr: string]: unknown;
  };
}

export interface Text {
  type: "text";
  parent: NodeId | undefined;
  text: string;
}
