import z from "zod";
import { defineElement, h, type WeirdElementContext } from "./utils.ts";
import clsx from "clsx";

const ButtonAttributes = z.object({});
type ButtonAttributes = z.output<typeof ButtonAttributes>;

export const Button = defineElement(
  ButtonAttributes,
  class {
    dom: HTMLButtonElement;
    domSlot: HTMLButtonElement;

    constructor(_attrs: ButtonAttributes, ctx: WeirdElementContext) {
      const button = h("button", {
        className: clsx(
          "bg-white border-2 border-black shadow-sm hover:shadow-sm/50 hover:bg-zinc-200 focus:shadow-sm/50 focus:bg-zinc-200 focus-visible:outline-2 focus-visible:outline-blue-400 dark:bg-zinc-800 dark:border-zinc-300 dark:hover:bg-zinc-700 dark:focus:bg-zinc-700 dark:shadow-md",
        ),
      });

      button.addEventListener("click", () => {
        ctx.triggerEvent("click", {});
      });

      this.dom = this.domSlot = button;
    }
  },
);
