import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { initOtel } from "./otel";
import "./styles.css";

initOtel();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
