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
        {
          style: {
            borderBottom: "1px solid black",
            touchAction: "none",
            userSelect: "none",
          },
        },
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

      shadow.appendChild(windowEl);
    }
  },
);
