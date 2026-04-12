import z from "zod";
import {
  defineElement,
  h,
  type Children,
  type ElementProperties,
  type WeirdElementContext,
} from "./utils.ts";
import clsx from "clsx";

const ButtonAttributes = z.object({});
type ButtonAttributes = z.output<typeof ButtonAttributes>;

export const Button = defineElement(
  ButtonAttributes,
  class {
    dom: HTMLButtonElement;
    domSlot: HTMLButtonElement;

    constructor(_attrs: ButtonAttributes, ctx: WeirdElementContext) {
      const button = buttonComponent();

      button.addEventListener("click", () => {
        ctx.triggerEvent("click", {});
      });

      this.dom = this.domSlot = button;
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
        "px-2 bg-white border-2 border-black shadow-sm hover:shadow-sm/50 hover:bg-zinc-200 focus-visible:shadow-sm/50 focus-visible:bg-zinc-200 aria-selected:bg-zinc-300 active:bg-zinc-300 focus-visible:outline-2 focus-visible:outline-blue-400 dark:text-white dark:bg-zinc-800 dark:border-zinc-300 dark:hover:bg-zinc-700 dark:focus-visible:bg-zinc-700 dark:aria-selected:bg-zinc-600 dark:active:bg-zinc-600 dark:shadow-md",
        attrs.className,
      ),
    },
    ...children,
  );
}
