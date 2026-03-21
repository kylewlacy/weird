import unreachable from "ts-unreachable";
import type { Merge, PickProperties, Prettify, Writable } from "ts-essentials";
import type * as z from "zod";

export interface WeirdElementClass {
  new (attributes: unknown): WeirdElement;
}

export interface WeirdElement {
  get dom(): Node;
  get domSlot(): Element | null;

  updateAttributes(attributes: unknown): void;
}

interface WeirdElementClassImpl<Attr extends object> {
  new (attributes: Attr): WeirdElementImpl<Attr>;
}

interface WeirdElementImpl<Attr extends object> {
  get dom(): Node;
  get domSlot(): Element | null;

  updateAttributes?(attributes: Attr, oldAttributes: Attr): void;
}

export function defineElement<ZAttr extends z.ZodObject>(
  attributeSchema: ZAttr,
  class_: WeirdElementClassImpl<z.output<ZAttr>>,
): WeirdElementClass {
  return class {
    #el: WeirdElementImpl<z.output<ZAttr>>;
    #currentAttributes: unknown;

    get dom(): Node {
      return this.#el.dom;
    }

    get domSlot(): Element | null {
      return this.#el.domSlot;
    }

    constructor(attributes: unknown) {
      const parsedAttributes = attributeSchema.parse(attributes);
      this.#currentAttributes = parsedAttributes;
      this.#el = new class_(parsedAttributes);
    }

    updateAttributes(attributes: unknown): void {
      const parsedAttributes = attributeSchema.parse(attributes);
      if (this.#el.updateAttributes) {
        this.#el.updateAttributes(
          parsedAttributes,
          this.#currentAttributes as z.output<ZAttr>,
        );
      }

      this.#currentAttributes = parsedAttributes;
    }
  };
}

type ElementProperties<E extends HTMLElement> = Prettify<
  Merge<
    Partial<
      PickProperties<
        Writable<Pick<E, ValidPropertyNames & keyof E>>,
        string | number | boolean | null | undefined | Node
      >
    >,
    {
      style?: Partial<CSSStyleDeclaration> | undefined;
    }
  >
>;

type ValidPropertyNames =
  | `a${string}`
  | `b${string}`
  | `c${string}`
  | `d${string}`
  | `e${string}`
  | `f${string}`
  | `g${string}`
  | `h${string}`
  | `i${string}`
  | `j${string}`
  | `k${string}`
  | `l${string}`
  | `m${string}`
  | `n${string}`
  | `o${string}`
  | `p${string}`
  | `q${string}`
  | `r${string}`
  | `s${string}`
  | `t${string}`
  | `u${string}`
  | `v${string}`
  | `w${string}`
  | `x${string}`
  | `y${string}`
  | `z${string}`;

type Children = undefined | null | string | Node | Children[];

export function h<Tag extends keyof HTMLElementTagNameMap>(
  tag: Tag,
  props?: ElementProperties<HTMLElementTagNameMap[Tag]>,
  ...children: Children[]
): HTMLElementTagNameMap[Tag] {
  const el = document.createElement(tag);
  for (const [key, value] of Object.entries(props ?? {})) {
    if (value == undefined) {
      continue;
    }

    if (key === "style") {
      if (typeof value !== "object") {
        throw new Error("invalid value for style property");
      }

      for (const [styleKey, styleValue] of Object.entries(value)) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        (el as any).style[styleKey] = styleValue;
      }
    } else {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (el as any)[key] = value;
    }
  }

  while (children.length > 0) {
    const child = children.shift();
    if (child == null) {
      continue;
    }

    if (Array.isArray(child)) {
      children.push(...child);
    } else if (child instanceof Node) {
      el.appendChild(child);
    } else if (typeof child === "string") {
      const childNode = document.createTextNode(child.toString());
      el.appendChild(childNode);
    } else {
      return unreachable(child);
    }
  }

  return el;
}

declare global {
  interface CSSStyleDeclaration {
    positionArea: string;
  }
}
