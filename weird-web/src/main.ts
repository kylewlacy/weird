import { parse } from "@bearcove/styx";
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
    console.warn("returned invalid type from WebSocket event, ignoring");
    return;
  }

  console.log("[WebSocket] unparsed", event.data);
  const message = parse(event.data);
  console.log("[WebSocket] parsed", message);
});

socket.addEventListener("error", (event) => {
  console.warn("WebSocket error:", event);
});
