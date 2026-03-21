import z from "zod";
import { defineElement, h } from "./utils.ts";

interface WindowMoveState {
  offsetX: number;
  offsetY: number;
}

const WindowAttributes = z.object({
  title: z.string().optional(),
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
      this.domSlot = h("div");

      this.#titleNode = document.createTextNode(
        attrs.title ?? DEFAULT_WINDOW_TITLE,
      );
      const windowTitlebar = h(
        "div",
        {
          style: {
            borderBottom: "1px solid black",
            touchAction: "none",
            userSelect: "none",
          },
        },
        this.#titleNode,
      );
      const windowEl = h(
        "div",
        {
          style: {
            border: "1px solid black",
            position: "absolute",
            backgroundColor: "white",
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
    }

    changeAttribute<K extends keyof WindowAttributes>(
      key: K,
      value: WindowAttributes[K] | undefined,
    ) {
      switch (key) {
        case "title": {
          this.#titleNode.textContent = value ?? DEFAULT_WINDOW_TITLE;
        }
      }
    }
  },
);
