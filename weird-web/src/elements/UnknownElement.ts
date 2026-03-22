import z from "zod";
import { h, defineElement } from "./utils.ts";

export const UnknownElement = defineElement(
  z.looseObject({}),
  class {
    dom: HTMLDivElement;
    domSlot: HTMLDivElement;
    constructor() {
      this.dom = this.domSlot = h("div", {
        className: "weird-container",
      });
    }
  },
);
