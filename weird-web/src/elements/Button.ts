import z from "zod";
import { defineElement, h, type WeirdElementContext } from "./utils.ts";

const ButtonAttributes = z.object({});
type ButtonAttributes = z.output<typeof ButtonAttributes>;

export const Button = defineElement(
  ButtonAttributes,
  class {
    dom: HTMLButtonElement;
    domSlot: HTMLButtonElement;

    constructor(attrs: ButtonAttributes, ctx: WeirdElementContext) {
      const button = h("button", {});

      button.addEventListener("click", () => {
        ctx.triggerEvent("click", {});
      });

      this.dom = this.domSlot = button;
    }
  },
);
