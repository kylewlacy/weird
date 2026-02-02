import { parseStyx } from "./styx.ts";
import { ServerMessage } from "./message.ts";
import "./styles/main.css";

document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
  <p>Hello world</p>
`;

const socket = new WebSocket("http://localhost:2552/ws");

socket.addEventListener("open", (_event) => {
  console.log("[WebSocket] opened");
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

  console.log("[WebSocket] parsed", message);
});

socket.addEventListener("error", (event) => {
  console.warn("WebSocket error:", event);
});
