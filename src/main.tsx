import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "@fontsource/barlow/latin-400.css";
import "@fontsource/barlow/latin-500.css";
import "@fontsource/barlow/latin-600.css";
import "./index.css";
import Widget from "./views/Widget";
import Dashboard from "./views/Dashboard";

const label = getCurrentWindow().label;
document.documentElement.dataset.window = label;

// No browser context menu or reload shortcuts in a desktop gadget.
window.addEventListener("contextmenu", (e) => {
  if (!(e.target as HTMLElement).closest("input, textarea")) e.preventDefault();
});
window.addEventListener("keydown", (e) => {
  if (e.key === "F5" || (e.ctrlKey && e.key.toLowerCase() === "r")) e.preventDefault();
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{label === "widget" ? <Widget /> : <Dashboard />}</React.StrictMode>,
);
