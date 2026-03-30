import z from "zod";
import { Debugger } from "./debugger.ts";
import { WeirdClient } from "./protocol/client.ts";
import "./styles/main.css";
import { World } from "./world.ts";

const Theme = z.enum(["system", "light", "dark"]);

const appElement = document.getElementById("app");
if (appElement == null) {
  throw new Error("#app not found");
}

const debuggerElement = document.getElementById("debugger");
if (debuggerElement == null) {
  throw new Error("#debugger not found");
}

const savedTheme = Theme.safeParse(localStorage.getItem("theme"));
document.body.dataset["theme"] = savedTheme.data ?? "system";

const themeChooser = document.getElementById("weird-theme");
if (themeChooser instanceof HTMLSelectElement) {
  themeChooser.value = savedTheme.data ?? "system";
  themeChooser.addEventListener("change", () => {
    const newValue = Theme.safeParse(themeChooser.value).data ?? "system";
    document.body.dataset["theme"] = newValue;
    localStorage.setItem("theme", newValue);
  });
} else {
  console.warn("select#weird-theme not found");
}

const url = new URL(window.location.href);
url.port = "2552";
url.pathname = "/ws";
const socket = new WebSocket(url);
const client = new WeirdClient(socket);

const world = new World();
world.onTriggerEvent = (id, event, params) => {
  client.triggerEvent(id, event, params);
};

world.mount(appElement);

const dbg = new Debugger();
dbg.mount(debuggerElement);

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
