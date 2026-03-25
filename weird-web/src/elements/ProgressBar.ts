import z from "zod";
import { defineElement, h } from "./utils.ts";
import clsx from "clsx";

const ProgressBarAttributes = z.object({
  value: z.number().optional(),
  min: z.number().optional(),
  max: z.number().optional(),
});
type ProgressBarAttributes = z.output<typeof ProgressBarAttributes>;

export const ProgressBar = defineElement(
  ProgressBarAttributes,
  class {
    dom: HTMLDivElement;
    domSlot: HTMLDivElement;
    #progressBar: HTMLDivElement;

    constructor(attrs: ProgressBarAttributes) {
      this.domSlot = h("div", {
        className: clsx(
          "absolute",
          "inset-0",
          "text-white mix-blend-difference text-center",
        ),
      });
      this.#progressBar = h(
        "div",
        {
          className: clsx(
            "absolute",
            "inset-y-0 right-auto left-0",
            "bg-black w-1/4",
          ),
        },
        "\u00A0",
      );
      this.dom = h(
        "div",
        {
          className: clsx("relative border-2 border-black"),
        },
        "\u00A0",
        this.#progressBar,
        this.domSlot,
      );

      this.updateAttributes(attrs);
    }

    updateAttributes(attrs: ProgressBarAttributes) {
      const min = attrs.min ?? 0;
      const max = attrs.max ?? 1;
      const value = attrs.value ?? 0;

      const denom = max - min;
      const fraction = denom === 0 ? 0 : (value - min) / (max - min);
      const fractionClamped = fraction > 1 ? 1 : fraction < 0 ? 0 : fraction;

      this.#progressBar.style.width = `${fractionClamped * 100}%`;
    }
  },
);
