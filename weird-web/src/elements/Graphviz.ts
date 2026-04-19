import z from "zod";
import {
  defineElement,
  h,
  type Children,
  type ElementProperties,
} from "./utils.ts";
import clsx from "clsx";
import * as Viz from "@viz-js/viz";
import Panzoom, { type PanzoomObject } from "@panzoom/panzoom";

const GraphvizGraphAttributes = z.record(
  z.string(),
  z
    .string()
    .or(z.number())
    .or(z.boolean())
    .or(z.object({ html: z.string() })),
);
type GraphvizGraphAttributes = z.output<typeof GraphvizGraphAttributes>;

const GraphvizGraphNode = z.object({
  name: z.string(),
  attributes: GraphvizGraphAttributes.optional(),
});
type GraphvizGraphNode = z.output<typeof GraphvizGraphNode>;

const GraphvizGraphEdge = z.object({
  tail: z.string(),
  head: z.string(),
  attributes: GraphvizGraphAttributes.optional(),
});
type GraphvizGraphEdge = z.output<typeof GraphvizGraphEdge>;

const GraphvizGraphSubgraph = z.object({
  name: z.string().optional(),
  graphAttributes: GraphvizGraphAttributes.optional(),
  nodeAttributes: GraphvizGraphAttributes.optional(),
  edgeAttributes: GraphvizGraphAttributes.optional(),
  nodes: GraphvizGraphNode.array().optional(),
  edges: GraphvizGraphEdge.array().optional(),
  get subgraphs() {
    return GraphvizGraphSubgraph.array().optional();
  },
});
type GraphvizGraphSubgraph = z.output<typeof GraphvizGraphSubgraph>;

const GraphvizGraph = z.object({
  name: z.string().optional(),
  strict: z.boolean().optional(),
  directed: z.boolean().optional(),
  graphAttributes: GraphvizGraphAttributes.optional(),
  nodeAttributes: GraphvizGraphAttributes.optional(),
  edgeAttributes: GraphvizGraphAttributes.optional(),
  nodes: GraphvizGraphNode.array().optional(),
  edges: GraphvizGraphEdge.array().optional(),
  subgraphs: GraphvizGraphSubgraph.array().optional(),
});
type GraphvizGraph = z.output<typeof GraphvizGraph>;

const GraphvizAttributes = z.object({
  graph: z.string().or(GraphvizGraph).optional(),
  engine: z.string().optional(),
  autoSize: z.boolean().optional(),
  pan: z.boolean().optional(),
  zoom: z.boolean().optional(),
});
type GraphvizAttributes = z.output<typeof GraphvizAttributes>;

export const Graphviz = defineElement(
  GraphvizAttributes,
  class {
    dom: HTMLDivElement;
    domSlot = null;
    #container: HTMLDivElement;
    #attrs: GraphvizAttributes = {};
    #renderedGraph: unknown;
    #renderedEngine: string | undefined;
    #inserted = false;
    #panzoom: PanzoomObject | undefined;

    constructor(attrs: GraphvizAttributes) {
      this.dom = h(
        "div",
        { className: clsx("overflow-hidden") },
        (this.#container = h("div", {
          className: clsx(
            "flex flex-row justify-around weird-graphviz max-w-full max-h-full",
          ),
        })),
      );
      this.dom.addEventListener("wheel", (event) => {
        if (event.shiftKey && this.#panzoom != null && this.#attrs.zoom) {
          this.#panzoom.zoomWithWheel(event);
        }
      });

      this.updateAttributes(attrs);
    }

    updateAttributes(attrs: GraphvizAttributes) {
      if (this.#inserted) {
        const pan = this.#attrs.pan ?? false;
        const zoom = this.#attrs.zoom ?? false;
        if (pan || zoom) {
          if (this.#panzoom == null) {
            this.#panzoom = Panzoom(this.#container, {
              canvas: true,
              disablePan: !pan,
              disableZoom: !zoom,
            });
          } else {
            this.#panzoom.setOptions({
              disablePan: !pan,
              disableZoom: !zoom,
            });
          }
        } else if (this.#panzoom != null) {
          this.#panzoom.reset();
          this.#panzoom.setOptions({
            disablePan: true,
            disableZoom: true,
          });
        }
      }

      this.#attrs = attrs;

      Viz.instance()
        .then((viz) => {
          const needsRerender =
            this.#attrs.engine !== this.#renderedGraph ||
            this.#attrs.graph !== this.#renderedEngine;
          if (!needsRerender) {
            return;
          }

          const renderOptions: Viz.RenderOptions = {
            graphAttributes: {
              bgcolor: "transparent",
              color: "transparent",
              fillcolor: "transparent",
            },
            nodeAttributes: {
              color: "transparent",
              fillcolor: "transparent",
            },
            edgeAttributes: {
              color: "transparent",
              fillcolor: "transparent",
              bgcolor: "transparent",
            },
          };
          if (this.#attrs.engine != null) {
            renderOptions.engine = this.#attrs.engine;
          }

          if (this.#attrs.graph != null) {
            const svg = viz.renderSVGElement(this.#attrs.graph, renderOptions);

            // Remove fixed SVG width/height when `autoSize` attribute is set,
            // so the element can resize to fit within its container
            const autoSize = this.#attrs.autoSize ?? false;
            if (autoSize) {
              svg.removeAttribute("width");
              svg.removeAttribute("height");
            }

            const defaultStroke = svg.querySelectorAll("[stroke=black]");
            for (const el of defaultStroke) {
              el.setAttribute("stroke", "var(--default-stroke)");
            }
            const defaultFill = svg.querySelectorAll("[fill=black]");
            for (const el of defaultFill) {
              el.setAttribute("fill", "var(--default-fill)");
            }
            const defaultFont = svg.querySelectorAll(
              '[font-family="Times,serif"]',
            );
            for (const el of defaultFont) {
              el.removeAttribute("font-family");
            }
            this.#container.replaceChildren(svg);
          }

          this.#renderedGraph = this.#attrs.graph;
          this.#renderedEngine = this.#attrs.engine;

          if (this.#panzoom != null) {
            this.#panzoom.reset();
          }
        })
        .catch((error) => {
          console.warn("Failed to render GraphViz graph", { error });
          this.#container.replaceChildren(
            h(
              "div",
              { className: clsx("bg-red-300 border-2 border-red-600 p-2") },
              h("strong", {}, "Failed to render GraphViz graph"),
              h("p", { className: clsx("font-mono") }, error.toString()),
            ),
          );
        });
    }

    didInsert() {
      this.#inserted = true;
      this.updateAttributes(this.#attrs);
    }
  },
);

export function buttonComponent(
  attrs: ElementProperties<HTMLButtonElement> = {},
  ...children: Children[]
): HTMLButtonElement {
  return h(
    "button",
    {
      ...attrs,
      className: clsx(
        "px-2 bg-white border-2 border-black shadow-sm hover:shadow-sm/50 hover:bg-zinc-200 focus-visible:shadow-sm/50 focus-visible:bg-zinc-200 active:bg-zinc-300 focus-visible:outline-2 focus-visible:outline-blue-400 dark:text-white dark:bg-zinc-800 dark:border-zinc-300 dark:hover:bg-zinc-700 dark:focus-visible:bg-zinc-700 dark:active:bg-zinc-600 dark:shadow-md",
        attrs.className,
      ),
    },
    ...children,
  );
}
