import unreachable from "ts-unreachable";
import { NodeId, type WorldDidChangeResponse } from "./protocol/types.ts";
import {
  ELEMENTS,
  type WeirdElement,
  type WeirdElementClass,
} from "./elements";
import { h, type WeirdElementContext } from "./elements/utils.ts";

export const ROOT_NODE_ID = NodeId.parse("0");

export class World {
  onTriggerEvent?: (id: NodeId, event: string, params: unknown) => void;

  #rootNode = createElement({
    id: ROOT_NODE_ID,
    tag: "World",
    attributes: {},
    parentId: null,
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

  handleWorldDidChangeEvent(event: WorldDidChangeResponse) {
    for (const change of event.changes) {
      switch (change.type) {
        case "created": {
          let worldNode: WorldNode;
          if (typeof change.node === "string") {
            const domText = document.createTextNode(change.node);
            const dom = h("span", {}, domText);
            worldNode = {
              type: "text",
              parentId: change.parentId,
              text: change.node,
              dom,
              domText,
            };
          } else {
            worldNode = createElement({
              id: change.id,
              tag: change.node.tag,
              attributes: change.node.attributes,
              parentId: change.parentId,
              triggerEvent: (id: NodeId, event: string, params: unknown) => {
                this.onTriggerEvent?.(id, event, params);
              },
            });
          }
          if (this.nodes[change.id] != null) {
            throw new Error(
              `tried to create node ${change.id} but a node with that ID already exists`,
            );
          }

          const parentNode = this.nodes[change.parentId];
          if (parentNode?.type !== "element") {
            throw new Error(
              `tried to create node ${change.id} but could not find valid parent with ID ${change.parentId}`,
            );
          }

          const siblingNode =
            change.beforeSiblingId != null
              ? this.nodes[change.beforeSiblingId]
              : undefined;
          if (siblingNode != null && siblingNode.parentId != change.parentId) {
            throw new Error(
              `tried to create node ${change.id} but sibling node ${change.beforeSiblingId} doesn't have the same parent (node has parent ${change.parentId} but sibling has parent ${siblingNode.parentId})`,
            );
          }

          parentNode.element.domSlot?.insertBefore(
            worldNode.dom,
            siblingNode?.dom ?? null,
          );
          parentNode.children.add(change.id);
          this.nodes[change.id] = worldNode;

          switch (worldNode.type) {
            case "element":
              worldNode.element.didInsert();
              break;
            case "text":
              break;
            default:
              return unreachable(worldNode);
          }
          break;
        }
        case "updated": {
          const node = this.nodes[change.id];
          if (node == null) {
            throw new Error(`updated node ${change.id} not found`);
          }
          switch (node.type) {
            case "element": {
              const newAttrs: Record<string, unknown> = {
                ...node.attributes,
                ...change.setAttributes,
              };
              for (const key of change.clearAttributes ?? []) {
                delete newAttrs[key];
              }
              node.element.updateAttributes(newAttrs);
              node.attributes = newAttrs;
              break;
            }
            case "text": {
              if (change.text != null) {
                node.text = change.text;
                node.dom.textContent = change.text;
              }
              break;
            }
            default:
              return unreachable(node);
          }
          break;
        }
        case "moved": {
          const worldNode = this.nodes[change.id];
          if (worldNode == null) {
            throw new Error("tried to move node, but it doesn't exist");
          }

          const parentNode = this.nodes[change.parentId];
          if (parentNode?.type !== "element") {
            throw new Error(
              `tried to create node ${change.id} but could not find valid parent with ID ${change.parentId}`,
            );
          }

          const siblingNode =
            change.beforeSiblingId != null
              ? this.nodes[change.beforeSiblingId]
              : undefined;
          if (siblingNode != null && siblingNode.parentId != change.parentId) {
            throw new Error(
              `tried to move node ${change.id} but sibling node ${change.beforeSiblingId} doesn't have the right parent (node moved to ${change.parentId} but sibling has parent ${siblingNode.parentId})`,
            );
          }

          const oldParentNodeId = worldNode.parentId;
          const oldParentNode =
            worldNode.parentId != null
              ? this.nodes[worldNode.parentId]
              : undefined;
          if (worldNode.parentId == null || oldParentNode?.type !== "element") {
            throw new Error(
              "previous valid parent not found while moving node",
            );
          }

          parentNode.element.domSlot?.insertBefore(
            worldNode.dom,
            siblingNode?.dom ?? null,
          );

          if (oldParentNodeId != change.parentId) {
            oldParentNode.children.delete(change.id);
            parentNode.children.add(change.id);
          }
          worldNode.parentId = change.parentId;

          break;
        }
        case "deleted": {
          const deleteQueue = [change.id];
          while (true) {
            const removed = deleteQueue.shift();
            if (removed == null) {
              break;
            }

            const worldNode = this.nodes[removed];
            if (worldNode == null) {
              // Node already deleted
              continue;
            }

            if (worldNode.parentId == null) {
              console.warn("tried to remove node with no parent", {
                worldNode,
                removed,
              });
              continue;
            }

            const parentNode = this.nodes[worldNode.parentId];
            switch (parentNode?.type) {
              case "element": {
                parentNode.children.delete(change.id);
                parentNode.element.domSlot?.removeChild(worldNode.dom);
                break;
              }
              case "text":
                console.warn("invalid parent node type", {
                  worldNode,
                  parentNode,
                });
                break;
              case undefined:
                // Parent node not found. This can happen if the parent node
                // was already removed.
                break;
              default:
                return unreachable(parentNode);
            }

            switch (worldNode.type) {
              case "element":
                worldNode.element.didRemove();
                deleteQueue.push(...worldNode.children);
                break;
              case "text":
                break;
              default:
                return unreachable(worldNode);
            }
            delete this.nodes[removed];
          }
          break;
        }
        default:
          return unreachable(change);
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
  parentId: NodeId | null;
  children: Set<NodeId>;
  element: WeirdElement;
  dom: Node;
}

export interface WorldText {
  type: "text";
  parentId: NodeId | null;
  text: string;
  dom: HTMLElement;
  domText: Text;
}

interface CreateElementOptions {
  id: NodeId;
  tag: string;
  attributes: object;
  parentId: NodeId | null;
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
    parentId: opts.parentId,
    children: new Set(),
    get dom(): Node {
      return this.element.dom;
    },
  };
}
