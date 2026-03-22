import { h } from "./elements/utils.ts";
import type {
  FlatNode,
  NodeId,
  WorldDidChangeEvent,
} from "./protocol/types.ts";
import { ROOT_NODE_ID } from "./world.ts";
import { ELEMENTS } from "./elements/index.ts";

export class Debugger {
  #tree: HTMLElement;
  #nodes: Record<NodeId, TreeNode>;

  #view: HTMLElement;
  #element: HTMLElement;

  constructor() {
    const rootNode = treeNode({ tag: "World", attributes: {} });

    const closeButton = h("button", { popoverTargetAction: "hide" }, "Close");

    this.#nodes = { [ROOT_NODE_ID]: rootNode };
    this.#tree = h(
      "ul",
      {
        className: "weird-debugger-tree",
        style: { overflow: "hidden", width: "100%" },
      },
      rootNode.dom,
    );
    this.#view = h(
      "div",
      {
        className: "weird-debugger-view",
        popover: "manual",
        style: {
          backgroundColor: "#c19d6e",
          padding: "0.5rem",
          border: "1px solid #4f3f2b",
          position: "fixed",
          margin: "0",
          inset: "auto",
          positionArea: "bottom span-left",
          maxHeight: "100%",
          width: "40%",
        },
      },
      closeButton,
      h("div", {}, "Tree", this.#tree),
    );
    this.#element = h(
      "div",
      {},
      h(
        "button",
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
      if (removedNode) {
        removedNode.dom.remove();
      } else {
        console.warn("[Debugger] Failed to remove node", { removed });
      }
      delete this.#nodes[removed];
    }

    for (const inserted of event.inserted) {
      const parentNode = this.#nodes[inserted.parent];
      const newNode = treeNode(inserted.node);
      this.#nodes[inserted.id] = newNode;

      if (parentNode?.kind === "tree") {
        parentNode.domSlot.appendChild(newNode.dom);
      } else {
        console.warn("[Debugger] Failed to insert node", { inserted });
      }
    }
  }
}

type TreeNode =
  | { kind: "tree"; dom: HTMLElement; domSlot: HTMLElement }
  | { kind: "leaf"; dom: HTMLElement };

function treeNode(node: FlatNode): TreeNode {
  if (typeof node === "string") {
    return {
      kind: "leaf",
      dom: h(
        "li",
        {
          title: node,
        },
        h(
          "span",
          {
            style: { whiteSpace: "noWrap" },
          },
          h(
            "span",
            {
              style: { fontFamily: "monospace, monospace" },
            },
            "Text",
          ),
          h(
            "span",
            {
              style: {
                fontSize: "0.75rem",
                color: "#444",
                marginLeft: "0.5rem",
              },
            },
            JSON.stringify(node),
          ),
        ),
      ),
    };
  } else {
    const domSlot = h("ul");
    return {
      kind: "tree",
      dom: h(
        "li",
        {
          className: "tree",
        },
        h(
          "details",
          {},
          h(
            "summary",
            {},
            h(
              "span",
              { style: { fontFamily: "monospace, monospace" } },
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
