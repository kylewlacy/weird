import { h } from "./elements/utils.ts";
import type {
  FlatNode,
  NodeId,
  WorldDidChangeResponse,
} from "./protocol/types.ts";
import { ROOT_NODE_ID } from "./world.ts";
import { ELEMENTS } from "./elements/index.ts";
import clsx from "clsx";
import unreachable from "ts-unreachable";

export class Debugger {
  #tree: HTMLElement;
  #nodes: Record<NodeId, TreeNode>;

  #dom: HTMLElement;

  constructor() {
    const rootNode = treeNode({ tag: "World", attributes: {} }, null);

    this.#nodes = { [ROOT_NODE_ID]: rootNode };
    this.#tree = h(
      "ul",
      {
        className: clsx("overflow-hidden w-full leading-6"),
      },
      rootNode.dom,
    );
    this.#dom = h("div", {}, "Tree", this.#tree);
  }

  mount(element: HTMLElement) {
    element.appendChild(this.#dom);
  }

  handleWorldDidChangeEvent(event: WorldDidChangeResponse) {
    for (const change of event.changes) {
      switch (change.type) {
        case "created": {
          if (this.#nodes[change.id] != null) {
            console.warn(
              "[Debugger] Tried to create node, but a node with the same ID already exists",
              { change, node: this.#nodes[change.id] },
            );
            continue;
          }

          const parentNode = this.#nodes[change.parentId];
          if (parentNode?.kind !== "tree") {
            console.warn(
              "[Debugger] Valid parent node couldn't be found when creating node",
              { change, parentNode },
            );
            continue;
          }

          const siblingNode =
            change.beforeSiblingId != null
              ? this.#nodes[change.beforeSiblingId]
              : undefined;
          if (siblingNode != null && siblingNode.parentId != change.parentId) {
            console.warn(
              "[Debugger] Tried to create node but sibling node has a different parent",
              { change, siblingNode },
            );
            continue;
          }

          const newNode = treeNode(change.node, change.parentId);

          parentNode.domSlot.insertBefore(
            newNode.dom,
            siblingNode?.dom ?? null,
          );
          parentNode.children.add(change.id);
          this.#nodes[change.id] = newNode;
          break;
        }
        case "updated": {
          break;
        }
        case "moved": {
          const node = this.#nodes[change.id];
          if (node == null) {
            console.warn("[Debugger] Failed to move node", { change });
            continue;
          }

          const parentNode = this.#nodes[change.parentId];
          if (parentNode?.kind !== "tree") {
            console.warn(
              "[Debugger] Failed to get valid parent node when moving",
              {
                change,
                parentNode,
              },
            );
            continue;
          }

          const siblingNode =
            change.beforeSiblingId != null
              ? this.#nodes[change.beforeSiblingId]
              : undefined;
          if (siblingNode != null && siblingNode.parentId != change.parentId) {
            console.warn(
              `[Debugger] Sibling had incorrect parent while moving node`,
              { change, siblingNode, node },
            );
            continue;
          }

          const oldParentNodeId = node.parentId;
          const oldParentNode =
            oldParentNodeId != null ? this.#nodes[oldParentNodeId] : undefined;
          if (
            oldParentNodeId == null ||
            oldParentNode == null ||
            oldParentNode?.kind !== "tree"
          ) {
            console.warn(
              "[Debugger] Failed to get valid old parent node when moving",
              { change, node, oldParentNode },
            );
            continue;
          }

          parentNode.domSlot.insertBefore(node.dom, siblingNode?.dom ?? null);

          if (oldParentNodeId != change.parentId) {
            oldParentNode.children.delete(change.id);
            parentNode.children.add(change.id);
          }
          node.parentId = change.parentId;
          break;
        }
        case "deleted": {
          const deleteQueue = [change.id];
          while (true) {
            const removed = deleteQueue.shift();
            if (removed == null) {
              break;
            }

            const treeNode = this.#nodes[removed];
            if (treeNode == null) {
              // Node already deleted
              continue;
            }

            if (treeNode.parentId == null) {
              console.warn("[Debugger] Tried to remove node with no parent", {
                treeNode,
                removed,
              });
              continue;
            }

            const parentNode = this.#nodes[treeNode.parentId];
            switch (parentNode?.kind) {
              case "tree": {
                parentNode.children.delete(change.id);
                parentNode.domSlot.removeChild(treeNode.dom);
                break;
              }
              case "leaf":
                console.warn("[Debugger] Invalid parent node type", {
                  treeNode,
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

            if (treeNode.kind === "tree") {
              deleteQueue.push(...treeNode.children);
            }
            delete this.#nodes[removed];
          }
          break;
        }
        default: {
          return unreachable(change);
        }
      }
    }
  }
}

type TreeNode =
  | {
      kind: "leaf";
      parentId: NodeId | null;
      dom: HTMLElement;
    }
  | {
      kind: "tree";
      parentId: NodeId | null;
      children: Set<NodeId>;
      dom: HTMLElement;
      domSlot: HTMLElement;
    };

function treeNode(node: FlatNode, parentId: NodeId | null): TreeNode {
  // Inspired by: https://iamkate.com/code/tree-views/
  const styleLi = clsx(
    "block relative pl-9 border-l-2 border-gray-400 last:border-transparent before:content-[''] before:block before:absolute before:-top-3 before:-left-0.5 before:w-6.5 before:h-6.25 before:border-gray-400 before:border-b-2 before:border-l-2 dark:border-gray-600 dark:before:border-gray-600",
  );
  const styleUl = clsx("-ml-3.5 pl-0");

  if (typeof node === "string") {
    return {
      kind: "leaf",
      parentId,
      dom: h(
        "li",
        {
          title: node,
          className: styleLi,
        },
        h(
          "span",
          {
            className: clsx("whitespace-nowrap"),
          },
          h(
            "span",
            {
              style: {
                fontFamily: "monospace, monospace",
              },
            },
            "Text",
          ),
          h(
            "span",
            {
              className: clsx("text-xs ml-2 text-gray-600 dark:text-gray-400"),
            },
            JSON.stringify(node),
          ),
        ),
      ),
    };
  } else {
    const domSlot = h("ul", { className: styleUl });
    return {
      kind: "tree",
      parentId,
      children: new Set(),
      dom: h(
        "li",
        {
          className: styleLi,
        },
        h(
          "details",
          {},
          h(
            "summary",
            {},
            h(
              "span",
              {
                style: { fontFamily: "monospace, monospace" },
              },
              `<${node.tag}>`,
            ),
            node.tag in ELEMENTS
              ? null
              : h(
                  "span",
                  {
                    style: {
                      marginLeft: "0.5rem",
                      color: "#444",
                      fontSize: "0.75rem",
                    },
                  },
                  "(unknown tag)",
                ),
          ),
          domSlot,
        ),
      ),
      domSlot,
    };
  }
}
