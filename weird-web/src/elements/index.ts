import type { WeirdElement, WeirdElementClass } from "./utils.ts";
import { UnknownElement } from "./UnknownElement.ts";
import { World } from "./World.ts";
import { Window } from "./Window.ts";

export type { WeirdElement, WeirdElementClass };

export const ELEMENTS = {
  UnknownElement,
  Window,
  World,
} as const satisfies Record<string, WeirdElementClass>;
