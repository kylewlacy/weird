import z from "zod";
import {
  defineElement,
  h,
  type Children,
  type ElementProperties,
  type WeirdElementContext,
} from "./utils.ts";
import clsx from "clsx";

let topZIndex: number = 2;

type PointerCaptureState = {
  offsetX: number;
  offsetY: number;
  captured: HTMLElement;
} & (
  | { type: "move" }
  | {
      type: "resize";
      direction: ResizeDirection;
    }
);

const WindowAttributes = z.object({
  title: z.string().optional(),
  unpadded: z.boolean().optional(),
  stale: z.boolean().optional(),
});
type WindowAttributes = z.output<typeof WindowAttributes>;

const DEFAULT_WINDOW_TITLE = "(untitled)" as const;

const RESIZE_DIRECTIONS = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"] as const;
type ResizeDirection = (typeof RESIZE_DIRECTIONS)[number];

export const Window = defineElement(
  WindowAttributes,
  class {
    dom: HTMLDivElement;
    domSlot: HTMLDivElement;
    #maximizeButton: HTMLButtonElement;
    #maximizeButtonTextNode: Text;
    #closeButton: HTMLButtonElement;
    #isMaximized: boolean = false;
    #titleNode: Text;
    #staleOverlay: HTMLDivElement;
    #left: number = 0;
    #top: number = 0;
    #resizeHandles: Record<ResizeDirection, HTMLDivElement>;

    #windowResizeListener = (): void => {
      this.moveWindowTo({ left: this.#left, top: this.#top });
    };

    constructor(attrs: WindowAttributes, ctx: WeirdElementContext) {
      const zIndex = topZIndex++;

      let windowTitlebar: HTMLDivElement;
      const resizeHandles: Partial<Record<ResizeDirection, HTMLDivElement>> =
        {};
      this.dom = h(
        "div",
        {
          className: clsx(
            "flex flex-col border-2 absolute shadow-md text-black bg-white border-black dark:text-white dark:border-zinc-300 dark:bg-zinc-800 dark:shadow-lg max-w-full max-h-full transition-window",
          ),
          style: {
            zIndex: zIndex.toString(),
          },
        },
        (windowTitlebar = h(
          "div",
          {
            className: clsx(
              "flex-none border-b-2 border-black touch-none select-none dark:border-zinc-300",
            ),
          },
          h(
            "div",
            {
              className: clsx("flex-1 flex items-center"),
            },
            h(
              "div",
              { className: clsx("flex-1 px-1 text-nowrap") },
              (this.#titleNode = document.createTextNode("")),
            ),
            (this.#maximizeButton = titlebarButtonComponent(
              {},
              (this.#maximizeButtonTextNode = document.createTextNode("^")),
            )),
            (this.#closeButton = titlebarButtonComponent({}, "X")),
          ),
        )),
        h(
          "div",
          { className: clsx("relative w-full h-full overflow-auto") },
          (this.#staleOverlay = h("div", {
            className: clsx("absolute inset-0 bg-black dark:bg-white"),
          })),
          (this.domSlot = h("div", {
            className: clsx("w-full h-full"),
          })),
        ),
        (resizeHandles.N = h("div", {
          className: clsx("w-full -my-4 h-4 absolute top-0 cursor-n-resize"),
        })),
        (resizeHandles.NE = h("div", {
          className: clsx(
            "size-4 -m-4 absolute top-0 right-0 cursor-ne-resize",
          ),
        })),
        (resizeHandles.E = h("div", {
          className: clsx(
            "h-full w-4 -mx-4 absolute top-0 right-0 cursor-e-resize",
          ),
        })),
        (resizeHandles.SE = h("div", {
          className: clsx(
            "size-4 -m-4 absolute bottom-0 right-0 cursor-se-resize",
          ),
        })),
        (resizeHandles.S = h("div", {
          className: clsx("w-full -my-4 h-4 absolute bottom-0 cursor-s-resize"),
        })),
        (resizeHandles.SW = h("div", {
          className: clsx(
            "size-4 -m-4 absolute bottom-0 left-0 cursor-sw-resize",
          ),
        })),
        (resizeHandles.W = h("div", {
          className: clsx(
            "h-full w-4 -mx-4 absolute top-0 left-0 cursor-w-resize",
          ),
        })),
        (resizeHandles.NW = h("div", {
          className: clsx("size-4 -m-4 absolute top-0 left-0 cursor-nw-resize"),
        })),
      );
      this.#resizeHandles = finishResizeHandles(resizeHandles);

      let pointerCaptureState: PointerCaptureState | undefined;

      const onPointerMove = (event: PointerEvent) => {
        if (event.button > 0) {
          windowTitlebar.releasePointerCapture(event.pointerId);
          return;
        }

        if (pointerCaptureState == null) {
          return;
        }

        switch (pointerCaptureState.type) {
          case "move": {
            const windowLeft =
              event.clientX - this.dom.offsetLeft - pointerCaptureState.offsetX;
            const windowTop =
              event.clientY - this.dom.offsetTop - pointerCaptureState.offsetY;
            this.moveWindowTo({ left: windowLeft, top: windowTop });
            break;
          }
          case "resize": {
            const windowRect = this.dom.getBoundingClientRect();
            let windowX: number | undefined;
            let windowY: number | undefined;
            let windowWidth: number | undefined;
            let windowHeight: number | undefined;

            switch (pointerCaptureState.direction) {
              case "N":
              case "NE":
              case "NW":
                windowHeight = windowRect.bottom - event.clientY;
                windowY = event.clientY;
                break;
              case "S":
              case "SE":
              case "SW":
                windowHeight = event.clientY - windowRect.top;
                break;
              case "E":
              case "W":
                break;
            }
            switch (pointerCaptureState.direction) {
              case "E":
              case "NE":
              case "SE":
                windowWidth = event.clientX - windowRect.left;
                break;
              case "W":
              case "NW":
              case "SW":
                windowWidth = windowRect.right - Math.round(event.clientX);
                windowX = Math.round(event.clientX);
                break;
              case "N":
              case "S":
                break;
            }

            if (windowWidth != null) {
              this.dom.style.width = `${windowWidth}px`;
            }
            if (windowHeight != null) {
              this.dom.style.height = `${windowHeight}px`;
            }
            if (windowX != null || windowY != null) {
              this.moveWindowTo({
                left: windowX ?? this.#left,
                top: windowY ?? this.#top,
              });
            }
            break;
          }
        }
      };

      windowTitlebar.addEventListener("pointerdown", (event) => {
        if (event.button !== 0) {
          return;
        }

        this.dom.classList.add("weird-window-dragging");

        const windowRect = this.dom.getBoundingClientRect();
        const parentRect = this.dom.parentElement?.getBoundingClientRect();
        windowTitlebar.setPointerCapture(event.pointerId);
        pointerCaptureState = {
          type: "move",
          offsetX: event.clientX - windowRect.left + (parentRect?.left ?? 0),
          offsetY: event.clientY - windowRect.top + (parentRect?.top ?? 0),
          captured: windowTitlebar,
        };

        const currentZIndex = Number(this.dom.style.zIndex);
        if (Number.isNaN(currentZIndex) || currentZIndex <= topZIndex) {
          if (currentZIndex < topZIndex) {
            topZIndex++;
          }
          this.dom.style.zIndex = topZIndex.toString();
        }

        event.preventDefault();
      });
      windowTitlebar.addEventListener("pointerup", (event) => {
        windowTitlebar.releasePointerCapture(event.pointerId);
      });
      windowTitlebar.addEventListener("gotpointercapture", () => {
        windowTitlebar.addEventListener("pointermove", onPointerMove);
      });
      windowTitlebar.addEventListener("lostpointercapture", () => {
        this.dom.classList.remove("weird-window-dragging");
        pointerCaptureState = undefined;
        windowTitlebar.removeEventListener("pointermove", onPointerMove);
      });

      for (const direction of RESIZE_DIRECTIONS) {
        const resizeHandle = this.#resizeHandles[direction];
        resizeHandle.addEventListener("pointerdown", (event: PointerEvent) => {
          if (event.button !== 0) {
            return;
          }

          this.dom.classList.add("weird-window-resizing");

          const windowRect = this.dom.getBoundingClientRect();
          const parentRect = this.dom.parentElement?.getBoundingClientRect();
          resizeHandle.setPointerCapture(event.pointerId);
          pointerCaptureState = {
            type: "resize",
            direction,
            offsetX: event.clientX - windowRect.left + (parentRect?.left ?? 0),
            offsetY: event.clientY - windowRect.top + (parentRect?.top ?? 0),
            captured: resizeHandle,
          };
        });
        resizeHandle.addEventListener("pointerup", (event) => {
          resizeHandle.releasePointerCapture(event.pointerId);
        });
        resizeHandle.addEventListener("gotpointercapture", () => {
          resizeHandle.addEventListener("pointermove", onPointerMove);
        });
        resizeHandle.addEventListener("lostpointercapture", () => {
          this.dom.classList.remove("weird-window-resizing");
          pointerCaptureState = undefined;
          resizeHandle.removeEventListener("pointermove", onPointerMove);
        });
      }

      this.#maximizeButton.addEventListener("pointerdown", (event) => {
        event.stopPropagation();
      });
      this.#maximizeButton.addEventListener("click", (event) => {
        event.preventDefault();
        this.#toggleMaximized();
      });

      this.#closeButton.addEventListener("pointerdown", (event) => {
        event.stopPropagation();
      });
      this.#closeButton.addEventListener("click", (event) => {
        event.preventDefault();
        ctx.triggerEvent("close", {});
      });

      window.addEventListener("resize", this.#windowResizeListener);
      this.moveWindowTo({ left: 0, top: 0 });
      this.updateAttributes(attrs);
    }

    didRemove() {
      window.removeEventListener("resize", this.#windowResizeListener);
    }

    moveWindowTo(pos: { left: number; top: number }) {
      const windowRect = this.dom.getBoundingClientRect();
      const parentRect = (
        this.dom.parentElement ?? document.body
      ).getBoundingClientRect();

      const leftMin = Math.min(0, 80 - windowRect.width);
      const leftMax = Math.max(0, parentRect.width - 40);
      const topMin = -10;
      const topMax = Math.max(0, parentRect.height - 20);
      const left = Math.min(leftMax, Math.max(leftMin, pos.left));
      const top = Math.min(topMax, Math.max(topMin, pos.top));

      this.#left = left;
      this.#top = top;

      this.dom.style.transform = `translateX(${left}px) translateY(${top}px)`;
    }

    #setMaximized(maximize: boolean) {
      this.#isMaximized = maximize;
      this.#maximizeButtonTextNode.textContent = maximize ? "v" : "^";
      if (maximize) {
        this.dom.style.width = "100%";
        this.dom.style.height = "100%";
        this.dom.style.transform = `translateX(0px) translateY(0px)`;
      } else {
        this.dom.style.width = "auto";
        this.dom.style.height = "auto";
        this.moveWindowTo({ left: this.#left, top: this.#top });
      }
    }

    #toggleMaximized() {
      this.#setMaximized(!this.#isMaximized);
    }

    updateAttributes(attrs: WindowAttributes) {
      const unpadded = attrs.unpadded ?? false;
      const stale = attrs.stale ?? false;
      this.#titleNode.textContent = attrs.title ?? DEFAULT_WINDOW_TITLE;
      this.domSlot.className = clsx(
        "flex flex-col gap-1 w-full h-full",
        unpadded ? "p-0" : "p-1",
      );
      this.#staleOverlay.classList.add(clsx("transition-opacity"));
      this.#staleOverlay.style.opacity = stale ? "40%" : "0";
      this.#staleOverlay.style.pointerEvents = stale ? "" : "none";
    }
  },
);

