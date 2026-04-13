import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ToastProvider } from "./components/Toast";

// Tag the root element with the current OS before React mounts so CSS
// can branch on it from the very first paint. We only distinguish the
// three OSes Tauri actually ships to, and fall back to "other" for
// anything unrecognized.
//
// Why this exists: macOS gets a frameless window with titleBarStyle=
// Overlay + our own invisible drag strip (so the traffic-light buttons
// can overlay the webview cleanly). Windows and Linux keep their native
// chrome because Tauri's Overlay/hiddenTitle options are macOS-only —
// without OS-specific styling, the Windows build would show both the
// native title bar AND our 28px drag strip, wasting space.
function detectOs(): "macos" | "windows" | "linux" | "other" {
  const ua = navigator.userAgent;
  if (/Mac OS X|Macintosh/.test(ua)) return "macos";
  if (/Windows/.test(ua)) return "windows";
  if (/Linux/.test(ua)) return "linux";
  return "other";
}
document.documentElement.dataset.os = detectOs();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ToastProvider>
      <App />
    </ToastProvider>
  </React.StrictMode>,
);
