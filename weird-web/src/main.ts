import z from "zod";
import { Debugger } from "./debugger.ts";
import { WeirdClient } from "./protocol/client.ts";
import "./styles/main.css";
import { World } from "./world.ts";
import { h } from "./elements/utils.ts";
import clsx from "clsx";
import { buttonComponent } from "./elements/Button.ts";

const Theme = z.enum(["system", "light", "dark"]);

const root = document.getElementById("root");
if (root == null) {
  throw new Error("#root not found");
}

const savedTheme = Theme.safeParse(localStorage.getItem("theme"));
document.body.dataset["theme"] = savedTheme.data ?? "system";

let worldEl: HTMLDivElement;
let debuggerEl: HTMLDivElement;
let themeChooserEl: HTMLSelectElement;
let debuggerButton: HTMLButtonElement;
const app = h(
  "div",
  {
    className: clsx(
      "size-full overflow-hidden touch-none bg-wallpaper flex flex-col",
    ),
  },
  (worldEl = h("div", {
    className: clsx("flex-1 size-full overflow-hidden, touch-none"),
  })),
  (debuggerEl = h("div", {
    id: "weird-debugger",
    className: clsx(
      "p-2 flex-none h-1/2 bg-white border-y-2 border-black overflow-auto z-0 dark:bg-zinc-900 dark:border-zinc-300 dark:text-white",
    ),
    style: { display: "none" },
  })),
  h(
    "div",
    {
      className: clsx(
        "flex-none overflow-hidden flex gap-x-2 px-2 py-2 relative z-0",
      ),
    },
    h("label", { htmlFor: "weird-theme", className: clsx("sr-only") }, "Theme"),
    (themeChooserEl = h(
      "select",
      {
        className: clsx(
          "px-2 bg-white border-2 border-black shadow-sm hover:shadow-sm/50 hover:bg-zinc-200 focus-visible:shadow-sm/50 focus-visible:bg-zinc-200 active:bg-zinc-300 focus-visible:outline-2 focus-visible:outline-blue-400 dark:text-white dark:bg-zinc-800 dark:border-zinc-300 dark:hover:bg-zinc-700 dark:focus-visible:bg-zinc-700 dark:active:bg-zinc-600 dark:shadow-md",
        ),
        value: savedTheme.data ?? "system",
      },
      h("option", { value: "system" }, "System"),
      h("option", { value: "light" }, "Light"),
      h("option", { value: "dark" }, "Dark"),
    )),
    (debuggerButton = buttonComponent({}, "Debugger")),
  ),
);
root.appendChild(app);

themeChooserEl.addEventListener("change", () => {
  const newValue = Theme.safeParse(themeChooserEl.value).data ?? "system";
  document.body.dataset["theme"] = newValue;
  localStorage.setItem("theme", newValue);
});

const url = new URL(window.location.href);
url.port = "2552";
url.pathname = "/ws";
const socket = new WebSocket(url);
const client = new WeirdClient(socket);

const world = new World();
world.onTriggerEvent = (id, event, params) => {
  client.triggerEvent(id, event, params);
};

world.mount(worldEl);

debuggerButton.addEventListener("click", (event) => {
  event.preventDefault();
  const isHidden = debuggerEl.style.display === "none";
  debuggerEl.style.display = isHidden ? "" : "none";
});

const dbg = new Debugger();
dbg.mount(debuggerEl);

socket.addEventListener("open", () => {
  console.info("[WebSocket] opened");

  client.subscribe({
    event: "syncWorld",
    params: {},
    on: (event) => {
      world.handleWorldDidChangeEvent(event);
      dbg.handleWorldDidChangeEvent(event);
    },
  });
});

socket.addEventListener("close", (event) => {
  console.info("[WebSocket] closed", event);
});

socket.addEventListener("error", (event) => {
  console.info("[WebSocket] error", event);
});