function titlebarButtonComponent(
  attrs: ElementProperties<HTMLButtonElement> = {},
  ...children: Children[]
): HTMLButtonElement {
  return h(
    "button",
    {
      ...attrs,
      className: clsx(
        "text-xs font-semibold w-6 h-6 m-0.5 bg-white border-2 border-black shadow-xs hover:shadow-xs/25 hover:bg-zinc-200 focus-visible:bg-zinc-200 active:bg-zinc-300 focus-visible:outline-2 focus-visible:outline-blue-400 dark:text-white dark:bg-zinc-800 dark:border-zinc-300 dark:hover:bg-zinc-700 dark:focus-visible:bg-zinc-700 dark:active:bg-zinc-600",
        attrs.className,
      ),
    },
    ...children,
  );
}

function finishResizeHandles(
  resizeHandles: Partial<Record<ResizeDirection, HTMLDivElement>>,
): Record<ResizeDirection, HTMLDivElement> {
  if (
    resizeHandles.N != null &&
    resizeHandles.NE != null &&
    resizeHandles.E != null &&
    resizeHandles.SE != null &&
    resizeHandles.S != null &&
    resizeHandles.SW != null &&
    resizeHandles.W != null &&
    resizeHandles.NW != null
  ) {
    return {
      N: resizeHandles.N,
      NE: resizeHandles.NE,
      E: resizeHandles.E,
      SE: resizeHandles.SE,
      S: resizeHandles.S,
      SW: resizeHandles.SW,
      W: resizeHandles.W,
      NW: resizeHandles.NW,
    };
  } else {
    throw new Error("failed to initialize resizeHandles");
  }
}
