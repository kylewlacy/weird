import z from "zod";
import { h, defineElement } from "./utils.ts";

export const World = defineElement(
  z.object(),
  class {
    dom: HTMLDivElement;
    domSlot: HTMLDivElement;
    constructor() {
      this.dom = this.domSlot = h("div", {
        style: { position: "relative" },
      });
    }
  },
);
