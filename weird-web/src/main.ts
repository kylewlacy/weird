import { parseStyx } from "./styx.ts";
import { ServerMessage } from "./message.ts";
import "./styles/main.css";
import { World } from "./world.ts";
import unreachable from "ts-unreachable";

const appElement = document.getElementById("app");
if (appElement == null) {
  throw new Error("#app not found");
}

const world = new World();
world.mount(appElement);

const socket = new WebSocket("http://localhost:2552/ws");

socket.addEventListener("open", (_event) => {
  console.info("[WebSocket] opened");
  socket.send(`syncWorld {requestId "syncWorld"}`);
});

socket.addEventListener("message", (event) => {
  if (typeof event.data !== "string") {
    console.warn("returned invalid type from WebSocket event, ignoring", {
      message: event.data,
    });
    return;
  }

  let message: ServerMessage;
  try {
    message = parseStyx(event.data, ServerMessage);
  } catch (error) {
    console.warn("failed to parse WebSocket message", {
      message: event.data,
      error,
    });
    return;
  }

  if ("syncWorld" in message) {
    world.applyChanges(message.syncWorld.changes);
    world.printNodes();
  } else {
    return unreachable(message);
  }

  console.info("[WebSocket] parsed", message);
});

socket.addEventListener("error", (event) => {
  console.warn("WebSocket error:", event);
});
