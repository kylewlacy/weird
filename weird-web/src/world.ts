import unreachable from "ts-unreachable";
import { NodeId, type SyncChange } from "./message";

const ROOT_NODE_ID = NodeId.parse("0");

export class World {
  #nodes: Record<NodeId, WorldNode> = {
    [ROOT_NODE_ID]: {
      type: "element",
      class: "World",
      attributes: {},
      children: [],
      dom: document.createElement("div"),
      parent: undefined,
    },
  };

  mount(element: HTMLElement) {
    const rootNode = this.#nodes[ROOT_NODE_ID];
    if (rootNode == null) {
      throw new Error("root node not found");
    }

    element.appendChild(rootNode.dom);
  }

  applyChanges(changes: SyncChange[]) {
    for (const change of changes) {
      switch (change.$tag) {
        case "DidInsert": {
          const id = change.$value.id;
          const parent = change.$value.parent ?? undefined;
          const worldNode: WorldNode =
            typeof change.$value.node === "string"
              ? {
                  type: "text",
                  parent: change.$value.parent ?? undefined,
                  text: change.$value.node,
                  dom: document.createTextNode(change.$value.node),
                }
              : {
                  type: "element",
                  parent: change.$value.parent ?? undefined,
                  class: change.$value.node.$tag,
                  attributes: change.$value.node.$value,
                  children: [],
                  dom: document.createElement("div"),
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
                parentNode.dom.appendChild(worldNode.dom);
                break;
              case "text":
                throw new Error(
                  `cannot add node ${id} as a child of ${parent}`,
                );
              default:
                return unreachable(parentNode);
            }
          }

          this.#nodes[id] = worldNode;

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

export type WorldNode = WorldElement | WorldText;

export interface WorldElement {
  type: "element";
  parent: NodeId | undefined;
  class: string | undefined;
  children: NodeId[];
  attributes: {
    [attr: string]: unknown;
  };
  dom: HTMLElement;
}

export interface WorldText {
  type: "text";
  parent: NodeId | undefined;
  text: string;
  dom: Text;
}
