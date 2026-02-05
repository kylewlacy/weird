import { defineElement, h } from "./utils.ts";

interface FrameMoveState {
  offsetX: number;
  offsetY: number;
}

export const Frame = defineElement(
  "weird-frame",
  class extends HTMLElement {
    connectedCallback() {
      const shadow = this.attachShadow({ mode: "open" });

      const frameTitlebar = h(
        "div",
        { style: { borderBottom: "1px solid black" } },
        "Frame",
      );
      const frame = h(
        "div",
        {
          style: {
            border: "1px solid black",
            position: "absolute",
            backgroundColor: "white",
          },
        },
        frameTitlebar,
        h("div", {}, h("slot")),
      );

      let pointerMoveState: FrameMoveState | undefined;

      const onPointerMove = (event: PointerEvent) => {
        if (pointerMoveState == null) {
          return;
        }
        const parentRect = frame.offsetParent?.getBoundingClientRect();
        const frameLeft =
          event.clientX - (parentRect?.left ?? 0) - pointerMoveState.offsetX;
        const frameTop =
          event.clientY - (parentRect?.top ?? 0) - pointerMoveState.offsetY;

        frame.style.transform = `translateX(${frameLeft}px) translateY(${frameTop}px)`;
      };

      frameTitlebar.addEventListener("pointerdown", (event) => {
        frameTitlebar.setPointerCapture(event.pointerId);
      });
      frameTitlebar.addEventListener("gotpointercapture", (event) => {
        const frameRect = frame.getBoundingClientRect();
        pointerMoveState = {
          offsetX: event.clientX - frameRect.left,
          offsetY: event.clientY - frameRect.top,
        };
        frameTitlebar.addEventListener("pointermove", onPointerMove);
      });
      frameTitlebar.addEventListener("lostpointercapture", () => {
        pointerMoveState = undefined;
        frameTitlebar.removeEventListener("pointermove", onPointerMove);
      });

      shadow.appendChild(frame);
    }
  },
);
