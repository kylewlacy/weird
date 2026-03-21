import unreachable from "ts-unreachable";
import { NodeId, type WorldDidChangeEvent } from "./protocol/types.ts";
import {
  ELEMENTS,
  type WeirdElement,
  type WeirdElementClass,
} from "./elements";

export const ROOT_NODE_ID = NodeId.parse("0");

export class World {
  #rootNode = createElement("World", {}, undefined);

  nodes: Record<NodeId, WorldNode> = {
    [ROOT_NODE_ID]: this.#rootNode,
  };

  mount(element: HTMLElement) {
    element.appendChild(this.#rootNode.element.dom);
  }

  handleWorldDidChangeEvent(event: WorldDidChangeEvent) {
    const removeQueue = event.removed;
    while (true) {
      const removed = removeQueue.shift();
      if (removed == null) {
        break;
      }

      const worldNode = this.nodes[removed];
      if (worldNode == null) {
        console.warn(
          "got WorldDidChange event, but node does not exist",
          removed,
        );
        continue;
      }

      delete this.nodes[removed];

      const parentNode =
        worldNode.parent != null ? this.nodes[worldNode.parent] : null;
      switch (parentNode?.type) {
        case "element": {
          const childIndex = parentNode.children.indexOf(removed);
          if (childIndex !== -1 && parentNode.element.domSlot != null) {
            parentNode.children.splice(childIndex, 1);
            parentNode.element.domSlot.removeChild(worldNode.dom);
          } else {
            console.warn("node not found within parent element", {
              worldNode,
              parentNode,
            });
          }
          break;
        }
        case "text":
          console.warn("invalid parent node type", { worldNode, parentNode });
          break;
        case null:
        case undefined:
          // Parent node not found. This is expected since we're removing
          // nodes outside-in
          break;
        default:
          return unreachable(parentNode);
      }

      switch (worldNode.type) {
        case "element":
          removeQueue.push(...worldNode.children);
          break;
        case "text":
          break;
        default:
          return unreachable(worldNode);
      }
    }

    for (const inserted of event.inserted) {
      let worldNode: WorldNode;
      if (typeof inserted.node === "string") {
        worldNode = {
          type: "text",
          parent: inserted.parent,
          text: inserted.node,
          dom: document.createTextNode(inserted.node),
        };
      } else {
        worldNode = createElement(
          inserted.node.tag,
          inserted.node.attributes,
          inserted.parent,
        );
      }
      if (this.nodes[inserted.id] != null) {
        throw new Error(
          `tried to insert node ${inserted.id} but a node with that ID already exists`,
        );
      }

      const parentNode = this.nodes[inserted.parent];
      if (parentNode == null) {
        throw new Error(
          `tried to insert node ${inserted.id} but parent ${inserted.parent} not found`,
        );
      }
      switch (parentNode?.type) {
        case "element":
          if (parentNode.element.domSlot == null) {
            throw new Error(
              `tried to insert node ${inserted.id} but parent ${inserted.parent} has no DOM slot`,
            );
          }
          parentNode.children.push(inserted.id);
          parentNode.element.domSlot.appendChild(worldNode.dom);
          break;
        case "text":
          throw new Error(
            `cannot add node ${inserted.id} as a child of ${parent}`,
          );
        default:
          return unreachable(parentNode);
      }

      this.nodes[inserted.id] = worldNode;
    }
  }

  printNodes() {
    console.info(this.nodes);
  }
}

export type WorldNode = WorldElement | WorldText;

export interface WorldElement {
  type: "element";
  tag: string;
  attributes: object;
  children: NodeId[];
  parent: NodeId | undefined;
  element: WeirdElement;
  dom: Node;
}

export interface WorldText {
  type: "text";
  parent: NodeId | undefined;
  text: string;
  dom: Text;
}

function createElement(
  tag: string,
  attributes: object,
  parent: NodeId | undefined,
): WorldElement {
  let elementClass: WeirdElementClass | undefined =
    tag in ELEMENTS ? ELEMENTS[tag as keyof typeof ELEMENTS] : undefined;
  if (elementClass == undefined) {
    elementClass = ELEMENTS.UnknownElement;
  }
  const element = new elementClass(attributes);
  return {
    type: "element",
    element,
    tag,
    attributes,
    children: [],
    parent,
    get dom(): Node {
      return this.element.dom;
    },
  };
}
