import z from "zod";
import { h, defineElement } from "./utils.ts";
import clsx from "clsx";

export const Col = defineElement(
  z.looseObject({}),
  class {
    dom: HTMLDivElement;
    domSlot: HTMLDivElement;
    constructor() {
      this.dom = this.domSlot = h("div", {
        className: clsx("flex flex-col gap-1"),
      });
    }
  },
);
