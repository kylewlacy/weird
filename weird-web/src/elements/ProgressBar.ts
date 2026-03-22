import z from "zod";
import { defineElement, h } from "./utils.ts";

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
        style: {
          position: "absolute",
          inset: "0",
          color: "#fff",
          mixBlendMode: "difference",
          textAlign: "center",
        },
      });
      this.#progressBar = h(
        "div",
        {
          style: {
            position: "absolute",
            inset: "0 auto 0 0",
            backgroundColor: "black",
            width: "25%",
          },
        },
        "\u00A0",
      );
      this.dom = h(
        "div",
        {
          style: {
            position: "relative",
            lineHeight: "1.5",
          },
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
