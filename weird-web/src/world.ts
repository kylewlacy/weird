import unreachable from "ts-unreachable";
import { NodeId, type WorldDidChangeEvent } from "./message.ts";
import { Frame } from "./elements/Frame.ts";

const ROOT_NODE_ID = NodeId.parse("0");

export class World {
  #nodes: Record<NodeId, WorldNode> = {
    [ROOT_NODE_ID]: {
      type: "element",
      class: "World",
      attributes: {},
      children: [],
      dom: createDomElement("World"),
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

  handleWorldDidChangeEvent(event: WorldDidChangeEvent) {
    const removeQueue = event.removed;
    while (true) {
      const removed = removeQueue.shift();
      if (removed == null) {
        break;
      }

      const worldNode = this.#nodes[removed];
      if (worldNode == null) {
        console.warn(
          "got WorldDidChange event, but node does not exist",
          removed,
        );
        continue;
      }

      delete this.#nodes[removed];

      const parentNode =
        worldNode.parent != null ? this.#nodes[worldNode.parent] : null;
      switch (parentNode?.type) {
        case "element": {
          const childIndex = parentNode.children.indexOf(removed);
          if (childIndex !== -1) {
            parentNode.children.splice(childIndex, 1);
            parentNode.dom.removeChild(worldNode.dom);
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
        worldNode = {
          type: "element",
          parent: inserted.parent,
          class: inserted.node.$tag,
          attributes: inserted.node.$value,
          children: [],
          dom: createDomElement(inserted.node.$tag),
        };
      }
      if (this.#nodes[inserted.id] != null) {
        throw new Error(
          `tried to insert node ${inserted.id} but a node with that ID already exists`,
        );
      }

      const parentNode = this.#nodes[inserted.parent];
      if (parentNode == null) {
        throw new Error(
          `tried to insert node ${inserted.id} but parent ${inserted.parent} not found`,
        );
      }
      switch (parentNode?.type) {
        case "element":
          parentNode.children.push(inserted.id);
          parentNode.dom.appendChild(worldNode.dom);
          break;
        case "text":
          throw new Error(
            `cannot add node ${inserted.id} as a child of ${parent}`,
          );
        default:
          return unreachable(parentNode);
      }

      this.#nodes[inserted.id] = worldNode;
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

interface WorldElementClass {
  name: string;
}

const ELEMENTS = {
  World: {
    name: "div",
  },
  ProgressBar: {
    name: "div",
  },
  Other: {
    name: "span",
  },
  Frame,
} as const satisfies Record<string, WorldElementClass>;

function createDomElement(className: string): HTMLElement {
  const elementClass =
    className in ELEMENTS
      ? ELEMENTS[className as keyof typeof ELEMENTS]
      : undefined;
  if (elementClass != null) {
    return document.createElement(elementClass.name);
  } else {
    // TODO: Show an error
    return document.createElement("div");
  }
}
