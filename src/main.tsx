import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";
import { App } from "./App.tsx";

const root = document.querySelector("main");
if (!root) throw new Error('Missing <main id="root">.');

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
