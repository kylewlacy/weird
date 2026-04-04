import z from "zod";
import { defineElement, h, type WeirdElementContext } from "./utils.ts";
import clsx from "clsx";

const InputAttributes = z.object({
  value: z.string().optional(),
  placeholder: z.string().optional(),
});
type InputAttributes = z.output<typeof InputAttributes>;

export const Input = defineElement(
  InputAttributes,
  class {
    dom: HTMLInputElement;
    domSlot = null;

    constructor(attrs: InputAttributes, ctx: WeirdElementContext) {
      this.dom = h("input", {
        className: clsx(
          "px-1 bg-white border-2 border-black shadow-sm focus-visible:shadow-sm/50 focus-visible:outline-2 focus-visible:outline-blue-400 dark:text-white dark:bg-zinc-800 dark:border-zinc-300 dark:shadow-md",
        ),
      });
      this.dom.addEventListener("beforeinput", (e) => {
        const prefix = this.dom.value.substring(
          0,
          this.dom.selectionStart ?? undefined,
        );
        const suffix =
          this.dom.selectionEnd != null
            ? this.dom.value.substring(this.dom.selectionEnd)
            : "";
        const newValue = prefix + (e.data ?? "") + suffix;
        ctx.triggerEvent("change", { value: newValue });
        e.preventDefault();
      });

      this.updateAttributes(attrs);
    }

    updateAttributes(attrs: InputAttributes) {
      this.dom.value = attrs.value ?? "";
      this.dom.placeholder = attrs.placeholder ?? "";
    }
  },
);
