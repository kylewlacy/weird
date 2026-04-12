import type { WeirdElement, WeirdElementClass } from "./utils.ts";
import { UnknownElement } from "./UnknownElement.ts";
import { World } from "./World.ts";
import { Window } from "./Window.ts";
import { ProgressBar } from "./ProgressBar.ts";
import { Button } from "./Button.ts";
import { Input } from "./Input.ts";
import { Graphviz } from "./Graphviz.ts";
import { Select } from "./Select.ts";

export type { WeirdElement, WeirdElementClass };

export const ELEMENTS = {
  Button,
  Graphviz,
  Input,
  ProgressBar,
  Select,
  UnknownElement,
  Window,
  World,
} as const satisfies Record<string, WeirdElementClass>;
