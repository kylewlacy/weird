import unreachable from "ts-unreachable";
import { NodeId, type WorldDidChangeEvent } from "./protocol/types.ts";
import {
  ELEMENTS,
  type WeirdElement,
  type WeirdElementClass,
} from "./elements";
import type { WeirdElementContext } from "./elements/utils.ts";

export const ROOT_NODE_ID = NodeId.parse("0");

export class World {
  onTriggerEvent?: (id: NodeId, event: string, params: unknown) => void;

  #rootNode = createElement({
    id: ROOT_NODE_ID,
    tag: "World",
    attributes: {},
    parentId: undefined,
    triggerEvent: (id, event, params) => {
      this.onTriggerEvent?.(id, event, params);
    },
  });

  nodes: Record<NodeId, WorldNode> = {
    [ROOT_NODE_ID]: this.#rootNode,
  };

  mount(element: HTMLElement) {
    element.appendChild(this.#rootNode.element.dom);
  }

  handleWorldDidChangeEvent(event: WorldDidChangeEvent) {
    const removeQueue = [...event.removed];
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

      switch (worldNode.type) {
        case "element":
          worldNode.element.didRemove();
          break;
        case "text":
          break;
        default:
          return unreachable(worldNode);
      }

      delete this.nodes[removed];

      const parentNode =
        worldNode.parentId != null ? this.nodes[worldNode.parentId] : null;
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
      if (inserted.node == null) {
        throw new Error("TODO: Move node");
      } else {
        let worldNode: WorldNode;
        if (typeof inserted.node === "string") {
          worldNode = {
            type: "text",
            parentId: inserted.parentId,
            text: inserted.node,
            dom: document.createTextNode(inserted.node),
          };
        } else {
          worldNode = createElement({
            id: inserted.id,
            tag: inserted.node.tag,
            attributes: inserted.node.attributes,
            parentId: inserted.parentId,
            triggerEvent: (id: NodeId, event: string, params: unknown) => {
              this.onTriggerEvent?.(id, event, params);
            },
          });
        }
        if (this.nodes[inserted.id] != null && inserted.node != null) {
          throw new Error(
            `tried to insert node ${inserted.id} but a node with that ID already exists`,
          );
        }

        const parentNode = this.nodes[inserted.parentId];
        if (parentNode == null) {
          throw new Error(
            `tried to insert node ${inserted.id} but parent ${inserted.parentId} not found`,
          );
        }
        switch (parentNode?.type) {
          case "element":
            if (parentNode.element.domSlot == null) {
              throw new Error(
                `tried to insert node ${inserted.id} but parent ${inserted.parentId} has no DOM slot`,
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

    for (const updated of event.updated) {
      const node = this.nodes[updated.id];
      if (node == null) {
        throw new Error(`updated node ${updated.id} not found`);
      }
      switch (node.type) {
        case "element": {
          const newAttrs: Record<string, unknown> = {
            ...node.attributes,
            ...updated.setAttributes,
          };
          for (const key of updated.clearAttributes ?? []) {
            delete newAttrs[key];
          }
          node.element.updateAttributes(newAttrs);
          node.attributes = newAttrs;
          break;
        }
        case "text": {
          if (updated.text != null) {
            node.text = updated.text;
            node.dom.textContent = updated.text;
          }
          break;
        }
        default:
          return unreachable(node);
      }
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
  parentId: NodeId | undefined;
  element: WeirdElement;
  dom: Node;
}

export interface WorldText {
  type: "text";
  parentId: NodeId | undefined;
  text: string;
  dom: Text;
}

interface CreateElementOptions {
  id: NodeId;
  tag: string;
  attributes: object;
  parentId: NodeId | undefined;
  triggerEvent(id: NodeId, event: string, params: unknown): void;
}

function createElement(opts: CreateElementOptions): WorldElement {
  let elementClass: WeirdElementClass | undefined =
    opts.tag in ELEMENTS
      ? ELEMENTS[opts.tag as keyof typeof ELEMENTS]
      : undefined;
  if (elementClass == undefined) {
    elementClass = ELEMENTS.UnknownElement;
  }

  const ctx = {
    triggerEvent(event, params) {
      opts.triggerEvent(opts.id, event, params);
    },
  } satisfies WeirdElementContext;

  const element = new elementClass(opts.attributes, ctx);
  return {
    type: "element",
    element,
    tag: opts.tag,
    attributes: opts.attributes,
    children: [],
    parentId: opts.parentId,
    get dom(): Node {
      return this.element.dom;
    },
  };
}
