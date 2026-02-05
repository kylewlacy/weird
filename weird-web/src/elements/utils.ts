import unreachable from "ts-unreachable";
import type { Merge, PickProperties, Prettify, Writable } from "ts-essentials";

interface DefinedElement {
  name: string;
}

export function defineElement(
  name: string,
  constructor: CustomElementConstructor,
): DefinedElement {
  customElements.define(name, constructor);
  return { name };
}

interface DefineElementFromTemplateOptions {
  beforeAppend?: (options: { content: DocumentFragment }) => void;
}

export function defineElementFromTemplate(
  name: string,
  options: DefineElementFromTemplateOptions = {},
): DefinedElement {
  return defineElement(
    name,
    class extends HTMLElement {
      constructor() {
        super();
        const template = document.getElementById(`template-${name}`);
        if (!(template instanceof HTMLTemplateElement)) {
          throw new Error(`template not found for custom element ${name}`);
        }

        const content = document.importNode(template.content, true);

        options.beforeAppend?.({ content });

        const shadow = this.attachShadow({ mode: "open" });
        shadow.appendChild(content);
      }
    },
  );
}

type ElementProperties<E extends HTMLElement> = Prettify<
  Merge<
    Partial<
      PickProperties<
        Writable<Pick<E, ValidPropertyNames & keyof E>>,
        string | number | boolean | null | undefined
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
