import { WeirdClient } from "./protocol/client.ts";
import "./styles/main.css";
import { World } from "./world.ts";

const appElement = document.getElementById("app");
if (appElement == null) {
  throw new Error("#app not found");
}

const world = new World();
world.mount(appElement);

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
      world.printNodes();
    },
  });
});

socket.addEventListener("close", (event) => {
  console.info("[WebSocket] closed", event);
});

socket.addEventListener("error", (event) => {
  console.info("[WebSocket] error", event);
});
