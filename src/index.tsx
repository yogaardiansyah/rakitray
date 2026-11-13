import { render } from "solid-js/web";
import { App } from "./App";
import "./styles.css";

if (import.meta.env.DEV) {
  void import("tauri-plugin-mcp").then((m) => m.setupPluginListeners());
  void import("./lib/mcpBridge").then((m) => m.initMcpBridge());
}

const root = document.getElementById("root");

if (root) {
  render(() => <App />, root);
}
