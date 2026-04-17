import z from "zod";
import { defineElement, h, type WeirdElementContext } from "./utils.ts";
import clsx from "clsx";

const CheckboxAttributes = z.object({
  value: z.boolean().optional(),
});
type CheckboxAttributes = z.output<typeof CheckboxAttributes>;

export const Checkbox = defineElement(
  CheckboxAttributes,
  class {
    dom: HTMLInputElement;
    domSlot = null;

    #value: boolean;

    constructor(attrs: CheckboxAttributes, ctx: WeirdElementContext) {
      this.dom = h("input", {
        type: "checkbox",
        className: clsx(
          "px-1 bg-white border-2 border-black shadow-sm focus-visible:shadow-sm/50 focus-visible:outline-2 focus-visible:outline-blue-400 dark:text-white dark:bg-zinc-800 dark:border-zinc-300 dark:shadow-md",
        ),
      });
      this.dom.addEventListener("input", () => {
        this.#inputDidChange(ctx);
      });

      this.#value = attrs.value ?? false;
      this.updateAttributes(attrs);
    }

    #inputDidChange(ctx: WeirdElementContext) {
      ctx.triggerEvent("change", { value: this.dom.checked });
      this.dom.checked = this.#value;
    }

    updateAttributes(attrs: CheckboxAttributes) {
      const value = attrs.value ?? false;
      this.dom.checked = value;
      this.#value = value;
    }
  },
);
