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

    #beforeChange: InputState | null = null;
    #afterChange: InputState | null = null;

    constructor(attrs: InputAttributes, ctx: WeirdElementContext) {
      this.dom = h("input", {
        className: clsx(
          "px-1 bg-white border-2 border-black shadow-sm focus-visible:shadow-sm/50 focus-visible:outline-2 focus-visible:outline-blue-400 dark:text-white dark:bg-zinc-800 dark:border-zinc-300 dark:shadow-md",
        ),
      });
      this.dom.addEventListener("beforeinput", () => {
        if (this.dom.selectionStart == null || this.dom.selectionEnd == null) {
          console.warn("expected selectionStart / selectionEnd to not be null");
          return;
        }
        this.#beforeChange = {
          start: this.dom.selectionStart,
          end: this.dom.selectionEnd,
          direction: this.dom.selectionDirection,
          value: this.dom.value,
        };
      });
      this.dom.addEventListener("input", (e) => {
        if (!(e instanceof InputEvent)) {
          console.warn("unexpected event type for input handler", e);
          return;
        }
        if (e.isComposing) {
          // Input is being composed (IME), handle with `compositionend` instead
          return;
        }

        this.#inputDidChange(ctx);
      });
      this.dom.addEventListener("compositionend", () => {
        this.#inputDidChange(ctx);
      });

      // Explicitly disable completions from 1Password extension:
      // https://developer.1password.com/docs/web/compatible-website-design/
      this.dom.dataset["1pIgnore"] = "";

      this.updateAttributes(attrs);
    }

    #inputDidChange(ctx: WeirdElementContext) {
      if (this.dom.selectionStart == null || this.dom.selectionEnd == null) {
        console.warn("expected selectionStart / selectionEnd to not be null");
        return;
      }
      this.#afterChange = {
        start: this.dom.selectionStart,
        end: this.dom.selectionEnd,
        direction: this.dom.selectionDirection,
        value: this.dom.value,
      };

      ctx.triggerEvent("change", { value: this.dom.value });

      if (this.#beforeChange != null) {
        this.dom.value = this.#beforeChange.value;
        this.dom.setSelectionRange(
          this.#beforeChange.start,
          this.#beforeChange.end,
          this.#beforeChange.direction ?? undefined,
        );
      }
    }

    updateAttributes(attrs: InputAttributes) {
      const newValue = attrs.value ?? "";
      if (newValue != this.dom.value) {
        this.dom.value = newValue;

        if (
          newValue === this.#afterChange?.value ||
          (newValue.length === this.#afterChange?.value.length &&
            newValue !== this.#beforeChange?.value)
        ) {
          this.dom.setSelectionRange(
            this.#afterChange.start,
            this.#afterChange.end,
            this.#afterChange.direction ?? undefined,
          );
        } else if (newValue.length === this.#beforeChange?.value.length) {
          this.dom.setSelectionRange(
            this.#beforeChange.start,
            this.#beforeChange.end,
            this.#beforeChange.direction ?? undefined,
          );
        }
      }
      this.dom.placeholder = attrs.placeholder ?? "";
    }
  },
);

interface InputState {
  start: number;
  end: number;
  direction: SelectionDirection | null;
  value: string;
}

type SelectionDirection = "forward" | "backward" | "none";
