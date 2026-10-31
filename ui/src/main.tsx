import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./tokens.css";

const container = document.getElementById("root");
if (!container) throw new Error("Wafflehouse root element is missing.");

// https://react.dev/reference/react-dom/client/createRoot
createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
