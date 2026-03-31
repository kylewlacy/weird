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

export class Debugger {
  #tree: HTMLElement;
  #nodes: Record<NodeId, TreeNode>;

  #view: HTMLElement;
  #element: HTMLElement;

  constructor() {
    const rootNode = treeNode({ tag: "World", attributes: {} });

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
  // Inspired by: https://iamkate.com/code/tree-views/
  const styleLi = clsx(
    "block relative pl-9 border-l-2 border-gray-400 last:border-transparent before:content-[''] before:block before:absolute before:-top-3 before:-left-0.5 before:w-6.5 before:h-6.25 before:border-gray-400 before:border-b-2 before:border-l-2",
  );
  const styleUl = clsx("-ml-3.5 pl-0");

  if (typeof node === "string") {
    return {
      kind: "leaf",
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
