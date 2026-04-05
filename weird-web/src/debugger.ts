import { h } from "./elements/utils.ts";
import type {
  FlatNode,
  NodeId,
  WorldDidChangeEvent,
} from "./protocol/types.ts";
import { ROOT_NODE_ID } from "./world.ts";
import { ELEMENTS } from "./elements/index.ts";
import clsx from "clsx";
import { buttonComponent } from "./elements/Button.ts";
import unreachable from "ts-unreachable";

export class Debugger {
  #tree: HTMLElement;
  #nodes: Record<NodeId, TreeNode>;

  #view: HTMLElement;
  #element: HTMLElement;

  constructor() {
    const rootNode = treeNode({ tag: "World", attributes: {} }, null);

    const closeButton = buttonComponent(
      {
        popoverTargetAction: "hide",
      },
      "Close",
    );

    this.#nodes = { [ROOT_NODE_ID]: rootNode };
    this.#tree = h(
      "ul",
      {
        className: clsx("overflow-hidden w-full leading-6"),
      },
      rootNode.dom,
    );
    this.#view = h(
      "div",
      {
        className: clsx(
          "bg-orange-200 p-2 border border-orange-900 fixed m-0 inset-auto position-area-bottom-span-left max-h-full w-2/5 shadow-md dark:bg-amber-900 dark:border-amber-600 dark:shadow-lg dark:text-white",
        ),
        popover: "manual",
        style: {
          positionArea: "bottom span-left",
        },
      },
      closeButton,
      h("div", {}, "Tree", this.#tree),
    );
    this.#element = h(
      "div",
      {},
      buttonComponent(
        {
          popoverTargetElement: this.#view,
        },
        "Debugger",
      ),
      this.#view,
    );
    closeButton.popoverTargetElement = this.#view;
  }

  mount(element: HTMLElement) {
    element.appendChild(this.#element);
  }

  handleWorldDidChangeEvent(event: WorldDidChangeEvent) {
    for (const removed of event.removed) {
      const removedNode = this.#nodes[removed];
      if (removedNode != null) {
        removedNode.dom.remove();
      } else {
        console.warn("[Debugger] Failed to remove node", { removed });
      }

      if (removedNode?.parent != null) {
        const parentNode = this.#nodes[removedNode.parent.id];
        switch (parentNode?.kind) {
          case "tree": {
            parentNode.children.splice(removedNode.parent.index, 1);

            // Update the parent index for each sibling node
            for (
              let i = removedNode.parent.index + 1;
              i < parentNode.children.length;
              i++
            ) {
              const siblingNodeId = parentNode.children[i];
              const siblingNode =
                siblingNodeId != null ? this.#nodes[siblingNodeId] : undefined;
              if (siblingNode?.parent != null) {
                siblingNode.parent.index = i;
              }
            }

            break;
          }
          case "leaf": {
            console.warn("[Debugger] Failed to remove child node from parent", {
              removed,
              removedNode,
              parentNode,
            });
            break;
          }
          case undefined:
            // Parent node not found. This can happen if the parent node
            // was already removed.
            break;
          default:
            return unreachable(parentNode);
        }
      }
      delete this.#nodes[removed];
    }

    for (const inserted of event.inserted) {
      if (inserted.node == null) {
        const node = this.#nodes[inserted.id];
        if (node == null) {
          console.warn("[Debugger] Failed to move node", { inserted });
          continue;
        }

        const parentNode = this.#nodes[inserted.parentId];
        if (parentNode?.kind !== "tree") {
          console.warn(
            "[Debugger] Failed to get valid parent node when moving",
            {
              inserted,
              parentNode,
            },
          );
          continue;
        }

        const oldParentNode =
          node.parent != null ? this.#nodes[node.parent.id] : undefined;
        if (
          node.parent == null ||
          oldParentNode == null ||
          oldParentNode?.kind !== "tree"
        ) {
          console.warn(
            "[Debugger] Failed to get valid old parent node when moving",
            { inserted, node, oldParentNode },
          );
          continue;
        }
        const oldParentNodeId = node.parent.id;
        const oldParentNodeIndex = node.parent.index;

        if (
          inserted.parentId === node.parent?.id &&
          inserted.parentIndex === node.parent.index
        ) {
          console.info(
            "Got move message, but node is moving to it's current position",
            { inserted },
          );
          continue;
        }

        oldParentNode.children.splice(oldParentNodeIndex, 1);
        parentNode.children.splice(inserted.parentIndex, 0, inserted.id);

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
            siblingNodeId != null ? this.#nodes[siblingNodeId] : undefined;
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
              siblingNodeId != null ? this.#nodes[siblingNodeId] : undefined;
            if (siblingNode?.parent != null) {
              siblingNode.parent.id = inserted.parentId;
              siblingNode.parent.index = i;
            }
          }
        }

        const siblingId = parentNode.children[inserted.parentIndex + 1];
        const sibling = siblingId != null ? this.#nodes[siblingId] : undefined;
        parentNode.domSlot.insertBefore(node.dom, sibling?.dom ?? null);
      } else {
        const parentNode = this.#nodes[inserted.parentId];
        const newNode = treeNode(inserted.node, {
          id: inserted.parentId,
          index: inserted.parentIndex,
        });
        this.#nodes[inserted.id] = newNode;

        if (parentNode?.kind === "tree") {
          const siblingId = parentNode.children[inserted.parentIndex + 1];
          const sibling =
            siblingId != null ? this.#nodes[siblingId] : undefined;
          parentNode.domSlot.insertBefore(newNode.dom, sibling?.dom ?? null);
          parentNode.children.splice(inserted.parentIndex, 0, inserted.id);

          // Update the parent index for each sibling node
          for (
            let i = inserted.parentIndex + 1;
            i < parentNode.children.length;
            i++
          ) {
            const siblingNodeId = parentNode.children[i];
            const siblingNode =
              siblingNodeId != null ? this.#nodes[siblingNodeId] : undefined;
            if (siblingNode?.parent != null) {
              siblingNode.parent.index = i;
            }
          }
        } else {
          console.warn("[Debugger] Failed to insert node", { inserted });
        }
      }
    }
  }
}

type TreeNode =
  | {
      kind: "leaf";
      parent: TreeNodeParent | null;
      dom: HTMLElement;
    }
  | {
      kind: "tree";
      parent: TreeNodeParent | null;
      children: NodeId[];
      dom: HTMLElement;
      domSlot: HTMLElement;
    };

interface TreeNodeParent {
  id: NodeId;
  index: number;
}

function treeNode(node: FlatNode, parent: TreeNodeParent | null): TreeNode {
  // Inspired by: https://iamkate.com/code/tree-views/
  const styleLi = clsx(
    "block relative pl-9 border-l-2 border-gray-400 last:border-transparent before:content-[''] before:block before:absolute before:-top-3 before:-left-0.5 before:w-6.5 before:h-6.25 before:border-gray-400 before:border-b-2 before:border-l-2",
  );
  const styleUl = clsx("-ml-3.5 pl-0");

  if (typeof node === "string") {
    return {
      kind: "leaf",
      parent,
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
              className: clsx("text-xs text-gray-600 ml-2"),
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
      parent,
      children: [],
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
