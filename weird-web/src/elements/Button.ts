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
          "bg-white border-2 border-current shadow-sm hover:shadow-sm/50 hover:bg-gray-200 focus:shadow-sm/50 focus:bg-gray-200 focus-visible:outline-2 focus-visible:outline-blue-400",
        ),
      });

      button.addEventListener("click", () => {
        ctx.triggerEvent("click", {});
      });

      this.dom = this.domSlot = button;
    }
  },
);
