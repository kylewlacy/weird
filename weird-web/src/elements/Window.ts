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

interface WindowMoveState {
  offsetX: number;
  offsetY: number;
}

const WindowAttributes = z.object({
  title: z.string().optional(),
  unpadded: z.boolean().optional(),
  stale: z.boolean().optional(),
});
type WindowAttributes = z.output<typeof WindowAttributes>;

const DEFAULT_WINDOW_TITLE = "(untitled)" as const;

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

    #windowResizeListener = (): void => {
      this.moveWindowTo({ left: this.#left, top: this.#top });
    };

    constructor(attrs: WindowAttributes, ctx: WeirdElementContext) {
      const zIndex = topZIndex++;

      let windowTitlebar: HTMLDivElement;
      this.dom = h(
        "div",
        {
          className: clsx(
            "border-2 absolute shadow-md text-black bg-white border-black dark:text-white dark:border-zinc-300 dark:bg-zinc-800 dark:shadow-lg transition-window",
          ),
          style: {
            zIndex: zIndex.toString(),
          },
        },
        (windowTitlebar = h(
          "div",
          {
            className: clsx(
              "border-b-2 border-black touch-none select-none dark:border-zinc-300",
            ),
          },
          h(
            "div",
            {
              className: clsx("flex items-center"),
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
          { className: clsx("relative") },
          (this.#staleOverlay = h("div", {
            className: clsx("absolute inset-0 bg-black dark:bg-white"),
          })),
          (this.domSlot = h("div", {})),
        ),
      );

      let pointerMoveState: WindowMoveState | undefined;

      const onPointerMove = (event: PointerEvent) => {
        if (event.button > 0) {
          windowTitlebar.releasePointerCapture(event.pointerId);
          return;
        }

        if (pointerMoveState == null) {
          return;
        }
        const windowLeft =
          event.clientX - this.dom.offsetLeft - pointerMoveState.offsetX;
        const windowTop =
          event.clientY - this.dom.offsetTop - pointerMoveState.offsetY;
        this.moveWindowTo({ left: windowLeft, top: windowTop });
      };
      windowTitlebar.addEventListener("pointerdown", (event) => {
        if (event.button !== 0) {
          return;
        }

        this.dom.classList.add("weird-window-dragging");

        const windowRect = this.dom.getBoundingClientRect();
        const parentRect = this.dom.parentElement?.getBoundingClientRect();
        pointerMoveState = {
          offsetX: event.clientX - windowRect.left + (parentRect?.left ?? 0),
          offsetY: event.clientY - windowRect.top + (parentRect?.top ?? 0),
        };

        windowTitlebar.setPointerCapture(event.pointerId);

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
        pointerMoveState = undefined;
        windowTitlebar.removeEventListener("pointermove", onPointerMove);
      });

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
        "flex flex-col gap-1",
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
