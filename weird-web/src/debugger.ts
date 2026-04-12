import { h, type Children, type ElementProperties } from "./elements/utils.ts";
import type {
  ConnectionDetails,
  ConnectionEvent,
  ConnectionId,
  FlatNode,
  InitResponse,
  NodeId,
  WorldDidChangeResponse,
} from "./protocol/types.ts";
import { ROOT_NODE_ID } from "./world.ts";
import { ELEMENTS } from "./elements/index.ts";
import clsx from "clsx";
import unreachable from "ts-unreachable";
import { buttonComponent } from "./elements/Button.ts";

const DEBUGGER_TABS = [
  { label: "Tree", id: "tree" },
  { label: "Connections", id: "connections" },
] as const;
type DebuggerTabId = (typeof DEBUGGER_TABS)[number]["id"];

export class Debugger {
  #nodes: Record<NodeId, TreeNode>;
  #connections: ConnectionDetails[] = [];
  #connectionId: ConnectionId | undefined;
  #connectedConnections: HTMLUListElement;
  #disconnectedConnections: HTMLUListElement;
  #connectedConnectionsLabel: Text;
  #disconnectedConnectionsLabel: Text;
  #tabs: Record<DebuggerTabId, HTMLButtonElement>;
  #tabPanels: Record<DebuggerTabId, HTMLDivElement>;
  #selectedTab: DebuggerTabId = "tree";

  #dom: HTMLElement;

