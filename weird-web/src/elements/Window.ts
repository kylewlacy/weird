import { defineElement, h } from "./utils.ts";

interface WindowMoveState {
  offsetX: number;
  offsetY: number;
}

export const Window = defineElement(
  "weird-window",
  class extends HTMLElement {
    connectedCallback() {
      const shadow = this.attachShadow({ mode: "open" });

      const windowTitlebar = h(
        "div",
        { style: { borderBottom: "1px solid black" } },
        "Window",
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
        h("div", {}, h("slot")),
      );

      let pointerMoveState: WindowMoveState | undefined;

      const onPointerMove = (event: PointerEvent) => {
        if (pointerMoveState == null) {
          return;
        }
        const parentRect = windowEl.offsetParent?.getBoundingClientRect();
        const windowLeft =
          event.clientX - (parentRect?.left ?? 0) - pointerMoveState.offsetX;
        const windowTop =
          event.clientY - (parentRect?.top ?? 0) - pointerMoveState.offsetY;

        windowEl.style.transform = `translateX(${windowLeft}px) translateY(${windowTop}px)`;
      };

      windowTitlebar.addEventListener("pointerdown", (event) => {
        windowTitlebar.setPointerCapture(event.pointerId);
      });
      windowTitlebar.addEventListener("gotpointercapture", (event) => {
        const windowRect = windowEl.getBoundingClientRect();
        pointerMoveState = {
          offsetX: event.clientX - windowRect.left,
          offsetY: event.clientY - windowRect.top,
        };
        windowTitlebar.addEventListener("pointermove", onPointerMove);
      });
      windowTitlebar.addEventListener("lostpointercapture", () => {
        pointerMoveState = undefined;
        windowTitlebar.removeEventListener("pointermove", onPointerMove);
      });

      shadow.appendChild(windowEl);
    }
  },
);
