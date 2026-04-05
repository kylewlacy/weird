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
    parent: null,
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
        // Node already deleted
        continue;
      }

      if (worldNode.parent == null) {
        console.warn("tried to remove node with no parent", {
          worldNode,
          removed,
        });
        continue;
      }

      const parentNode = this.nodes[worldNode.parent.id];
      switch (parentNode?.type) {
        case "element": {
          parentNode.children.splice(worldNode.parent.index);
          parentNode.element.domSlot?.removeChild(worldNode.dom);

          // Update the parent index for each sibling node
          for (
            let i = worldNode.parent.index;
            i < parentNode.children.length;
            i++
          ) {
            const siblingNodeId = parentNode.children[i];
            const siblingNode =
              siblingNodeId != null ? this.nodes[siblingNodeId] : undefined;
            if (siblingNode?.parent != null) {
              siblingNode.parent.index = i;
            }
          }

          break;
        }
        case "text":
          console.warn("invalid parent node type", { worldNode, parentNode });
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
          removeQueue.push(...worldNode.children);
          break;
        case "text":
          break;
        default:
          return unreachable(worldNode);
      }
      delete this.nodes[removed];
    }

    for (const inserted of event.inserted) {
      if (inserted.node == null) {
        // Move an existing node

        const worldNode = this.nodes[inserted.id];
        if (worldNode == null) {
          throw new Error("tried to move node, but it doesn't exist");
        }

        const parentNode = this.nodes[inserted.parentId];
        if (parentNode?.type !== "element") {
          throw new Error("valid parent not found while moving node");
        }

        if (parentNode.element.domSlot == null) {
          throw new Error(
            `tried to insert node ${inserted.id} but parent ${inserted.parentId} has no DOM slot`,
          );
        }

        const oldParentNode =
          worldNode.parent?.id != null
            ? this.nodes[worldNode.parent?.id]
            : undefined;
        if (worldNode.parent == null || oldParentNode?.type !== "element") {
          throw new Error("previous valid parent not found while moving node");
        }

        const oldParentNodeId = worldNode.parent.id;
        const oldParentNodeIndex = worldNode.parent.index;

        // Adjust the parent node indices for each child node.
        // TODO: This can be optimized by limiting the upper bound when
        // the old and new parent node are the same
        for (
          let i = oldParentNodeIndex;
          i < oldParentNode.children.length;
          i++
        ) {
          const siblingNodeId = oldParentNode.children[i];
          const siblingNode =
            siblingNodeId != null ? this.nodes[siblingNodeId] : undefined;
          if (siblingNode?.parent != null) {
            siblingNode.parent.id = oldParentNodeId;
            siblingNode.parent.index = i;
          }
        }
        if (oldParentNodeId != inserted.parentId) {
          for (
            let i = inserted.parentIndex;
            i < parentNode.children.length;
            i++
          ) {
            const siblingNodeId = parentNode.children[i];
            const siblingNode =
              siblingNodeId != null ? this.nodes[siblingNodeId] : undefined;
            if (siblingNode?.parent != null) {
              siblingNode.parent.id = inserted.parentId;
              siblingNode.parent.index = i;
            }
          }
        }

        oldParentNode.children.splice(oldParentNodeIndex, 1);
        parentNode.children.splice(inserted.parentIndex, 0, inserted.id);

        const siblingId = parentNode.children[inserted.parentIndex + 1];
        const sibling = siblingId != null ? this.nodes[siblingId] : undefined;
        parentNode.element.domSlot.insertBefore(
          worldNode.dom,
          sibling?.dom ?? null,
        );
      } else {
        // Insert a new node

        let worldNode: WorldNode;
        if (typeof inserted.node === "string") {
          worldNode = {
            type: "text",
            parent: {
              id: inserted.parentId,
              index: inserted.parentIndex,
            },
            text: inserted.node,
            dom: document.createTextNode(inserted.node),
          };
        } else {
          worldNode = createElement({
            id: inserted.id,
            tag: inserted.node.tag,
            attributes: inserted.node.attributes,
            parent: {
              id: inserted.parentId,
              index: inserted.parentIndex,
            },
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
            {
              if (parentNode.element.domSlot == null) {
                throw new Error(
                  `tried to insert node ${inserted.id} but parent ${inserted.parentId} has no DOM slot`,
                );
              }

              const siblingId = parentNode.children[inserted.parentIndex + 1];
              const sibling =
                siblingId != null ? this.nodes[siblingId] : undefined;
              parentNode.element.domSlot.insertBefore(
                worldNode.dom,
                sibling?.dom ?? null,
              );
              parentNode.children.splice(inserted.parentIndex, 0, inserted.id);
            }
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
  parent: WorldNodeParent | null;
  element: WeirdElement;
  dom: Node;
}

export interface WorldText {
  type: "text";
  parent: WorldNodeParent | null;
  text: string;
  dom: Text;
}

interface WorldNodeParent {
  id: NodeId;
  index: number;
}

interface CreateElementOptions {
  id: NodeId;
  tag: string;
  attributes: object;
  parent: WorldNodeParent | null;
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
    parent: opts.parent,
    get dom(): Node {
      return this.element.dom;
    },
  };
}