  constructor() {
    const rootNode = treeNode({ tag: "World", attributes: {} }, null);

    this.#tabs = Object.fromEntries(
      DEBUGGER_TABS.map((tab) => {
        const tabEl = buttonComponent(
          { ariaSelected: this.#selectedTab === tab.id ? "true" : "false" },
          tab.label,
        );
        tabEl.addEventListener("click", (e) => {
          e.preventDefault();
          this.#setSelectedTab(tab.id);
        });
        return [tab.id, tabEl];
      }),
    ) as Record<DebuggerTabId, HTMLButtonElement>;

    const panels = {
      tree: h(
        "ul",
        {
          className: clsx("overflow-hidden w-full leading-6"),
        },
        rootNode.dom,
      ),
      connections: h(
        "div",
        {},
        h(
          "details",
          {},
          h(
            "summary",
            { className: clsx("font-bold") },
            (this.#connectedConnectionsLabel =
              document.createTextNode("Connected (?)")),
          ),
          (this.#connectedConnections = h("ul", {
            className: clsx("list-disc list-inside pl-4"),
          })),
        ),
        h(
          "details",
          {},
          h(
            "summary",
            {},
            (this.#disconnectedConnectionsLabel =
              document.createTextNode("Disconnected (?)")),
          ),
          (this.#disconnectedConnections = h("ul", {
            className: clsx("list-disc list-inside pl-4"),
          })),
        ),
      ),
    } satisfies Record<DebuggerTabId, Node>;
    this.#tabPanels = Object.fromEntries(
      Object.entries(panels).map(([id, panel]) => {
        const tabPanel = h(
          "div",
          {
            role: "tabpanel",
            style: { display: id === this.#selectedTab ? "" : "none" },
          },
          panel,
        );
        tabPanel.ariaLabelledByElements = [this.#tabs[id as DebuggerTabId]];
        return [id, tabPanel];
      }),
    ) as Record<DebuggerTabId, HTMLDivElement>;

    this.#nodes = { [ROOT_NODE_ID]: rootNode };
    this.#dom = h(
      "div",
      {},
      h(
        "div",
        { role: "tablist", className: clsx("flex gap-2 mb-2") },
        ...Object.values(this.#tabs),
      ),
      ...Object.values(this.#tabPanels),
    );
  }

  #setSelectedTab(tabId: DebuggerTabId) {
    for (const [id, tabEl] of Object.entries(this.#tabs)) {
      tabEl.ariaSelected = id === tabId ? "true" : "false";
    }
    for (const [id, panelEl] of Object.entries(this.#tabPanels)) {
      panelEl.style.display = id === tabId ? "" : "none";
    }
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

  setConnections(connections: ConnectionDetails[], initResponse: InitResponse) {
    this.#connections = [...connections];
    this.#connectionId = initResponse.connectionId;
    const connectedEls: HTMLElement[] = [];
    const disconnectedEls: HTMLElement[] = [];

    for (const conn of connections) {
      const connEl = connectionEl({
        conn,
        isCurrent: conn.connectionId === initResponse.connectionId,
      });
      if (conn.connected) {
        connectedEls.push(connEl);
      } else {
        disconnectedEls.push(connEl);
      }
    }

    this.#connectedConnections.replaceChildren(...connectedEls);
    this.#disconnectedConnections.replaceChildren(...disconnectedEls);

    this.#connectedConnectionsLabel.textContent = `Connected (${connectedEls.length})`;
    this.#disconnectedConnectionsLabel.textContent = `Disconnected (${disconnectedEls.length})`;
  }

  handleConnectionEvent(event: ConnectionEvent) {
    // TODO: Handle connection events more efficiently, instead of recreating
    // the whole DOM!
    switch (event.type) {
      case "connected": {
        const conn: ConnectionDetails & { type?: string } = { ...event };
        delete conn.type;
        this.#connections.push(conn);
        this.#didUpdateConnections();
        break;
      }
      case "disconnected":
        for (const conn of this.#connections) {
          if (conn.connectionId === event.connectionId) {
            conn.connected = false;
          }
        }
        this.#didUpdateConnections();
        break;
      default:
        break;
    }
  }

  #didUpdateConnections() {
    const connectedEls: HTMLElement[] = [];
    const disconnectedEls: HTMLElement[] = [];

    for (const conn of this.#connections) {
      const connEl = connectionEl({
        conn,
        isCurrent: conn.connectionId === this.#connectionId,
      });
      if (conn.connected) {
        connectedEls.push(connEl);
      } else {
        disconnectedEls.push(connEl);
      }
    }

    this.#connectedConnections.replaceChildren(...connectedEls);
    this.#disconnectedConnections.replaceChildren(...disconnectedEls);

    this.#connectedConnectionsLabel.textContent = `Connected (${connectedEls.length})`;
    this.#disconnectedConnectionsLabel.textContent = `Disconnected (${disconnectedEls.length})`;
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

interface ConnectionElementOptions {
  conn: ConnectionDetails;
  isCurrent: boolean;
}

function connectionEl(opts: ConnectionElementOptions): HTMLLIElement {
  let sourceBadgeClass: string;
  switch (opts.conn.source.type) {
    case "websocket":
      sourceBadgeClass = clsx(
        "bg-violet-100 border-violet-400 dark:bg-violet-700 dark:border-violet-500",
      );
      break;
    case "unixSocket":
      sourceBadgeClass = clsx(
        "bg-green-100 border-green-400 dark:bg-green-700 dark:border-green-500",
      );
      break;
    default:
      sourceBadgeClass = clsx(
        "bg-zinc-100 border-zinc-400 dark:bg-zinc-700 dark:border-zinc-500",
      );
      break;
  }

  return h(
    "li",
    { className: clsx("flex gap-x-2 items-baseline py-0.5") },
    h(
      "span",
      {
        className: clsx(
          opts.conn.app != null
            ? undefined
            : "text-gray-600 dark:text-gray-400",
        ),
      },
      opts.conn.app ?? "(no name)",
    ),
    h(
      "span",
      { className: clsx("font-mono text-gray-600 dark:text-gray-400") },
      `id=${opts.conn.connectionId}`,
    ),
    opts.conn.client != null
      ? h(
          "span",
          { className: clsx("font-mono text-gray-600 dark:text-gray-400") },
          `client=${opts.conn.client}`,
        )
      : undefined,
    connectionBadge({ className: sourceBadgeClass }, opts.conn.source.type),
    opts.isCurrent
      ? connectionBadge(
          {
            className: clsx(
              "bg-zinc-100 border-zinc-400 dark:bg-zinc-700 dark:border-zinc-500",
            ),
          },
          "current",
        )
      : undefined,
  );
}

function connectionBadge(
  attrs: ElementProperties<HTMLSpanElement> = {},
  ...children: Children[]
): HTMLSpanElement {
  return h(
    "span",
    {
      ...attrs,
      className: clsx("px-1 border-2", attrs.className),
    },
    ...children,
  );
}
