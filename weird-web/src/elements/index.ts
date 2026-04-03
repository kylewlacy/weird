import type { WeirdElement, WeirdElementClass } from "./utils.ts";
import { UnknownElement } from "./UnknownElement.ts";
import { World } from "./World.ts";
import { Window } from "./Window.ts";
import { ProgressBar } from "./ProgressBar.ts";
import { Button } from "./Button.ts";
import { Input } from "./Input.ts";

export type { WeirdElement, WeirdElementClass };

export const ELEMENTS = {
  Button,
  Input,
  ProgressBar,
  UnknownElement,
  Window,
  World,
} as const satisfies Record<string, WeirdElementClass>;
