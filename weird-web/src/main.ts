import "./styles/main.css";

document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
  <p>Hello world</p>
`;

const socket = new WebSocket("http://localhost:2552/ws");

socket.addEventListener("open", (_event) => {
  console.log("[WebSocket] opened");
  socket.send("Hello world!");
});

socket.addEventListener("message", (event) => {
  console.log("[WebSocket] message", event.data);
});

socket.addEventListener("error", (event) => {
  console.warn("WebSocket error:", event);
});
