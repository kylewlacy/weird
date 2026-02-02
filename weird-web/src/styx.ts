import { parse, type Entry, type Value } from "@bearcove/styx";
import unreachable from "ts-unreachable";
import * as z from "zod";

export function parseStyx<T>(styx: string, type: z.ZodType<T>) {
  const document = parse(styx);
  const value = mapStyxEntries(document.entries);
  console.log({ document, value });
  return type.parse(value);
}

export type StyxValue =
  | undefined
  | string
  | StyxValue[]
  | StyxObject
  | { $tag: string; $value: StyxValue };

type StyxObject = { [key: string]: StyxValue };

function mapStyxValue(value: Value): StyxValue {
  let mappedValue: StyxValue;
  switch (value.payload?.type) {
    case undefined:
      mappedValue = undefined;
      break;
    case "scalar":
      mappedValue = value.payload.text;
      break;
    case "sequence":
      mappedValue = value.payload.items.map((item) => mapStyxValue(item));
      break;
    case "object":
      mappedValue = mapStyxEntries(value.payload.entries);
      break;
    default:
      return unreachable(value.payload);
  }

  if (value.tag != null) {
    mappedValue = {
      $tag: value.tag.name,
      $value: mappedValue,
    };
  }

  return mappedValue;
}

function mapStyxEntries(entries: Entry[]): StyxObject {
  return entries.reduce<StyxObject>((object, { key, value }) => {
    const objectKey = mapStyxValue(key);
    const objectValue = mapStyxValue(value);

    if (typeof objectKey !== "string") {
      throw new Error("invalid object key");
    }

    return { ...object, [objectKey]: objectValue };
  }, {});
}
