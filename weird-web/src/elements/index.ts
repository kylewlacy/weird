import type { WeirdElement, WeirdElementClass } from "./utils.ts";
import { UnknownElement } from "./UnknownElement.ts";
import { World } from "./World.ts";
import { Window } from "./Window.ts";
import { ProgressBar } from "./ProgressBar.ts";
import { Button } from "./Button.ts";
import { Input } from "./Input.ts";
import { Graphviz } from "./Graphviz.ts";
import { Select } from "./Select.ts";
import { Checkbox } from "./Checkbox.ts";
import { Col } from "./Col.ts";
import { Row } from "./Row.ts";
import { Flex } from "./Flex.ts";

export type { WeirdElement, WeirdElementClass };

export const ELEMENTS = {
  Button,
  Checkbox,
  Col,
  Flex,
  Graphviz,
  Input,
  ProgressBar,
  Row,
  Select,
  UnknownElement,
  Window,
  World,
} as const satisfies Record<string, WeirdElementClass>;
