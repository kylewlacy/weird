import z from "zod";
import { defineElement, h } from "./utils.ts";

let topZIndex: number = 2;

interface WindowMoveState {
  offsetX: number;
  offsetY: number;
}

const WindowAttributes = z.object({
  title: z.string().optional(),
  unpadded: z.boolean().optional(),
});
type WindowAttributes = z.output<typeof WindowAttributes>;

const DEFAULT_WINDOW_TITLE = "Untitled Window" as const;

export const Window = defineElement(
  WindowAttributes,
  class {
    dom: HTMLDivElement;
    domSlot: HTMLDivElement;
    #titleNode: Text;

    constructor(attrs: WindowAttributes) {
      const zIndex = topZIndex++;

      this.domSlot = h("div", {
        className: "weird-container",
      });
      this.#titleNode = document.createTextNode("");
      const windowTitlebar = h(
        "div",
        {
          style: {
            borderBottom: "2px solid black",
            touchAction: "none",
            userSelect: "none",
            padding: "0.25rem",
            textWrap: "nowrap",
          },
        },
        this.#titleNode,
      );
      const windowEl = h(
        "div",
        {
          style: {
            border: "2px solid black",
            position: "absolute",
            backgroundColor: "white",
            zIndex: zIndex.toString(),
            boxShadow: "0.25rem 0.25rem rgba(0,0,0,0.5)",
          },
        },
        windowTitlebar,
        h("div", {}, this.domSlot),
      );
      this.dom = windowEl;

      let pointerMoveState: WindowMoveState | undefined;

      const onPointerMove = (event: PointerEvent) => {
        if (pointerMoveState == null) {
          return;
        }
        const windowLeft =
          event.clientX - windowEl.offsetLeft - pointerMoveState.offsetX;
        const windowTop =
          event.clientY - windowEl.offsetTop - pointerMoveState.offsetY;

        windowEl.style.transform = `translateX(${windowLeft}px) translateY(${windowTop}px)`;
      };
      windowTitlebar.addEventListener("pointerdown", (event) => {
        const windowRect = windowEl.getBoundingClientRect();
        pointerMoveState = {
          offsetX: event.clientX - windowRect.left,
          offsetY: event.clientY - windowRect.top,
        };

        windowTitlebar.setPointerCapture(event.pointerId);

        const currentZIndex = Number(windowEl.style.zIndex);
        if (Number.isNaN(currentZIndex) || currentZIndex <= topZIndex) {
          if (currentZIndex < topZIndex) {
            topZIndex++;
          }
          windowEl.style.zIndex = topZIndex.toString();
        }
      });
      windowTitlebar.addEventListener("pointerup", (event) => {
        windowTitlebar.releasePointerCapture(event.pointerId);
      });
      windowTitlebar.addEventListener("gotpointercapture", () => {
        windowTitlebar.addEventListener("pointermove", onPointerMove);
      });
      windowTitlebar.addEventListener("lostpointercapture", () => {
        pointerMoveState = undefined;
        windowTitlebar.removeEventListener("pointermove", onPointerMove);
      });

      this.updateAttributes(attrs);
    }

    updateAttributes(attrs: WindowAttributes) {
      this.#titleNode.textContent = attrs.title ?? DEFAULT_WINDOW_TITLE;
      this.domSlot.style.padding = (attrs.unpadded ?? false) ? "0" : "0.25rem";
    }
  },
);
