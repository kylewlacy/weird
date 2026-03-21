import { Debugger } from "./debugger.ts";
import { WeirdClient } from "./protocol/client.ts";
import "./styles/main.css";
import { World } from "./world.ts";

const appElement = document.getElementById("app");
if (appElement == null) {
  throw new Error("#app not found");
}

const debuggerElement = document.getElementById("debugger");
if (debuggerElement == null) {
  throw new Error("#debugger not found");
}

const world = new World();
world.mount(appElement);

const dbg = new Debugger();
dbg.mount(debuggerElement);

const url = new URL(window.location.href);
url.port = "2552";
url.pathname = "/ws";
const socket = new WebSocket(url);
const client = new WeirdClient(socket);

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
