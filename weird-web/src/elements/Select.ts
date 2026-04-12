import z from "zod";
import {
  defineElement,
  h,
  type Children,
  type ElementProperties,
  type WeirdElementContext,
} from "./utils.ts";
import clsx from "clsx";
import unreachable from "ts-unreachable";

const SelectValue = z.string().or(
  z.object({
    value: z.string(),
    label: z.string().optional(),
  }),
);
type SelectValue = z.output<typeof SelectValue>;

const SelectAttributes = z.object({
  value: SelectValue.optional(),
  choices: SelectValue.array().optional(),
});
type SelectAttributes = z.output<typeof SelectAttributes>;

export const Select = defineElement(
  SelectAttributes,
  class {
    dom: HTMLSelectElement;
    domSlot = null;
    #value: SelectValue | undefined;
    #choices: SelectValue[] | undefined;

    constructor(attrs: SelectAttributes, ctx: WeirdElementContext) {
      this.#value = attrs.value;
      this.#choices = attrs.choices;

      this.dom = selectComponent({}, ...options(attrs.choices ?? []));

      this.dom.addEventListener("change", () => {
        let currentValue: string | undefined;
        if (this.#value != null) {
          if (typeof this.#value === "string") {
            currentValue = this.#value;
          } else if (
            typeof this.#value === "object" &&
            "value" in this.#value
          ) {
            currentValue = this.#value.value;
          } else {
            return unreachable(this.#value);
          }
        }
        const newChoice = this.#choices?.find((choice) => {
          if (typeof choice == "string") {
            return choice == this.dom.value;
          } else if (typeof choice === "object" && "value" in choice) {
            return choice.value === this.dom.value;
          } else {
            return unreachable(choice);
          }
        });
        ctx.triggerEvent("change", { value: newChoice });

        this.dom.value = currentValue ?? "";
      });
    }

    updateAttributes(attrs: SelectAttributes) {
      if (attrs.choices !== this.#choices) {
        this.#choices = attrs.choices;
        this.dom.replaceChildren(...options(this.#choices ?? []));
      }

      if (attrs.value !== this.#value) {
        this.#value = attrs.value;

        let newValue: string | undefined;
        if (attrs.value != null) {
          if (typeof attrs.value === "string") {
            newValue = attrs.value;
          } else if (
            typeof attrs.value === "object" &&
            "value" in attrs.value
          ) {
            newValue = attrs.value.value;
          } else {
            return unreachable(attrs.value);
          }
        }

        this.dom.value = newValue ?? "";
      }
    }
  },
);

export function selectComponent(
  attrs: ElementProperties<HTMLSelectElement> = {},
  ...children: Children[]
): HTMLSelectElement {
  return h(
    "select",
    {
      ...attrs,
      className: clsx(
        "px-2 bg-white border-2 border-black shadow-sm hover:shadow-sm/50 hover:bg-zinc-200 focus-visible:shadow-sm/50 focus-visible:bg-zinc-200 active:bg-zinc-300 focus-visible:outline-2 focus-visible:outline-blue-400 dark:text-white dark:bg-zinc-800 dark:border-zinc-300 dark:hover:bg-zinc-700 dark:focus-visible:bg-zinc-700 dark:active:bg-zinc-600 dark:shadow-md",
        attrs.className,
      ),
    },
    ...children,
  );
}

function options(choices: SelectValue[]): HTMLOptionElement[] {
  return choices.map((choice) => {
    if (typeof choice === "string") {
      return h("option", { value: choice }, choice);
    } else if (typeof choice === "object" && "value" in choice) {
      const label = choice.label ?? choice.value;
      return h("option", { value: choice.value }, label);
    } else {
      return unreachable(choice);
    }
  });
}
