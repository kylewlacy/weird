import z from "zod";
import { h, defineElement } from "./utils.ts";
import clsx from "clsx";

export const World = defineElement(
  z.object(),
  class {
    dom: HTMLDivElement;
    domSlot: HTMLDivElement;
    constructor() {
      this.dom = this.domSlot = h("div", {
        className: clsx("relative size-full z-0"),
      });
    }
  },
);
