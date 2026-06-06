import "./styles.css";
import App from "./App.svelte";
import { installViewportScrollLock } from "./lib/viewport-scroll-lock";

const app = new App({
  target: document.getElementById("app"),
});

installViewportScrollLock();

export default app;
